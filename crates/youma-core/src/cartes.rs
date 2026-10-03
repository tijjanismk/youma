//! Cartes cadeaux et bons d'avoir (RG-CAD-01 à 05), fiche 0039.
//! Une carte cadeau est payée d'avance : l'argent entre en caisse à la vente mais n'est pas du chiffre d'affaires ;
//! la vente est comptée quand la carte paie une addition. Un bon d'avoir est une carte offerte par le restaurant
//! (client mécontent). Solde = somme des mouvements de la carte (ajout seul).

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::db::{Acteur, Db, Op};
use crate::entrantes::code_aleatoire;
use crate::erreur::{Erreur, Resultat};
use crate::fidelite::aujourdhui;
use crate::permissions as perm;

/// Code saisi à la caisse : majuscules, sans espace ni tiret (« ab12-cd34 » → « AB12CD34 »).
pub fn normaliser_code(code: &str) -> String {
    code.chars().filter(|c| c.is_ascii_alphanumeric()).collect::<String>().to_ascii_uppercase()
}

/// Code affiché et imprimé : « AB12-CD34 ».
pub fn code_lisible(code: &str) -> String {
    if code.len() == 8 {
        format!("{}-{}", &code[..4], &code[4..])
    } else {
        code.to_string()
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct Carte {
    pub id: String,
    pub code: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub montant_initial: i64,
    pub solde: i64,
    pub client_id: Option<String>,
    pub client_nom: Option<String>,
    pub beneficiaire: String,
    pub motif: String,
    pub expire_le: Option<String>,
    pub cree_le: i64,
}

const COLS: &str = "k.id, k.code, k.type, k.montant_initial,
    (SELECT COALESCE(SUM(m.montant), 0) FROM mouvements_carte m WHERE m.carte_id = k.id),
    k.client_id, cl.nom, k.beneficiaire, k.motif, k.expire_le, k.cree_le";

fn carte_depuis(r: &rusqlite::Row) -> rusqlite::Result<Carte> {
    let code: String = r.get(1)?;
    Ok(Carte {
        id: r.get(0)?,
        code: code_lisible(&code),
        type_: r.get(2)?,
        montant_initial: r.get(3)?,
        solde: r.get(4)?,
        client_id: r.get(5)?,
        client_nom: r.get(6)?,
        beneficiaire: r.get(7)?,
        motif: r.get(8)?,
        expire_le: r.get(9)?,
        cree_le: r.get(10)?,
    })
}

pub fn lister(conn: &Connection) -> Resultat<Vec<Carte>> {
    let mut s = conn.prepare(&format!("SELECT {COLS} FROM cartes_cadeaux k LEFT JOIN clients cl ON cl.id = k.client_id ORDER BY k.cree_le DESC LIMIT 300"))?;
    let v = s.query_map([], carte_depuis)?.collect::<Result<_, _>>()?;
    Ok(v)
}

pub fn par_code(conn: &Connection, code: &str) -> Resultat<Carte> {
    conn.query_row(
        &format!("SELECT {COLS} FROM cartes_cadeaux k LEFT JOIN clients cl ON cl.id = k.client_id WHERE k.code = ?1"),
        params![normaliser_code(code)],
        carte_depuis,
    )
    .optional()?
    .ok_or_else(|| Erreur::regle("RG-CAD-02", "Code de carte inconnu"))
}

fn nouveau_code(op: &Op) -> Resultat<String> {
    loop {
        let c = code_aleatoire(8);
        let pris: i64 = op.query_row("SELECT COUNT(*) FROM cartes_cadeaux WHERE code = ?1", params![c], |r| r.get(0))?;
        if pris == 0 {
            return Ok(c);
        }
    }
}

fn valider_expiration(op: &Op, expire_le: Option<&str>) -> Resultat<Option<String>> {
    let d = expire_le.map(str::trim).filter(|d| !d.is_empty());
    if let Some(d) = d {
        if d.len() != 10 || d < aujourdhui(op.maintenant, op.params.fuseau_minutes).as_str() {
            return Err(Erreur::regle("RG-CAD-01", "Date de fin de validité invalide ou déjà passée"));
        }
    }
    Ok(d.map(str::to_string))
}

#[allow(clippy::too_many_arguments)]
fn creer(op: &Op, type_: &str, montant: i64, client_id: Option<&str>, beneficiaire: &str, motif: &str, expire_le: Option<&str>, autorise_par: Option<&str>) -> Resultat<(String, String)> {
    if montant <= 0 {
        return Err(Erreur::regle("RG-CAD-01", "Montant de la carte invalide"));
    }
    if let Some(c) = client_id {
        crate::clients::client(op, c)?;
    }
    let expire = valider_expiration(op, expire_le)?;
    let id = op.nouvel_id();
    let code = nouveau_code(op)?;
    op.execute(
        "INSERT INTO cartes_cadeaux(id, code, type, montant_initial, client_id, beneficiaire, motif, expire_le, cree_le, utilisateur_id, autorise_par)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![id, code, type_, montant, client_id, beneficiaire.trim(), motif.trim(), expire, op.maintenant, op.utilisateur(), autorise_par],
    )?;
    Ok((id, code))
}

#[derive(Debug, Deserialize)]
pub struct VenteCarte {
    pub montant: i64,
    /// especes | mobile_money
    pub moyen: String,
    #[serde(default)]
    pub compte_id: Option<String>,
    #[serde(default)]
    pub reference: String,
    #[serde(default)]
    pub client_id: Option<String>,
    #[serde(default)]
    pub beneficiaire: String,
    #[serde(default)]
    pub expire_le: Option<String>,
}

/// RG-CAD-01 : vente d'une carte cadeau. L'argent entre dans la caisse de la session (espèces) ou sur le compte
/// Mobile Money, en mouvement « vente_carte_cadeau » : ce n'est pas une vente du jour (pas de chiffre d'affaires).
pub fn vendre(db: &mut Db, acteur: &Acteur, v: &VenteCarte) -> Resultat<Carte> {
    let (id, _) = db.executer(acteur, |op| {
        op.exiger(perm::CAISSE_ENCAISSER)?;
        let uid = op.utilisateur().ok_or(Erreur::NonAuthentifie)?.to_string();
        let session = crate::caisse::session_utilisateur(op, &uid)?
            .ok_or_else(|| Erreur::regle("RG-CAI-01", "Ouvrez votre session de caisse avant de vendre une carte"))?;
        let compte = match v.moyen.as_str() {
            "especes" => session.compte_id.clone(),
            "mobile_money" => {
                let c = v.compte_id.clone().ok_or_else(|| Erreur::validation("Choisissez l'opérateur Mobile Money"))?;
                let t: String = op.query_row("SELECT type FROM comptes_tresorerie WHERE id = ?1", params![c], |r| r.get(0))?;
                if t != "mobile_money" {
                    return Err(Erreur::validation("Ce compte n'est pas un compte Mobile Money"));
                }
                if op.params.reference_mm_obligatoire && v.reference.trim().is_empty() {
                    return Err(Erreur::regle("RG-CAI-04", "Référence de transaction Mobile Money obligatoire"));
                }
                c
            }
            _ => return Err(Erreur::validation("Une carte cadeau se paie en espèces ou en Mobile Money")),
        };
        let (id, code) = creer(op, "cadeau", v.montant, v.client_id.as_deref(), &v.beneficiaire, "", v.expire_le.as_deref(), None)?;
        let libelle = format!("Carte cadeau {}{}", code_lisible(&code), if v.reference.trim().is_empty() { String::new() } else { format!(" — réf. {}", v.reference.trim()) });
        let mvt = crate::caisse::mouvement(op, &compte, Some(&session.id), "vente_carte_cadeau", v.montant, Some(("carte_cadeau", &id)), &libelle, None)?;
        op.execute(
            "INSERT INTO mouvements_carte(id, carte_id, type, montant, mouvement_tresorerie_id, horodatage, utilisateur_id)
             VALUES (?1, ?2, 'emission', ?3, ?4, ?5, ?6)",
            params![op.nouvel_id(), id, v.montant, mvt, op.maintenant, op.utilisateur()],
        )?;
        op.audit("carte.vendre", "carte_cadeau", Some(&id), None, Some(json!({ "montant": v.montant, "moyen": v.moyen })), None, None)?;
        op.evenement("caisse", Some(&session.id));
        Ok((id, code))
    })?;
    carte(db.conn(), &id)
}

fn carte(conn: &Connection, id: &str) -> Resultat<Carte> {
    Ok(conn.query_row(&format!("SELECT {COLS} FROM cartes_cadeaux k LEFT JOIN clients cl ON cl.id = k.client_id WHERE k.id = ?1"), params![id], carte_depuis)?)
}

#[derive(Debug, Deserialize)]
pub struct NouvelAvoir {
    pub montant: i64,
    #[serde(default)]
    pub client_id: Option<String>,
    #[serde(default)]
    pub beneficiaire: String,
    pub motif: String,
    #[serde(default)]
    pub expire_le: Option<String>,
}

/// RG-CAD-04 : bon d'avoir offert (client mécontent) : motif obligatoire, accord du gérant (comme un article offert).
/// Aucun argent n'entre ; à l'utilisation, il réduit ce que le client paie.
pub fn offrir_avoir(db: &mut Db, acteur: &Acteur, a: &NouvelAvoir) -> Resultat<Carte> {
    let id = db.executer(acteur, |op| {
        let autorise_par = op.exiger(perm::COMMANDE_OFFRIR)?;
        if a.motif.trim().is_empty() {
            return Err(Erreur::regle("RG-CAD-04", "Motif obligatoire pour un bon d'avoir"));
        }
        if a.client_id.is_none() && a.beneficiaire.trim().is_empty() {
            return Err(Erreur::regle("RG-CAD-04", "Indiquez le client qui reçoit le bon"));
        }
        let (id, _) = creer(op, "avoir", a.montant, a.client_id.as_deref(), &a.beneficiaire, &a.motif, a.expire_le.as_deref(), autorise_par.as_deref())?;
        op.execute(
            "INSERT INTO mouvements_carte(id, carte_id, type, montant, horodatage, utilisateur_id) VALUES (?1, ?2, 'emission', ?3, ?4, ?5)",
            params![op.nouvel_id(), id, a.montant, op.maintenant, op.utilisateur()],
        )?;
        op.audit("carte.avoir", "carte_cadeau", Some(&id), None, Some(json!({ "montant": a.montant, "client": a.client_id })), Some(a.motif.trim()), autorise_par.as_deref())?;
        Ok(id)
    })?;
    carte(db.conn(), &id)
}

/// RG-CAD-02/03 : paiement d'une addition par carte (appelé à l'encaissement). Carte connue, non expirée, solde
/// suffisant ; renvoie le code normalisé gardé sur la part de paiement.
pub(crate) fn debiter_op(op: &Op, code: &str, montant: i64, paiement_id: &str) -> Resultat<String> {
    let code = normaliser_code(code);
    let c = par_code(op, &code)?;
    if let Some(d) = &c.expire_le {
        if d.as_str() < aujourdhui(op.maintenant, op.params.fuseau_minutes).as_str() {
            return Err(Erreur::regle("RG-CAD-03", format!("Carte {} expirée depuis le {d}", c.code)));
        }
    }
    if montant > c.solde {
        return Err(Erreur::regle("RG-CAD-02", format!("Solde de la carte {} : {} FCFA", c.code, c.solde)));
    }
    op.execute(
        "INSERT INTO mouvements_carte(id, carte_id, type, montant, paiement_id, horodatage, utilisateur_id)
         VALUES (?1, ?2, 'utilisation', ?3, ?4, ?5, ?6)",
        params![op.nouvel_id(), c.id, -montant, paiement_id, op.maintenant, op.utilisateur()],
    )?;
    Ok(code)
}

/// RG-CAD-05 : paiement annulé, la carte retrouve son solde.
pub(crate) fn recrediter_op(op: &Op, code: &str, montant: i64, paiement_id: &str) -> Resultat<()> {
    let c = par_code(op, code)?;
    op.execute(
        "INSERT INTO mouvements_carte(id, carte_id, type, montant, paiement_id, horodatage, utilisateur_id)
         VALUES (?1, ?2, 'annulation_utilisation', ?3, ?4, ?5, ?6)",
        params![op.nouvel_id(), c.id, montant, paiement_id, op.maintenant, op.utilisateur()],
    )?;
    Ok(())
}

/// Cartes d'un client (écran client, encaissement).
pub fn du_client(conn: &Connection, client_id: &str) -> Resultat<Vec<Carte>> {
    let mut s = conn.prepare(&format!(
        "SELECT {COLS} FROM cartes_cadeaux k LEFT JOIN clients cl ON cl.id = k.client_id WHERE k.client_id = ?1 ORDER BY k.cree_le DESC"
    ))?;
    let v = s.query_map(params![client_id], carte_depuis)?.collect::<Result<_, _>>()?;
    Ok(v)
}
