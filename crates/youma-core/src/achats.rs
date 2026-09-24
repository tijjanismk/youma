use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::catalogue::non_vide;
use crate::db::{trouver, Acteur, Db};
use crate::erreur::{Erreur, Resultat};
use crate::permissions as perm;
use crate::{caisse, stock};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fournisseur {
    #[serde(default)]
    pub id: String,
    pub nom: String,
    #[serde(default)]
    pub telephone: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default = "vrai")]
    pub actif: bool,
    #[serde(default)]
    pub dette: i64,
}

fn vrai() -> bool {
    true
}

pub fn dette(conn: &Connection, fournisseur_id: &str) -> Resultat<i64> {
    Ok(conn.query_row(
        "SELECT COALESCE(SUM(montant), 0) FROM mouvements_fournisseur WHERE fournisseur_id = ?1",
        params![fournisseur_id],
        |r| r.get(0),
    )?)
}

pub fn lister_fournisseurs(conn: &Connection) -> Resultat<Vec<Fournisseur>> {
    let mut s = conn.prepare(
        "SELECT f.id, f.nom, f.telephone, f.notes, f.actif,
                (SELECT COALESCE(SUM(m.montant), 0) FROM mouvements_fournisseur m WHERE m.fournisseur_id = f.id)
         FROM fournisseurs f ORDER BY f.nom",
    )?;
    let v = s
        .query_map([], |r| {
            Ok(Fournisseur {
                id: r.get(0)?,
                nom: r.get(1)?,
                telephone: r.get(2)?,
                notes: r.get(3)?,
                actif: r.get(4)?,
                dette: r.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(v)
}

pub fn enregistrer_fournisseur(db: &mut Db, acteur: &Acteur, f: &Fournisseur) -> Resultat<String> {
    db.executer(acteur, |op| {
        op.exiger(perm::ACHAT_GERER)?;
        non_vide(&f.nom, "nom du fournisseur")?;
        let id = if f.id.is_empty() { op.nouvel_id() } else { f.id.clone() };
        op.execute(
            "INSERT INTO fournisseurs(id, nom, telephone, notes, actif, cree_le, modifie_le) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)
             ON CONFLICT(id) DO UPDATE SET nom = excluded.nom, telephone = excluded.telephone, notes = excluded.notes,
               actif = excluded.actif, modifie_le = excluded.modifie_le",
            params![id, f.nom.trim(), f.telephone.trim(), f.notes, f.actif, op.maintenant],
        )?;
        op.outbox("fournisseur", &id, "enregistrer")?;
        Ok(id)
    })
}

#[derive(Debug, Deserialize)]
pub struct LigneAchatSaisie {
    pub article_id: String,
    #[serde(default)]
    pub conditionnement_id: Option<String>,
    /// Nombre de conditionnements (ou d'unités de base sans conditionnement).
    pub quantite: i64,
    /// Prix total payé pour la ligne.
    pub prix_total: i64,
}

#[derive(Debug, Deserialize)]
pub struct NouvelAchat {
    /// RG-ACH-01 : absent = achat au marché.
    #[serde(default)]
    pub fournisseur_id: Option<String>,
    /// comptant | credit
    pub mode: String,
    /// Compte payeur si comptant (défaut : caisse de la session).
    #[serde(default)]
    pub compte_id: Option<String>,
    pub lignes: Vec<LigneAchatSaisie>,
    #[serde(default)]
    pub note: String,
}

/// Réception d'achat : RG-ACH-01 à 03, RG-STK-03/06.
pub fn receptionner(db: &mut Db, acteur: &Acteur, a: &NouvelAchat) -> Resultat<String> {
    db.executer(acteur, |op| {
        op.exiger(perm::ACHAT_GERER)?;
        if a.lignes.is_empty() {
            return Err(Erreur::validation("Ajoutez au moins un article"));
        }
        let journee = crate::journee::ouverte(op)?.map(|j| j.id);
        let total: i64 = a.lignes.iter().map(|l| l.prix_total).sum();
        let id = op.nouvel_id();
        let numero = op.sequence("achat")?;
        let (compte, session) = match a.mode.as_str() {
            "comptant" => {
                let session = op.utilisateur().map(|u| caisse::session_utilisateur(op, u)).transpose()?.flatten();
                let compte = match (&a.compte_id, &session) {
                    (Some(c), _) => c.clone(),
                    (None, Some(s)) => s.compte_id.clone(),
                    (None, None) => {
                        return Err(Erreur::regle("RG-ACH-02", "Achat comptant : ouvrez une session de caisse ou choisissez un compte"))
                    }
                };
                (Some(compte), session.map(|s| s.id))
            }
            "credit" => {
                if a.fournisseur_id.is_none() {
                    return Err(Erreur::regle("RG-ACH-02", "Un achat à crédit exige un fournisseur"));
                }
                (None, None)
            }
            _ => return Err(Erreur::validation("Mode d'achat inconnu")),
        };
        op.execute(
            "INSERT INTO achats(id, numero, fournisseur_id, journee_id, mode, compte_id, total, note, horodatage, utilisateur_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![id, numero, a.fournisseur_id, journee, a.mode, compte, total, a.note.trim(), op.maintenant, op.utilisateur()],
        )?;
        for l in &a.lignes {
            if l.quantite <= 0 || l.prix_total < 0 {
                return Err(Erreur::validation("Quantité ou prix invalide"));
            }
            let contenance = stock::contenance(op, l.conditionnement_id.as_deref())?;
            let base = l.quantite * contenance;
            // RG-STK-06 : coût unitaire arrondi à l'entier le plus proche.
            let cout = (l.prix_total + base / 2) / base;
            op.execute(
                "INSERT INTO lignes_achat(id, achat_id, article_id, conditionnement_id, quantite_saisie, quantite_base, prix_total, cout_unitaire)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![op.nouvel_id(), id, l.article_id, l.conditionnement_id, l.quantite, base, l.prix_total, cout],
            )?;
            stock::inserer_mouvement(op, &l.article_id, "achat", base, cout, "", Some(("achat", &id)))?;
            op.execute(
                "UPDATE articles_stock SET cout_unitaire = ?1, modifie_le = ?2 WHERE id = ?3",
                params![cout, op.maintenant, l.article_id],
            )?;
        }
        let fournisseur_nom: String = match &a.fournisseur_id {
            Some(f) => trouver(op.query_row("SELECT nom FROM fournisseurs WHERE id = ?1", params![f], |r| r.get(0)), "Fournisseur")?,
            None => "marché".into(),
        };
        if let Some(c) = &compte {
            if total > 0 {
                caisse::mouvement(op, c, session.as_deref(), "achat", -total, Some(("achat", &id)), &format!("Achat n°{numero} ({fournisseur_nom})"), None)?;
            }
        } else if let Some(f) = &a.fournisseur_id {
            op.execute(
                "INSERT INTO mouvements_fournisseur(id, fournisseur_id, type, montant, achat_id, horodatage, utilisateur_id)
                 VALUES (?1, ?2, 'achat_credit', ?3, ?4, ?5, ?6)",
                params![op.nouvel_id(), f, total, id, op.maintenant, op.utilisateur()],
            )?;
        }
        op.audit("achat.receptionner", "achat", Some(&id), None, Some(json!({ "total": total, "mode": a.mode })), None, None)?;
        op.outbox("achat", &id, "creer")?;
        op.evenement("stock", None);
        Ok(id)
    })
}

#[derive(Debug, Deserialize)]
pub struct ReglementFournisseur {
    pub fournisseur_id: String,
    pub montant: i64,
    #[serde(default)]
    pub compte_id: Option<String>,
    #[serde(default)]
    pub note: String,
}

/// RG-ACH-04.
pub fn regler(db: &mut Db, acteur: &Acteur, r: &ReglementFournisseur) -> Resultat<String> {
    db.executer(acteur, |op| {
        op.exiger(perm::ACHAT_GERER)?;
        let d = dette(op, &r.fournisseur_id)?;
        if r.montant <= 0 || r.montant > d {
            return Err(Erreur::regle("RG-ACH-04", format!("Montant invalide (dette actuelle : {d})")));
        }
        let session = op.utilisateur().map(|u| caisse::session_utilisateur(op, u)).transpose()?.flatten();
        let compte = match (&r.compte_id, &session) {
            (Some(c), _) => c.clone(),
            (None, Some(s)) => s.compte_id.clone(),
            (None, None) => return Err(Erreur::regle("RG-CAI-01", "Ouvrez une session de caisse ou choisissez un compte")),
        };
        let mvt = caisse::mouvement(op, &compte, session.as_ref().map(|s| s.id.as_str()), "reglement_fournisseur", -r.montant, Some(("fournisseur", &r.fournisseur_id)), r.note.trim(), None)?;
        let id = op.nouvel_id();
        op.execute(
            "INSERT INTO mouvements_fournisseur(id, fournisseur_id, type, montant, mouvement_tresorerie_id, motif, horodatage, utilisateur_id)
             VALUES (?1, ?2, 'reglement', ?3, ?4, ?5, ?6, ?7)",
            params![id, r.fournisseur_id, -r.montant, mvt, r.note.trim(), op.maintenant, op.utilisateur()],
        )?;
        op.audit("fournisseur.reglement", "fournisseur", Some(&r.fournisseur_id), None, Some(json!({ "montant": r.montant })), None, None)?;
        Ok(id)
    })
}

#[derive(Debug, Serialize)]
pub struct AchatLu {
    pub id: String,
    pub numero: i64,
    pub fournisseur: Option<String>,
    pub mode: String,
    pub total: i64,
    pub horodatage: i64,
    pub lignes: Vec<(String, i64, i64, i64)>,
}

pub fn lister_achats(conn: &Connection, limite: i64) -> Resultat<Vec<AchatLu>> {
    let mut s = conn.prepare(
        "SELECT a.id, a.numero, f.nom, a.mode, a.total, a.horodatage FROM achats a
         LEFT JOIN fournisseurs f ON f.id = a.fournisseur_id ORDER BY a.horodatage DESC LIMIT ?1",
    )?;
    let base = s
        .query_map(params![limite], |r| {
            Ok(AchatLu { id: r.get(0)?, numero: r.get(1)?, fournisseur: r.get(2)?, mode: r.get(3)?, total: r.get(4)?, horodatage: r.get(5)?, lignes: vec![] })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let mut v = Vec::new();
    for mut a in base {
        let mut s = conn.prepare_cached(
            "SELECT ar.nom, l.quantite_base, l.prix_total, l.cout_unitaire FROM lignes_achat l
             JOIN articles_stock ar ON ar.id = l.article_id WHERE l.achat_id = ?1",
        )?;
        a.lignes = s.query_map(params![a.id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?.collect::<Result<_, _>>()?;
        v.push(a);
    }
    Ok(v)
}

/// Historique des prix d'achat d'un article (cahier §10).
pub fn historique_prix_achat(conn: &Connection, article_id: &str) -> Resultat<Vec<(i64, i64, Option<String>)>> {
    let mut s = conn.prepare(
        "SELECT a.horodatage, l.cout_unitaire, f.nom FROM lignes_achat l JOIN achats a ON a.id = l.achat_id
         LEFT JOIN fournisseurs f ON f.id = a.fournisseur_id WHERE l.article_id = ?1 ORDER BY a.horodatage DESC, a.numero DESC LIMIT 100",
    )?;
    let v = s.query_map(params![article_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?.collect::<Result<Vec<_>, _>>()?;
    Ok(v)
}
