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
