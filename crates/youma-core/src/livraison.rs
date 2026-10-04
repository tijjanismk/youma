use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use serde_json::json;

use crate::db::{trouver, Acteur, Db, Op};
use crate::erreur::{Erreur, Resultat};
use crate::permissions as perm;
use crate::{caisse, commandes};

pub const STATUTS: &[&str] = &[
    "nouvelle", "confirmee", "en_preparation", "prete", "assignee", "en_route", "livree", "echec", "annulee",
];

/// RG-LIV-02 : chaque livreur a un compte « à remettre », créé à la demande.
pub(crate) fn compte_du_livreur(op: &Op, employe_id: &str) -> Resultat<String> {
    if let Some(id) = op
        .query_row(
            "SELECT id FROM comptes_tresorerie WHERE type = 'livreur' AND employe_id = ?1",
            params![employe_id],
            |r| r.get::<_, String>(0),
        )
        .optional()?
    {
        return Ok(id);
    }
    let nom: String = trouver(op.query_row("SELECT nom FROM employes WHERE id = ?1", params![employe_id], |r| r.get(0)), "Livreur")?;
    let id = op.nouvel_id();
    op.execute(
        "INSERT INTO comptes_tresorerie(id, nom, type, employe_id, ordre, modifie_le) VALUES (?1, ?2, 'livreur', ?3, 90, ?4)",
        params![id, format!("À remettre — {nom}"), employe_id, op.maintenant],
    )?;
    Ok(id)
}

pub fn assigner(db: &mut Db, acteur: &Acteur, commande_id: &str, livreur_id: &str) -> Resultat<()> {
    db.executer(acteur, |op| {
        op.exiger(perm::LIVRAISON_GERER)?;
        let c = commandes::etat(op, commande_id)?;
        if c.type_ != "livraison" {
            return Err(Erreur::validation("Ce n'est pas une livraison"));
        }
        compte_du_livreur(op, livreur_id)?;
        op.execute(
            "UPDATE commandes SET livreur_id = ?1, livraison_statut = 'assignee', modifie_le = ?2 WHERE id = ?3",
            params![livreur_id, op.maintenant, commande_id],
        )?;
        // RG-LIV-05 : la course apparaît dans l'application du livreur ; il lui faut son code (et le suivi publié).
        op.execute(
            "UPDATE commandes SET code_suivi = COALESCE(code_suivi, ?1), code_livreur = COALESCE(code_livreur, ?2) WHERE id = ?3",
            params![crate::entrantes::code_aleatoire(8), crate::entrantes::code_aleatoire(12), commande_id],
        )?;
        op.audit("livraison.assigner", "commande", Some(commande_id), None, Some(json!({ "livreur": livreur_id })), None, None)?;
        op.evenement("livraison", Some(commande_id));
        Ok(())
    })
}

pub fn changer_statut(db: &mut Db, acteur: &Acteur, commande_id: &str, statut: &str, motif: &str) -> Resultat<()> {
    db.executer(acteur, |op| {
        op.exiger(perm::LIVRAISON_GERER)?;
        if !STATUTS.contains(&statut) {
            return Err(Erreur::validation("Statut de livraison inconnu"));
        }
        if (statut == "echec" || statut == "annulee") && motif.trim().is_empty() {
            return Err(Erreur::validation("Motif obligatoire"));
        }
        let n = op.execute(
            "UPDATE commandes SET livraison_statut = ?1, modifie_le = ?2 WHERE id = ?3 AND type = 'livraison'",
            params![statut, op.maintenant, commande_id],
        )?;
        if n == 0 {
            return Err(Erreur::NonTrouve("Livraison".into()));
        }
        op.audit("livraison.statut", "commande", Some(commande_id), None, Some(json!({ "statut": statut })), Some(motif.trim()).filter(|m| !m.is_empty()), None)?;
        op.evenement("livraison", Some(commande_id));
        Ok(())
    })
}

#[derive(Debug, Serialize)]
pub struct SoldeLivreur {
    pub employe_id: String,
    pub nom: String,
    pub compte_id: String,
    pub a_remettre: i64,
    pub livraisons_en_cours: i64,
}

pub fn soldes_livreurs(conn: &Connection) -> Resultat<Vec<SoldeLivreur>> {
    let mut s = conn.prepare(
        "SELECT e.id, e.nom, c.id,
                (SELECT COALESCE(SUM(m.montant), 0) FROM mouvements_tresorerie m WHERE m.compte_id = c.id),
                (SELECT COUNT(*) FROM commandes k WHERE k.livreur_id = e.id AND k.livraison_statut IN ('assignee','en_route'))
         FROM comptes_tresorerie c JOIN employes e ON e.id = c.employe_id WHERE c.type = 'livreur' ORDER BY e.nom",
    )?;
    let v = s
        .query_map([], |r| {
            Ok(SoldeLivreur { employe_id: r.get(0)?, nom: r.get(1)?, compte_id: r.get(2)?, a_remettre: r.get(3)?, livraisons_en_cours: r.get(4)? })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(v)
}

#[derive(Debug, Serialize)]
pub struct ResultatRemise {
    pub attendu: i64,
    pub remis: i64,
    pub ecart: i64,
}

/// RG-LIV-03 : remise livreur vers la caisse, écart enregistré.
pub fn remise_livreur(db: &mut Db, acteur: &Acteur, livreur_id: &str, remis: i64, motif: &str) -> Resultat<ResultatRemise> {
    db.executer(acteur, |op| {
        op.exiger(perm::CAISSE_ENCAISSER)?;
        let session = op
            .utilisateur()
            .map(|u| caisse::session_utilisateur(op, u))
            .transpose()?
            .flatten()
            .ok_or_else(|| Erreur::regle("RG-CAI-01", "Ouvrez votre session de caisse"))?;
        let compte = compte_du_livreur(op, livreur_id)?;
        let attendu = caisse::solde(op, &compte)?;
        if remis < 0 {
            return Err(Erreur::validation("Montant remis invalide"));
        }
        let ecart = remis - attendu;
        if ecart != 0 && motif.trim().is_empty() {
            return Err(Erreur::regle("RG-LIV-03", format!("Écart de {ecart} FCFA : motif obligatoire")));
        }
        // Le livreur remet `remis` : il sort de son compte et entre dans la caisse.
        if remis > 0 {
            caisse::mouvement(op, &compte, Some(&session.id), "remise_livreur", -remis, Some(("employe", livreur_id)), "Remise livreur", None)?;
            caisse::mouvement(op, &session.compte_id, Some(&session.id), "remise_livreur", remis, Some(("employe", livreur_id)), "Remise livreur", None)?;
        }
        // Le reste (manquant > 0 ou excédent < 0) solde le compte livreur et reste tracé.
        if ecart != 0 {
            caisse::mouvement(op, &compte, Some(&session.id), "ecart_livreur", ecart, Some(("employe", livreur_id)), motif.trim(), None)?;
        }
        op.audit("livraison.remise", "employe", Some(livreur_id), None, Some(json!({ "attendu": attendu, "remis": remis, "ecart": ecart })), Some(motif.trim()).filter(|m| !m.is_empty()), None)?;
        op.evenement("caisse", None);
        Ok(ResultatRemise { attendu, remis, ecart })
    })
}

/// Dernière position connue de chaque livreur en course (écran Livraisons du restaurant).
#[derive(Debug, Serialize)]
pub struct PositionCourse {
    pub commande_id: String,
    pub numero: i64,
    pub livreur_nom: Option<String>,
    /// Microdegrés, comme `positions_livreur`.
    pub lat: i64,
    pub lon: i64,
    pub horodatage: i64,
    /// Position partagée par le client à la commande, si elle existe.
    pub destination: Option<(i64, i64)>,
}

pub fn positions_en_cours(conn: &Connection, journee_id: &str) -> Resultat<Vec<PositionCourse>> {
    let mut s = conn.prepare(
        "SELECT c.id, c.numero, e.nom, p.lat, p.lon, p.horodatage, c.livraison_lat, c.livraison_lon
         FROM commandes c
         JOIN positions_livreur p ON p.id = (SELECT id FROM positions_livreur WHERE commande_id = c.id ORDER BY horodatage DESC LIMIT 1)
         LEFT JOIN employes e ON e.id = c.livreur_id
         WHERE c.journee_id = ?1 AND c.type = 'livraison' AND c.livraison_statut IN ('assignee', 'en_route')
         ORDER BY c.numero",
    )?;
    let lignes = s
        .query_map(params![journee_id], |r| {
            let (dlat, dlon): (Option<i64>, Option<i64>) = (r.get(6)?, r.get(7)?);
            Ok(PositionCourse {
                commande_id: r.get(0)?,
                numero: r.get(1)?,
                livreur_nom: r.get(2)?,
                lat: r.get(3)?,
                lon: r.get(4)?,
                horodatage: r.get(5)?,
                destination: dlat.zip(dlon),
            })
        })?
        .collect::<Result<_, _>>()?;
    Ok(lignes)
}

/// Course d'un livreur, publiée au relais pour l'application Youma Livreur (RG-LIV-05, fiche 0047).
#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct CourseLivreur {
    pub code_livreur: String,
    pub numero: i64,
    pub statut: String,
    pub client_nom: String,
    pub telephone: Option<String>,
    pub quartier: Option<String>,
    pub repere: Option<String>,
    pub lat: Option<i64>,
    pub lon: Option<i64>,
    /// Reste à encaisser auprès du client.
    pub reste: i64,
}

/// Livreur ayant l'accès à l'application : le relais vérifie son PIN (empreinte Argon2) et lui montre ses courses.
#[derive(Debug, Serialize)]
pub struct LivreurRelais {
    pub employe_id: String,
    pub nom: String,
    pub telephone: String,
    pub pin_hash: String,
    pub courses: Vec<CourseLivreur>,
}

pub fn livreurs_relais(conn: &Connection) -> Resultat<Vec<LivreurRelais>> {
    let livreurs: Vec<(String, String, String, String)> = conn
        .prepare("SELECT id, nom, telephone, pin_livreur_hash FROM employes WHERE pin_livreur_hash IS NOT NULL AND statut = 'actif' ORDER BY nom")?
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?
        .collect::<Result<_, _>>()?;
    let mut s = conn.prepare_cached(
        "SELECT c.id, c.code_livreur, c.numero, c.livraison_statut, COALESCE(cl.nom, c.client_nom_saisi, ''), c.livraison_telephone,
                c.livraison_quartier, c.livraison_repere, c.livraison_lat, c.livraison_lon
         FROM commandes c LEFT JOIN clients cl ON cl.id = c.client_id
         WHERE c.livreur_id = ?1 AND c.code_livreur IS NOT NULL AND c.livraison_statut IN ('assignee', 'en_route') ORDER BY c.cree_le",
    )?;
    let mut v = Vec::with_capacity(livreurs.len());
    for (id, nom, telephone, pin_hash) in livreurs {
        let lignes: Vec<(String, CourseLivreur)> = s
            .query_map(params![id], |r| {
                Ok((
                    r.get(0)?,
                    CourseLivreur {
                        code_livreur: r.get(1)?,
                        numero: r.get(2)?,
                        statut: r.get(3)?,
                        client_nom: r.get(4)?,
                        telephone: r.get(5)?,
                        quartier: r.get(6)?,
                        repere: r.get(7)?,
                        lat: r.get(8)?,
                        lon: r.get(9)?,
                        reste: 0,
                    },
                ))
            })?
            .collect::<Result<_, _>>()?;
        let mut courses = Vec::with_capacity(lignes.len());
        for (commande_id, mut c) in lignes {
            c.reste = commandes::totaux(conn, &commande_id)?.reste;
            courses.push(c);
        }
        v.push(LivreurRelais { employe_id: id, nom, telephone: crate::zones_risque::normaliser_telephone(&telephone), pin_hash, courses });
    }
    Ok(v)
}
