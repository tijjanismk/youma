//! Contrats société (RG-SOC-01 à 04), fiche 0039 : une société prend en charge une part des repas de ses employés
//! (pourcentage ou montant par repas, plafond facultatif). Sa part est une vente à crédit sur le compte de la société
//! (cliente à crédit) ; l'employé paie le reste. Le relevé liste les repas à facturer à la société.

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::db::{Acteur, Db, Op};
use crate::erreur::{Erreur, Resultat};
use crate::permissions as perm;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Contrat {
    #[serde(default)]
    pub id: String,
    pub client_id: String,
    #[serde(default)]
    pub client_nom: String,
    pub nom: String,
    /// pourcentage | montant
    pub type_prise: String,
    /// % (1 à 100) ou FCFA par repas.
    pub valeur: i64,
    /// FCFA au plus par repas ; 0 = pas de plafond.
    #[serde(default)]
    pub plafond_repas: i64,
    #[serde(default = "vrai")]
    pub actif: bool,
    /// Ce que la société doit au restaurant (compte client).
    #[serde(default)]
    pub dette: i64,
}

fn vrai() -> bool {
    true
}

const COLS: &str = "k.id, k.client_id, cl.nom, k.nom, k.type_prise, k.valeur, k.plafond_repas, k.actif,
    (SELECT COALESCE(SUM(m.montant), 0) FROM mouvements_client m WHERE m.client_id = k.client_id)";

fn depuis(r: &rusqlite::Row) -> rusqlite::Result<Contrat> {
    Ok(Contrat {
        id: r.get(0)?,
        client_id: r.get(1)?,
        client_nom: r.get(2)?,
        nom: r.get(3)?,
        type_prise: r.get(4)?,
        valeur: r.get(5)?,
        plafond_repas: r.get(6)?,
        actif: r.get(7)?,
        dette: r.get(8)?,
    })
}

pub fn lister(conn: &Connection) -> Resultat<Vec<Contrat>> {
    let mut s = conn.prepare(&format!("SELECT {COLS} FROM contrats_societe k JOIN clients cl ON cl.id = k.client_id ORDER BY k.actif DESC, k.nom"))?;
    let v = s.query_map([], depuis)?.collect::<Result<_, _>>()?;
    Ok(v)
}

pub fn contrat(conn: &Connection, id: &str) -> Resultat<Contrat> {
    conn.query_row(&format!("SELECT {COLS} FROM contrats_societe k JOIN clients cl ON cl.id = k.client_id WHERE k.id = ?1"), params![id], depuis)
        .optional()?
        .ok_or_else(|| Erreur::NonTrouve("Contrat".into()))
}

/// RG-SOC-01 : contrat avec une société cliente autorisée à crédit (c'est sur son compte que va sa part).
pub fn enregistrer(db: &mut Db, acteur: &Acteur, c: &Contrat) -> Resultat<String> {
    db.executer(acteur, |op| {
        let autorise_par = op.exiger(perm::CLIENT_DEPASSER_LIMITE)?;
        if c.nom.trim().is_empty() {
            return Err(Erreur::validation("Nom du contrat obligatoire"));
        }
        match c.type_prise.as_str() {
            "pourcentage" if (1..=100).contains(&c.valeur) => {}
            "montant" if c.valeur > 0 => {}
            _ => return Err(Erreur::regle("RG-SOC-01", "Part de la société : 1 à 100 %, ou un montant en FCFA")),
        }
        if c.plafond_repas < 0 {
            return Err(Erreur::validation("Plafond invalide"));
        }
        let societe = crate::clients::client(op, &c.client_id)?;
        if !societe.credit_autorise {
            return Err(Erreur::regle("RG-SOC-01", format!("Autorisez d'abord le crédit de {} (fiche client) : sa part y est inscrite", societe.nom)));
        }
        let id = if c.id.is_empty() { op.nouvel_id() } else { c.id.clone() };
        op.execute(
            "INSERT INTO contrats_societe(id, client_id, nom, type_prise, valeur, plafond_repas, actif, cree_le, modifie_le)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)
             ON CONFLICT(id) DO UPDATE SET client_id = excluded.client_id, nom = excluded.nom, type_prise = excluded.type_prise,
               valeur = excluded.valeur, plafond_repas = excluded.plafond_repas, actif = excluded.actif, modifie_le = excluded.modifie_le",
            params![id, c.client_id, c.nom.trim(), c.type_prise, c.valeur, c.plafond_repas, c.actif, op.maintenant],
        )?;
        op.audit(
            "contrat.enregistrer",
            "contrat_societe",
            Some(&id),
            None,
            Some(json!({ "societe": societe.nom, "type": c.type_prise, "valeur": c.valeur, "plafond": c.plafond_repas, "actif": c.actif })),
            None,
            autorise_par.as_deref(),
        )?;
        Ok(id)
    })
}

/// RG-SOC-02 : part de la société sur un repas de `total` FCFA.
pub fn part_societe(c: &Contrat, total: i64) -> i64 {
    let part = match c.type_prise.as_str() {
        "pourcentage" => total * c.valeur / 100,
        _ => c.valeur,
    };
    let part = if c.plafond_repas > 0 { part.min(c.plafond_repas) } else { part };
    part.clamp(0, total.max(0))
}

/// RG-SOC-02/03 : à l'encaissement, contrôle la part société d'une addition et renvoie la société à débiter.
/// Employé nommé obligatoire ; la part ne dépasse pas ce que le contrat prévoit pour ce repas (parts déjà
/// payées par ce contrat comprises).
pub(crate) fn controler_part_op(op: &Op, contrat_id: &str, commande_id: &str, montant: i64, employe: &str) -> Resultat<String> {
    let c = contrat(op, contrat_id)?;
    if !c.actif {
        return Err(Erreur::regle("RG-SOC-02", format!("Le contrat {} n'est plus actif", c.nom)));
    }
    if employe.trim().is_empty() {
        return Err(Erreur::regle("RG-SOC-02", "Nom de l'employé de la société obligatoire (relevé à facturer)"));
    }
    let t = crate::commandes::totaux(op, commande_id)?;
    let deja: i64 = op.query_row(
        "SELECT COALESCE(SUM(pp.montant), 0) FROM parts_paiement pp JOIN paiements p ON p.id = pp.paiement_id
         WHERE p.commande_id = ?1 AND pp.contrat_id = ?2",
        params![commande_id, contrat_id],
        |r| r.get(0),
    )?;
    let permis = part_societe(&c, t.total) - deja;
    if montant > permis {
        return Err(Erreur::regle("RG-SOC-02", format!("Le contrat {} prend en charge au plus {} FCFA sur ce repas", c.nom, permis.max(0))));
    }
    Ok(c.client_id)
}

#[derive(Debug, Serialize)]
pub struct LigneReleve {
    pub date: String,
    pub commande_numero: i64,
    pub employe: String,
    pub total_repas: i64,
    pub part_societe: i64,
}

/// RG-SOC-04 : repas pris en charge sur une période (dates d'exploitation incluses), à facturer à la société.
/// Un paiement annulé apparaît en négatif (contre-passation).
pub fn releve(conn: &Connection, contrat_id: &str, debut: &str, fin: &str) -> Resultat<Vec<LigneReleve>> {
    let mut s = conn.prepare(
        "SELECT j.date_exploitation, c.numero, COALESCE(pp.reference, ''), c.id, pp.montant
         FROM parts_paiement pp JOIN paiements p ON p.id = pp.paiement_id JOIN journees j ON j.id = p.journee_id
         JOIN commandes c ON c.id = p.commande_id
         WHERE pp.contrat_id = ?1 AND j.date_exploitation BETWEEN ?2 AND ?3
         ORDER BY p.horodatage",
    )?;
    let lignes: Vec<(String, i64, String, String, i64)> =
        s.query_map(params![contrat_id, debut, fin], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)))?.collect::<Result<_, _>>()?;
    lignes
        .into_iter()
        .map(|(date, numero, employe, commande_id, part)| {
            Ok(LigneReleve { date, commande_numero: numero, employe, total_repas: crate::commandes::totaux(conn, &commande_id)?.total, part_societe: part })
        })
        .collect()
}
