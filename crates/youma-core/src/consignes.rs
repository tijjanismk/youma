//! Consignes (fiche 0015, RG-CON-01 à 06) : bouteilles et casiers consignés.
//! Détenus = pleins + vides présents au restaurant (Σ mouvements) ; vides = détenus − pleins en stock ;
//! consigne versée à un fournisseur = Σ montants (ce qu'il doit rendre contre les emballages).

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::catalogue::non_vide;
use crate::db::{trouver, Acteur, Db, Op};
use crate::erreur::{Erreur, Resultat};
use crate::permissions as perm;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Emballage {
    #[serde(default)]
    pub id: String,
    pub nom: String,
    /// Consigne d'un emballage, FCFA entiers.
    pub valeur: i64,
    #[serde(default = "vrai")]
    pub actif: bool,
    /// Articles dont une unité pleine contient cet emballage (bière 65 cl → bouteille 65 cl).
    #[serde(default)]
    pub articles: Vec<String>,
}

fn vrai() -> bool {
    true
}

#[derive(Debug, Serialize)]
pub struct EtatEmballage {
    pub emballage: Emballage,
    /// Pleins + vides présents au restaurant.
    pub detenus: i64,
    /// Unités pleines en stock (articles liés) ; `None` sans article lié (casiers).
    pub pleins: Option<i64>,
    /// RG-CON-03 : vides = détenus − pleins.
    pub vides: Option<i64>,
    /// Consigne versée non encore rendue, tous fournisseurs.
    pub consigne_versee: i64,
    /// Détenus × valeur.
    pub valeur_detenus: i64,
}

pub fn lister(conn: &Connection) -> Resultat<Vec<Emballage>> {
    let base: Vec<(String, String, i64, bool)> = conn
        .prepare("SELECT id, nom, valeur, actif FROM emballages ORDER BY nom")?
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?
        .collect::<Result<_, _>>()?;
    let mut v = Vec::new();
    for (id, nom, valeur, actif) in base {
        let articles: Vec<String> = conn
            .prepare_cached("SELECT id FROM articles_stock WHERE emballage_id = ?1 ORDER BY nom")?
            .query_map(params![id], |r| r.get(0))?
            .collect::<Result<_, _>>()?;
        v.push(Emballage { id, nom, valeur, actif, articles });
    }
    Ok(v)
}

/// RG-CON-01 : valeur entière ≥ 0 ; lien avec les articles dont une unité pleine contient l'emballage.
pub fn enregistrer(db: &mut Db, acteur: &Acteur, e: &Emballage) -> Resultat<String> {
    db.executer(acteur, |op| {
        op.exiger(perm::STOCK_MOUVEMENT)?;
        non_vide(&e.nom, "nom de l'emballage")?;
        if e.valeur < 0 {
            return Err(Erreur::regle("RG-CON-01", "La consigne ne peut pas être négative"));
        }
        let id = if e.id.is_empty() { op.nouvel_id() } else { e.id.clone() };
        op.execute(
            "INSERT INTO emballages(id, nom, valeur, actif, modifie_le) VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(id) DO UPDATE SET nom = excluded.nom, valeur = excluded.valeur, actif = excluded.actif,
               modifie_le = excluded.modifie_le",
            params![id, e.nom.trim(), e.valeur, e.actif, op.maintenant],
        )?;
        op.execute("UPDATE articles_stock SET emballage_id = NULL WHERE emballage_id = ?1", params![id])?;
        for a in &e.articles {
            let n = op.execute("UPDATE articles_stock SET emballage_id = ?1, modifie_le = ?2 WHERE id = ?3", params![id, op.maintenant, a])?;
            if n == 0 {
                return Err(Erreur::NonTrouve("Article de stock".into()));
            }
        }
        op.audit("emballage.enregistrer", "emballage", Some(&id), None, Some(json!(e)), None, None)?;
        op.outbox("emballage", &id, "enregistrer")?;
        op.evenement("stock", None);
        Ok(id)
    })
}

fn detenus(conn: &Connection, emballage_id: &str) -> Resultat<i64> {
    Ok(conn.query_row("SELECT COALESCE(SUM(quantite), 0) FROM mouvements_emballages WHERE emballage_id = ?1", params![emballage_id], |r| r.get(0))?)
}

fn pleins(conn: &Connection, emballage_id: &str) -> Resultat<Option<i64>> {
    let (liens, q): (i64, i64) = conn.query_row(
        "SELECT COUNT(DISTINCT a.id), COALESCE(SUM(m.quantite), 0) FROM articles_stock a
         LEFT JOIN mouvements_stock m ON m.article_id = a.id WHERE a.emballage_id = ?1",
        params![emballage_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    Ok((liens > 0).then_some(q.max(0)))
}

pub fn etats(conn: &Connection) -> Resultat<Vec<EtatEmballage>> {
    let mut v = Vec::new();
    for e in lister(conn)?.into_iter().filter(|e| e.actif) {
        let d = detenus(conn, &e.id)?;
        let p = pleins(conn, &e.id)?;
        let versee: i64 = conn.query_row(
            "SELECT COALESCE(SUM(montant), 0) FROM mouvements_emballages WHERE emballage_id = ?1",
            params![e.id],
            |r| r.get(0),
        )?;
        v.push(EtatEmballage { pleins: p, vides: p.map(|p| d - p), consigne_versee: versee, valeur_detenus: d * e.valeur, detenus: d, emballage: e });
    }
    Ok(v)
}

/// Consigne versée non rendue, par fournisseur : (fournisseur_id, nom, montant).
pub fn consignes_par_fournisseur(conn: &Connection) -> Resultat<Vec<(String, String, i64)>> {
    let mut s = conn.prepare(
        "SELECT f.id, f.nom, SUM(m.montant) FROM mouvements_emballages m JOIN fournisseurs f ON f.id = m.fournisseur_id
         GROUP BY f.id HAVING SUM(m.montant) <> 0 ORDER BY f.nom",
    )?;
    let v = s.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?.collect::<Result<Vec<_>, _>>()?;
    Ok(v)
}

fn valeur(op: &Op, emballage_id: &str) -> Resultat<i64> {
    trouver(op.query_row("SELECT valeur FROM emballages WHERE id = ?1 AND actif = 1", params![emballage_id], |r| r.get(0)), "Emballage")
}

#[allow(clippy::too_many_arguments)]
fn inserer(
    op: &Op,
    emballage_id: &str,
    fournisseur_id: Option<&str>,
    type_: &str,
    quantite: i64,
    montant: i64,
    motif: &str,
    achat_id: Option<&str>,
    tresorerie: Option<&str>,
) -> Resultat<String> {
    let id = op.nouvel_id();
    let journee = crate::journee::ouverte(op)?.map(|j| j.id);
    op.execute(
        "INSERT INTO mouvements_emballages(id, emballage_id, fournisseur_id, type, quantite, montant, motif, achat_id,
            mouvement_tresorerie_id, journee_id, horodatage, utilisateur_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![id, emballage_id, fournisseur_id, type_, quantite, montant, motif, achat_id, tresorerie, journee, op.maintenant, op.utilisateur()],
    )?;
    op.outbox("mouvement_emballage", &id, "creer")?;
    Ok(id)
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConsigneAchat {
    pub emballage_id: String,
    /// Emballages arrivés avec la livraison (pleins).
    #[serde(default)]
    pub recus: i64,
    /// Vides rendus au livreur du fournisseur.
    #[serde(default)]
    pub rendus: i64,
}

/// RG-CON-02 : à la réception d'un achat, reçus (+) et rendus (−) ; renvoie la consigne nette à ajouter au total.
pub(crate) fn a_la_reception(op: &Op, achat_id: &str, fournisseur_id: Option<&str>, c: &[ConsigneAchat]) -> Resultat<i64> {
    let mut net = 0;
    for x in c {
        if x.recus < 0 || x.rendus < 0 {
            return Err(Erreur::validation("Nombre d'emballages invalide"));
        }
        let v = valeur(op, &x.emballage_id)?;
        if x.rendus > 0 && detenus(op, &x.emballage_id)? < x.rendus {
            return Err(Erreur::regle("RG-CON-02", "Plus d'emballages rendus que détenus : faites d'abord l'inventaire des vides"));
        }
        if x.recus > 0 {
            inserer(op, &x.emballage_id, fournisseur_id, "reception", x.recus, x.recus * v, "", Some(achat_id), None)?;
        }
        if x.rendus > 0 {
            inserer(op, &x.emballage_id, fournisseur_id, "retour", -x.rendus, -x.rendus * v, "", Some(achat_id), None)?;
        }
        net += (x.recus - x.rendus) * v;
    }
    Ok(net)
}

pub const TYPES_MANUELS: &[&str] = &["casse", "perte", "sortie_client", "retour_client", "regularisation"];

#[derive(Debug, Deserialize)]
pub struct MouvementEmballage {
    pub emballage_id: String,
    #[serde(rename = "type")]
    pub type_: String,
    /// Nombre positif ; le sens dépend du type (régularisation : signée).
    pub quantite: i64,
    pub motif: String,
}

/// RG-CON-04 : casse, perte, bouteille partie avec un client (ou rapportée), régularisation : motif obligatoire.
pub fn mouvement(db: &mut Db, acteur: &Acteur, m: &MouvementEmballage) -> Resultat<String> {
    db.executer(acteur, |op| {
        let autorise = op.exiger(perm::STOCK_MOUVEMENT)?;
        if !TYPES_MANUELS.contains(&m.type_.as_str()) {
            return Err(Erreur::regle("RG-CON-04", "Type de mouvement d'emballage non autorisé"));
        }
        if m.motif.trim().is_empty() {
            return Err(Erreur::regle("RG-CON-04", "Motif obligatoire"));
        }
        if m.quantite == 0 || (m.type_ != "regularisation" && m.quantite < 0) {
            return Err(Erreur::validation("Quantité invalide"));
        }
        valeur(op, &m.emballage_id)?;
        let q = match m.type_.as_str() {
            "retour_client" | "regularisation" => m.quantite,
            _ => -m.quantite,
        };
        let id = inserer(op, &m.emballage_id, None, &m.type_, q, 0, m.motif.trim(), None, None)?;
        op.audit("emballage.mouvement", "emballage", Some(&m.emballage_id), None, Some(json!({ "type": m.type_, "quantite": q })), Some(m.motif.trim()), autorise.as_deref())?;
        op.evenement("stock", None);
        Ok(id)
    })
}

/// RG-CON-05 : inventaire des vides ; écart = vides comptés − vides théoriques (sans article lié : détenus comptés).
pub fn inventaire(db: &mut Db, acteur: &Acteur, emballage_id: &str, comptes: i64) -> Resultat<i64> {
    db.executer(acteur, |op| {
        let autorise = op.exiger(perm::STOCK_VALIDER_INVENTAIRE)?;
        if comptes < 0 {
            return Err(Erreur::validation("Nombre compté invalide"));
        }
        valeur(op, emballage_id)?;
        let d = detenus(op, emballage_id)?;
        let theorique = match pleins(op, emballage_id)? {
            Some(p) => d - p,
            None => d,
        };
        let ecart = comptes - theorique;
        if ecart != 0 {
            inserer(op, emballage_id, None, "inventaire", ecart, 0, &format!("Compté {comptes}, attendu {theorique}"), None, None)?;
        }
        op.audit("emballage.inventaire", "emballage", Some(emballage_id), Some(json!({ "attendu": theorique })), Some(json!({ "compte": comptes, "ecart": ecart })), None, autorise.as_deref())?;
        op.evenement("stock", None);
        Ok(ecart)
    })
}

#[derive(Debug, Deserialize)]
pub struct RetourFournisseur {
    pub fournisseur_id: String,
    pub emballage_id: String,
    pub quantite: i64,
    /// especes : le fournisseur rembourse (entrée de caisse) ; dette : déduit de ce qu'on lui doit.
    pub remboursement: String,
    #[serde(default)]
    pub compte_id: Option<String>,
    #[serde(default)]
    pub note: String,
}

/// RG-CON-06 : vides rendus au fournisseur hors livraison ; la consigne revient en caisse ou réduit la dette.
pub fn retour_fournisseur(db: &mut Db, acteur: &Acteur, r: &RetourFournisseur) -> Resultat<String> {
    db.executer(acteur, |op| {
        op.exiger(perm::ACHAT_GERER)?;
        if r.quantite <= 0 {
            return Err(Erreur::validation("Quantité invalide"));
        }
        let v = valeur(op, &r.emballage_id)?;
        let nom: String = trouver(op.query_row("SELECT nom FROM fournisseurs WHERE id = ?1", params![r.fournisseur_id], |x| x.get(0)), "Fournisseur")?;
        if detenus(op, &r.emballage_id)? < r.quantite {
            return Err(Erreur::regle("RG-CON-06", "Plus d'emballages rendus que détenus"));
        }
        let versee: i64 = op.query_row(
            "SELECT COALESCE(SUM(montant), 0) FROM mouvements_emballages WHERE emballage_id = ?1 AND fournisseur_id = ?2",
            params![r.emballage_id, r.fournisseur_id],
            |x| x.get(0),
        )?;
        let montant = r.quantite * v;
        if montant > versee {
            return Err(Erreur::regle("RG-CON-06", format!("Ce fournisseur ne détient que {versee} FCFA de consigne pour cet emballage")));
        }
        let tresorerie = match r.remboursement.as_str() {
            "especes" => {
                let session = op.utilisateur().map(|u| crate::caisse::session_utilisateur(op, u)).transpose()?.flatten();
                let compte = match (&r.compte_id, &session) {
                    (Some(c), _) => c.clone(),
                    (None, Some(s)) => s.compte_id.clone(),
                    (None, None) => return Err(Erreur::regle("RG-CAI-01", "Ouvrez une session de caisse ou choisissez un compte")),
                };
                let libelle = format!("Consigne rendue par {nom}");
                Some(crate::caisse::mouvement(op, &compte, session.as_ref().map(|s| s.id.as_str()), "remboursement_consigne", montant, Some(("fournisseur", &r.fournisseur_id)), &libelle, None)?)
            }
            "dette" => {
                op.execute(
                    "INSERT INTO mouvements_fournisseur(id, fournisseur_id, type, montant, motif, horodatage, utilisateur_id)
                     VALUES (?1, ?2, 'regularisation', ?3, ?4, ?5, ?6)",
                    params![op.nouvel_id(), r.fournisseur_id, -montant, "Consigne d'emballages rendus", op.maintenant, op.utilisateur()],
                )?;
                None
            }
            _ => return Err(Erreur::validation("Remboursement : especes ou dette")),
        };
        let id = inserer(op, &r.emballage_id, Some(&r.fournisseur_id), "retour", -r.quantite, -montant, r.note.trim(), None, tresorerie.as_deref())?;
        op.audit("emballage.retour_fournisseur", "fournisseur", Some(&r.fournisseur_id), None, Some(json!({ "quantite": r.quantite, "montant": montant, "remboursement": r.remboursement })), None, None)?;
        op.evenement("stock", None);
        Ok(id)
    })
}

#[derive(Debug, Serialize)]
pub struct MouvementLu {
    #[serde(rename = "type")]
    pub type_: String,
    pub quantite: i64,
    pub montant: i64,
    pub fournisseur: Option<String>,
    pub motif: String,
    pub horodatage: i64,
}

pub fn historique(conn: &Connection, emballage_id: &str) -> Resultat<Vec<MouvementLu>> {
    let mut s = conn.prepare(
        "SELECT m.type, m.quantite, m.montant, f.nom, m.motif, m.horodatage FROM mouvements_emballages m
         LEFT JOIN fournisseurs f ON f.id = m.fournisseur_id WHERE m.emballage_id = ?1 ORDER BY m.horodatage DESC LIMIT 200",
    )?;
    let v = s
        .query_map(params![emballage_id], |r| {
            Ok(MouvementLu { type_: r.get(0)?, quantite: r.get(1)?, montant: r.get(2)?, fournisseur: r.get(3)?, motif: r.get(4)?, horodatage: r.get(5)? })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(v)
}
