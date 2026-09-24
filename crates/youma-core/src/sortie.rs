//! Bon de sortie : le ticket de caisse payé prouve que le client a payé ce qu'il emporte.
//! Pas de facture séparée (décision du porteur de projet, fiche 0012).

use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use serde_json::json;
use sha2::{Digest, Sha256};

use crate::db::{installation_id, Acteur, Db};
use crate::erreur::{Erreur, Resultat};
use crate::permissions as perm;

/// Sans 0/O ni 1/I/L : lisible et dictable.
const ALPHABET: &[u8] = b"ABCDEFGHJKMNPQRSTUVWXYZ23456789";

/// RG-SOR-02 : code de contrôle à 4 caractères, propre à l'installation et à la commande.
/// Un numéro de ticket inventé ou recopié d'un autre restaurant ne passe pas.
pub fn code_controle(conn: &Connection, commande_id: &str) -> Resultat<String> {
    let h = Sha256::digest(format!("{}|sortie|{commande_id}", installation_id(conn)?).as_bytes());
    Ok(h.iter().take(4).map(|b| ALPHABET[*b as usize % ALPHABET.len()] as char).collect())
}

#[derive(Debug, Serialize)]
pub struct Presentation {
    pub horodatage: i64,
    pub par: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ResultatControle {
    /// paye | non_paye
    pub statut: String,
    pub numero: i64,
    pub titre: String,
    pub total: i64,
    pub reste: i64,
    pub payee_le: Option<i64>,
    pub lignes: Vec<(i64, String)>,
    /// Présentations antérieures du même ticket (RG-SOR-03).
    pub deja_presente: Vec<Presentation>,
}

/// Contrôle d'un ticket à la sortie : numéro + code imprimés sur le ticket.
/// RG-SOR-01 : seul un ticket entièrement payé vaut bon de sortie.
/// RG-SOR-03 : chaque présentation est enregistrée ; une deuxième est signalée.
pub fn controler(db: &mut Db, acteur: &Acteur, numero: i64, code: &str) -> Resultat<ResultatControle> {
    db.executer(acteur, |op| {
        op.exiger(perm::SORTIE_CONTROLER)?;
        let id: String = op
            .query_row("SELECT id FROM commandes WHERE numero = ?1", params![numero], |r| r.get(0))
            .optional()?
            .ok_or_else(|| Erreur::regle("RG-SOR-02", format!("Aucun ticket n°{numero}")))?;
        if !code.trim().eq_ignore_ascii_case(&code_controle(op, &id)?) {
            op.audit("sortie.code_faux", "commande", Some(&id), None, Some(json!({ "numero": numero })), None, None)?;
            return Err(Erreur::regle("RG-SOR-02", "Code de contrôle faux : ticket douteux"));
        }
        let c = crate::commandes::detail(op, &id)?;
        let paye = matches!(c.statut.as_str(), "payee" | "cloturee") && c.totaux.reste == 0;
        let payee_le: Option<i64> = op.query_row("SELECT payee_le FROM commandes WHERE id = ?1", params![id], |r| r.get(0))?;
        let deja_presente = op
            .prepare(
                "SELECT s.horodatage, u.nom FROM controles_sortie s LEFT JOIN utilisateurs u ON u.id = s.utilisateur_id
                 WHERE s.commande_id = ?1 ORDER BY s.horodatage",
            )?
            .query_map(params![id], |r| Ok(Presentation { horodatage: r.get(0)?, par: r.get(1)? }))?
            .collect::<Result<Vec<_>, _>>()?;
        if paye {
            op.execute(
                "INSERT INTO controles_sortie(id, commande_id, horodatage, utilisateur_id, appareil_id) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![op.nouvel_id(), id, op.maintenant, op.utilisateur(), op.appareil_id],
            )?;
            if !deja_presente.is_empty() {
                op.audit("sortie.deja_presente", "commande", Some(&id), None, Some(json!({ "numero": numero, "fois": deja_presente.len() + 1 })), None, None)?;
            }
        }
        let e = crate::commandes::etat(op, &id)?;
        Ok(ResultatControle {
            statut: if paye { "paye" } else { "non_paye" }.into(),
            numero,
            titre: crate::commandes::titre_commande(&e),
            total: c.totaux.total,
            reste: c.totaux.reste,
            payee_le,
            lignes: c
                .lignes
                .iter()
                .filter(|l| l.quantite > l.quantite_annulee)
                .map(|l| (l.quantite - l.quantite_annulee, l.libelle.clone()))
                .collect(),
            deja_presente,
        })
    })
}
