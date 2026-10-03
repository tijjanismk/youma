//! Fidélité par points (RG-FID-01 à 05) et clients privilégiés (RG-VIP-01 à 03), fiche 0039.
//! Le solde de points est la somme des mouvements (ajout seul) ; utiliser des points crée une remise tracée.

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::db::{trouver, Acteur, Db, Op};
use crate::erreur::{Erreur, Resultat};
use crate::permissions as perm;
use crate::{commandes, horloge};

pub fn solde(conn: &Connection, client_id: &str) -> Resultat<i64> {
    Ok(conn.query_row("SELECT COALESCE(SUM(points), 0) FROM mouvements_fidelite WHERE client_id = ?1", params![client_id], |r| r.get(0))?)
}

#[derive(Debug, Serialize)]
pub struct EtatFidelite {
    pub active: bool,
    pub points: i64,
    /// Valeur des points en FCFA, aux règles du moment.
    pub valeur: i64,
    pub minimum_points: i64,
    pub valeur_point: i64,
    pub fcfa_par_point: i64,
}

pub fn etat(conn: &Connection, client_id: &str) -> Resultat<EtatFidelite> {
    let f = crate::parametres::lire(conn)?.fidelite;
    let points = solde(conn, client_id)?;
    Ok(EtatFidelite {
        active: f.active,
        points,
        valeur: points * f.valeur_point,
        minimum_points: f.minimum_points,
        valeur_point: f.valeur_point,
        fcfa_par_point: f.fcfa_par_point,
    })
}

#[derive(Debug, Serialize)]
pub struct MouvementFidelite {
    pub type_: String,
    pub points: i64,
    pub commande_numero: Option<i64>,
    pub motif: String,
    pub horodatage: i64,
}

pub fn historique(conn: &Connection, client_id: &str) -> Resultat<Vec<MouvementFidelite>> {
    let mut s = conn.prepare(
        "SELECT m.type, m.points, c.numero, m.motif, m.horodatage FROM mouvements_fidelite m
         LEFT JOIN commandes c ON c.id = m.commande_id WHERE m.client_id = ?1 ORDER BY m.horodatage DESC LIMIT 100",
    )?;
    let v = s
        .query_map(params![client_id], |r| {
            Ok(MouvementFidelite { type_: r.get(0)?, points: r.get(1)?, commande_numero: r.get(2)?, motif: r.get(3)?, horodatage: r.get(4)? })
        })?
        .collect::<Result<_, _>>()?;
    Ok(v)
}

fn inserer(op: &Op, client_id: &str, type_: &str, points: i64, commande_id: Option<&str>, remise_id: Option<&str>, motif: &str) -> Resultat<()> {
    op.execute(
        "INSERT INTO mouvements_fidelite(id, client_id, type, points, commande_id, remise_id, motif, horodatage, utilisateur_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![op.nouvel_id(), client_id, type_, points, commande_id, remise_id, motif, op.maintenant, op.utilisateur()],
    )?;
    Ok(())
}

/// Points déjà acquis (nets) sur une commande et client à qui ils reviennent.
fn gains_nets(op: &Op, commande_id: &str) -> Resultat<Vec<(String, i64)>> {
    let v = op
        .prepare(
            "SELECT client_id, SUM(points) FROM mouvements_fidelite WHERE commande_id = ?1 AND type IN ('gain','annulation_gain')
             GROUP BY client_id HAVING SUM(points) <> 0",
        )?
        .query_map(params![commande_id], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<_, _>>()?;
    Ok(v)
}

/// RG-FID-02 : à l'addition soldée, le client rattaché gagne un point par tranche de `fcfa_par_point` payée par lui.
/// Ne comptent pas : la part payée par une société (contrat) et les bons d'avoir offerts par le restaurant.
pub(crate) fn gagner_op(op: &Op, commande_id: &str) -> Resultat<i64> {
    let f = &op.params.fidelite;
    if !f.active {
        return Ok(0);
    }
    let client: Option<String> = op.query_row("SELECT client_id FROM commandes WHERE id = ?1", params![commande_id], |r| r.get(0))?;
    let Some(client) = client else { return Ok(0) };
    let base: i64 = op.query_row(
        "SELECT COALESCE(SUM(pp.montant), 0) FROM parts_paiement pp JOIN paiements p ON p.id = pp.paiement_id
         LEFT JOIN cartes_cadeaux cc ON pp.moyen = 'carte_cadeau' AND cc.code = pp.reference
         WHERE p.commande_id = ?1 AND pp.contrat_id IS NULL AND COALESCE(cc.type, '') <> 'avoir'",
        params![commande_id],
        |r| r.get(0),
    )?;
    let dus = base.max(0) / f.fcfa_par_point;
    let deja: i64 = gains_nets(op, commande_id)?.iter().filter(|(c, _)| *c == client).map(|(_, p)| p).sum();
    let points = dus - deja;
    if points > 0 {
        inserer(op, &client, "gain", points, Some(commande_id), None, "")?;
    }
    Ok(points.max(0))
}

/// RG-FID-04 : paiement annulé, l'addition se rouvre : les points gagnés sur elle sont retirés.
pub(crate) fn annuler_gain_op(op: &Op, commande_id: &str, motif: &str) -> Resultat<()> {
    for (client, points) in gains_nets(op, commande_id)? {
        inserer(op, &client, "annulation_gain", -points, Some(commande_id), None, motif)?;
    }
    Ok(())
}

/// RG-FID-03 : utiliser des points = remise de `points × valeur_point` sur l'addition du client, sans plafond de
/// remise (c'est un droit du client), au plus le reste à payer.
pub fn utiliser(db: &mut Db, acteur: &Acteur, commande_id: &str, points: i64) -> Resultat<i64> {
    db.executer(acteur, |op| {
        op.exiger(perm::CAISSE_ENCAISSER)?;
        let f = op.params.fidelite.clone();
        if !f.active {
            return Err(Erreur::regle("RG-FID-03", "La fidélité n'est pas activée (Administration)"));
        }
        let e = commandes::etat(op, commande_id)?;
        commandes::exiger_ouverte(&e)?;
        let client: String = op
            .query_row("SELECT client_id FROM commandes WHERE id = ?1", params![commande_id], |r| r.get::<_, Option<String>>(0))?
            .ok_or_else(|| Erreur::regle("RG-FID-03", "Choisissez d'abord le client de l'addition"))?;
        let dispo = solde(op, &client)?;
        if points <= 0 || points > dispo {
            return Err(Erreur::regle("RG-FID-03", format!("Le client a {dispo} point(s)")));
        }
        if points < f.minimum_points {
            return Err(Erreur::regle("RG-FID-03", format!("Il faut au moins {} points pour les utiliser", f.minimum_points)));
        }
        let montant = points * f.valeur_point;
        let reste = commandes::totaux(op, commande_id)?.reste;
        if montant > reste {
            return Err(Erreur::regle(
                "RG-FID-03",
                format!("{points} points valent {montant} FCFA, plus que le reste à payer ({reste}) : au plus {} points", reste / f.valeur_point),
            ));
        }
        let motif = format!("Fidélité : {points} points");
        let remise = op.nouvel_id();
        op.execute(
            "INSERT INTO remises(id, commande_id, ligne_id, montant, motif, horodatage, utilisateur_id) VALUES (?1, ?2, NULL, ?3, ?4, ?5, ?6)",
            params![remise, commande_id, montant, motif, op.maintenant, op.utilisateur()],
        )?;
        inserer(op, &client, "utilisation", -points, Some(commande_id), Some(&remise), &motif)?;
        op.audit("fidelite.utiliser", "commande", Some(commande_id), None, Some(json!({ "client": client, "points": points, "montant": montant })), None, None)?;
        commandes::toucher(op, commande_id)?;
        Ok(montant)
    })
}

// ───────────── Clients privilégiés ─────────────

/// Condition SQL « client privilégié à la date ?N » sur l'alias `cl`.
pub(crate) fn sql_vip(param: &str) -> String {
    format!("(cl.vip = 1 AND (cl.vip_jusqu_au IS NULL OR cl.vip_jusqu_au >= {param}))")
}

/// Expression SQL « la commande `c` est celle d'un client privilégié » (client rattaché ou même téléphone), à la
/// date de la journée ouverte (lecture sans horloge).
pub(crate) const SQL_COMMANDE_VIP: &str = "EXISTS (SELECT 1 FROM clients cl WHERE (cl.id = c.client_id OR (c.client_telephone IS NOT NULL
    AND cl.telephone = c.client_telephone)) AND cl.vip = 1 AND (cl.vip_jusqu_au IS NULL OR cl.vip_jusqu_au >=
    COALESCE((SELECT date_exploitation FROM journees WHERE statut = 'ouverte' LIMIT 1), '0000-00-00')))";

pub(crate) fn aujourdhui(op_maintenant: i64, fuseau_minutes: i64) -> String {
    horloge::date_locale(op_maintenant, fuseau_minutes)
}

#[derive(Debug, Deserialize)]
pub struct Privilege {
    pub vip: bool,
    /// Date AAAA-MM-JJ incluse ; absent = sans fin.
    #[serde(default)]
    pub jusqu_au: Option<String>,
    #[serde(default)]
    pub motif: String,
}

/// RG-VIP-01 : un client devient privilégié (sans fin ou jusqu'à une date), par un caissier ou un gérant, tracé.
pub fn definir_privilege(db: &mut Db, acteur: &Acteur, client_id: &str, p: &Privilege) -> Resultat<()> {
    db.executer(acteur, |op| {
        let autorise_par = op.exiger(perm::CLIENT_CREDIT)?;
        crate::clients::client(op, client_id)?;
        let jusqu = p.jusqu_au.as_deref().map(str::trim).filter(|d| !d.is_empty());
        if let Some(d) = jusqu {
            if d.len() != 10 || d < aujourdhui(op.maintenant, op.params.fuseau_minutes).as_str() {
                return Err(Erreur::regle("RG-VIP-01", "Date de fin invalide ou déjà passée"));
            }
        }
        let avant: (bool, Option<String>) =
            trouver(op.query_row("SELECT vip, vip_jusqu_au FROM clients WHERE id = ?1", params![client_id], |r| Ok((r.get(0)?, r.get(1)?))), "Client")?;
        op.execute(
            "UPDATE clients SET vip = ?1, vip_jusqu_au = ?2, modifie_le = ?3, version = version + 1 WHERE id = ?4",
            params![p.vip, if p.vip { jusqu } else { None }, op.maintenant, client_id],
        )?;
        op.audit(
            "client.privilege",
            "client",
            Some(client_id),
            Some(json!({ "vip": avant.0, "jusqu_au": avant.1 })),
            Some(json!({ "vip": p.vip, "jusqu_au": jusqu })),
            Some(p.motif.trim()).filter(|m| !m.is_empty()),
            autorise_par.as_deref(),
        )?;
        op.outbox("client", client_id, "enregistrer")?;
        Ok(())
    })
}

/// Le client est-il privilégié aujourd'hui ?
pub fn est_vip(conn: &Connection, client_id: &str, date: &str) -> Resultat<bool> {
    Ok(conn
        .query_row(&format!("SELECT {} FROM clients cl WHERE cl.id = ?1", sql_vip("?2")), params![client_id, date], |r| r.get::<_, bool>(0))
        .optional()?
        .unwrap_or(false))
}
