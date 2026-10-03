//! Avis des clients (RG-AVI-01 à 03, fiche 0043) : une note de 1 à 5 et un commentaire par commande terminée,
//! donnés depuis la page de suivi (QR de table ou commande en ligne, sur le poste ou par le relais). Les avis faibles
//! (2 ou moins) attendent une suite du gérant (appel, bon d'avoir…).

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::db::{Acteur, Db};
use crate::erreur::{Erreur, Resultat};
use crate::permissions as perm;
use crate::{entrantes, rapports};

/// Note à partir de laquelle un avis demande une suite (RG-AVI-02).
pub const NOTE_FAIBLE: i64 = 2;
/// Délai pour donner son avis après la commande (RG-AVI-01).
pub const DELAI_MS: i64 = 7 * 86_400_000;
const COMMENTAIRE_MAX: usize = 500;

#[derive(Debug, Deserialize, Clone)]
pub struct NouvelAvis {
    pub note: i64,
    #[serde(default)]
    pub commentaire: String,
}

/// Étapes du suivi après lesquelles le client peut noter : servie ou livrée, ou livraison en échec.
pub fn possible_a_l_etape(etape: &str) -> bool {
    matches!(etape, "livree" | "echec")
}

/// Note déjà donnée pour cette commande.
pub fn note_de(conn: &Connection, commande_id: &str) -> Resultat<Option<i64>> {
    Ok(conn.query_row("SELECT note FROM avis WHERE commande_id = ?1", params![commande_id], |r| r.get(0)).optional()?)
}

/// RG-AVI-01 : avis donné avec le code de suivi (sans connexion), une seule fois, commande terminée, dans les 7 jours.
pub fn donner(db: &mut Db, code_suivi: &str, a: &NouvelAvis) -> Resultat<()> {
    if !(1..=5).contains(&a.note) {
        return Err(Erreur::validation("Note de 1 à 5"));
    }
    let commentaire = a.commentaire.trim();
    if commentaire.chars().count() > COMMENTAIRE_MAX {
        return Err(Erreur::validation(format!("Commentaire trop long ({COMMENTAIRE_MAX} caractères au plus)")));
    }
    let suivi = entrantes::suivi(db.conn(), code_suivi)?;
    db.executer(&Acteur::systeme(), |op| {
        let id = entrantes::id_par_code(op, code_suivi.trim())?;
        if !possible_a_l_etape(&suivi.etape) {
            return Err(Erreur::regle("RG-AVI-01", "Vous pourrez donner votre avis une fois la commande servie ou livrée"));
        }
        if note_de(op, &id)?.is_some() {
            return Err(Erreur::regle("RG-AVI-01", "Votre avis a déjà été reçu : merci"));
        }
        let cree: i64 = op.query_row("SELECT cree_le FROM commandes WHERE id = ?1", params![id], |r| r.get(0))?;
        if op.maintenant - cree > DELAI_MS {
            return Err(Erreur::regle("RG-AVI-01", "Le délai pour donner son avis (7 jours) est passé"));
        }
        let avis = op.nouvel_id();
        op.execute(
            "INSERT INTO avis(id, commande_id, note, commentaire, recu_le) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![avis, id, a.note, commentaire, op.maintenant],
        )?;
        op.audit("avis.recu", "commande", Some(&id), None, Some(json!({ "note": a.note })), None, None)?;
        op.evenement("avis", Some(&avis));
        Ok(())
    })
}

#[derive(Debug, Serialize)]
pub struct Avis {
    pub id: String,
    pub commande_id: String,
    pub commande_numero: i64,
    pub canal: String,
    pub type_: String,
    pub note: i64,
    pub commentaire: String,
    pub recu_le: i64,
    pub client: String,
    pub telephone: Option<String>,
    pub client_id: Option<String>,
    pub traite_le: Option<i64>,
    pub suite: Option<String>,
}

/// Avis reçus, du plus récent au plus ancien ; `a_traiter` : seulement les avis faibles sans suite (RG-AVI-02).
pub fn lister(conn: &Connection, a_traiter: bool) -> Resultat<Vec<Avis>> {
    let filtre = if a_traiter { format!("WHERE a.note <= {NOTE_FAIBLE} AND a.traite_le IS NULL") } else { String::new() };
    let mut s = conn.prepare(&format!(
        "SELECT a.id, a.commande_id, c.numero, c.canal, c.type, a.note, a.commentaire, a.recu_le,
                COALESCE(cl.nom, c.client_nom_saisi, ''), COALESCE(c.client_telephone, cl.telephone), c.client_id, a.traite_le, a.suite
         FROM avis a JOIN commandes c ON c.id = a.commande_id LEFT JOIN clients cl ON cl.id = c.client_id
         {filtre} ORDER BY a.recu_le DESC LIMIT 200"
    ))?;
    let v = s
        .query_map([], |r| {
            Ok(Avis {
                id: r.get(0)?,
                commande_id: r.get(1)?,
                commande_numero: r.get(2)?,
                canal: r.get(3)?,
                type_: r.get(4)?,
                note: r.get(5)?,
                commentaire: r.get(6)?,
                recu_le: r.get(7)?,
                client: r.get(8)?,
                telephone: r.get(9)?,
                client_id: r.get(10)?,
                traite_le: r.get(11)?,
                suite: r.get(12)?,
            })
        })?
        .collect::<Result<_, _>>()?;
    Ok(v)
}

/// Nombre d'avis faibles en attente d'une suite (tableau de bord).
pub fn nombre_a_traiter(conn: &Connection) -> Resultat<i64> {
    Ok(conn.query_row(&format!("SELECT COUNT(*) FROM avis WHERE note <= {NOTE_FAIBLE} AND traite_le IS NULL"), [], |r| r.get(0))?)
}

/// RG-AVI-02 : le gérant note la suite donnée à un avis faible (appel, bon d'avoir, explication), une seule fois.
pub fn traiter(db: &mut Db, acteur: &Acteur, avis_id: &str, suite: &str) -> Resultat<()> {
    db.executer(acteur, |op| {
        op.exiger(perm::COMMANDE_OFFRIR)?;
        let suite = suite.trim();
        if suite.is_empty() {
            return Err(Erreur::validation("Indiquez la suite donnée (appel, bon d'avoir, explication…)"));
        }
        let n = op.execute(
            "UPDATE avis SET traite_le = ?1, traite_par = ?2, suite = ?3 WHERE id = ?4 AND traite_le IS NULL",
            params![op.maintenant, op.utilisateur(), suite, avis_id],
        )?;
        if n == 0 {
            return Err(Erreur::regle("RG-AVI-02", "Avis introuvable ou déjà traité"));
        }
        op.audit("avis.traite", "avis", Some(avis_id), None, Some(json!({ "suite": suite })), None, None)?;
        op.evenement("avis", Some(avis_id));
        Ok(())
    })
}

/// RG-AVI-03 : rapport des avis sur des journées d'exploitation (date de la commande).
pub fn rapport(conn: &Connection, debut: &str, fin: &str) -> Resultat<rapports::Rapport> {
    let periode = "FROM avis a JOIN commandes c ON c.id = a.commande_id JOIN journees j ON j.id = c.journee_id
                   WHERE j.date_exploitation BETWEEN ?1 AND ?2";
    let (nb, somme, faibles, a_traiter): (i64, i64, i64, i64) = conn.query_row(
        &format!(
            "SELECT COUNT(*), COALESCE(SUM(a.note), 0), COALESCE(SUM(a.note <= {NOTE_FAIBLE}), 0),
                    COALESCE(SUM(a.note <= {NOTE_FAIBLE} AND a.traite_le IS NULL), 0) {periode}"
        ),
        params![debut, fin],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
    )?;
    // Note moyenne en dixièmes (4,3 / 5 → 43), arrondie.
    let moyenne = if nb > 0 { (somme * 10 + nb / 2) / nb } else { 0 };
    let groupes = |expr: &str| -> Resultat<Vec<Vec<serde_json::Value>>> {
        let mut s = conn.prepare(&format!(
            "SELECT {expr}, COUNT(*), (SUM(a.note) * 10 + COUNT(*) / 2) / COUNT(*), SUM(a.note <= {NOTE_FAIBLE}) {periode}
             GROUP BY 1 ORDER BY 2 DESC"
        ))?;
        let v = s
            .query_map(params![debut, fin], |r| {
                let note: i64 = r.get(2)?;
                Ok(vec![json!(r.get::<_, String>(0)?), json!(r.get::<_, i64>(1)?), json!(note_texte(note)), json!(r.get::<_, i64>(3)?)])
            })?
            .collect::<Result<_, _>>()?;
        Ok(v)
    };
    let colonnes = |premiere: &str| vec![premiere.to_string(), "Nombre".into(), "Note moyenne".into(), "Nombre de mécontents".into()];
    let mut repartition = Vec::new();
    for note in (1..=5).rev() {
        let n: i64 = conn.query_row(&format!("SELECT COUNT(*) {periode} AND a.note = ?3"), params![debut, fin, note], |r| r.get(0))?;
        repartition.push(vec![json!(format!("{note} / 5")), json!(n)]);
    }
    let mut s = conn.prepare(&format!(
        "SELECT j.date_exploitation, c.numero, a.note, a.commentaire, COALESCE(a.suite, CASE WHEN a.note <= {NOTE_FAIBLE} THEN 'à traiter' ELSE '' END)
         {periode} AND (a.commentaire <> '' OR a.note <= {NOTE_FAIBLE}) ORDER BY a.recu_le DESC LIMIT 100"
    ))?;
    let commentaires = s
        .query_map(params![debut, fin], |r| {
            Ok(vec![
                json!(r.get::<_, String>(0)?),
                json!(r.get::<_, i64>(1)?),
                json!(format!("{} / 5", r.get::<_, i64>(2)?)),
                json!(r.get::<_, String>(3)?),
                json!(r.get::<_, String>(4)?),
            ])
        })?
        .collect::<Result<_, _>>()?;
    let tableau = |titre: &str, colonnes: Vec<String>, lignes| rapports::Tableau { titre: titre.into(), colonnes, lignes, formule: None };
    Ok(rapports::Rapport {
        titre: "Avis des clients".into(),
        debut: debut.into(),
        fin: fin.into(),
        indicateurs: vec![
            rapports::Indicateur { cle: "nb_avis".into(), libelle: "Avis reçus".into(), valeur: nb, formule: String::new() },
            rapports::Indicateur { cle: "moyenne_note".into(), libelle: "Note moyenne".into(), valeur: moyenne, formule: String::new() },
            rapports::Indicateur { cle: "nb_mecontents".into(), libelle: "Clients mécontents (2 / 5 ou moins)".into(), valeur: faibles, formule: String::new() },
            rapports::Indicateur { cle: "nb_a_traiter".into(), libelle: "Avis faibles sans suite".into(), valeur: a_traiter, formule: String::new() },
        ],
        tableaux: vec![
            tableau("Répartition des notes", vec!["Note".into(), "Nombre".into()], repartition),
            tableau("Par canal", colonnes("Canal"), groupes(rapports::LIBELLE_CANAL)?),
            tableau("Par type de commande", colonnes("Type"), groupes(rapports::LIBELLE_TYPE)?),
            tableau(
                "Par livreur",
                colonnes("Livreur"),
                groupes("COALESCE((SELECT e.nom FROM employes e WHERE e.id = c.livreur_id), 'sans livreur')")?,
            ),
            tableau(
                "Par serveur",
                colonnes("Serveur"),
                groupes("COALESCE((SELECT u.nom FROM utilisateurs u WHERE u.id = c.serveur_id), 'sans serveur')")?,
            ),
            tableau(
                "Commentaires et avis faibles",
                vec!["Date".into(), "N°".into(), "Note".into(), "Commentaire".into(), "Suite".into()],
                commentaires,
            ),
        ],
    })
}

fn note_texte(dixiemes: i64) -> String {
    format!("{},{} / 5", dixiemes / 10, dixiemes % 10)
}
