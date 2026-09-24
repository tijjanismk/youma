use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::catalogue::non_vide;
use crate::commandes::{self, totaux};
use crate::db::{trouver, Acteur, Db, Op};
use crate::erreur::{Erreur, Resultat};
use crate::permissions as perm;

// ───────────── Comptes ─────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Compte {
    #[serde(default)]
    pub id: String,
    pub nom: String,
    #[serde(rename = "type")]
    pub type_: String,
    #[serde(default)]
    pub operateur: String,
    #[serde(default)]
    pub employe_id: Option<String>,
    #[serde(default = "vrai")]
    pub actif: bool,
    #[serde(default)]
    pub solde: i64,
}

fn vrai() -> bool {
    true
}

pub fn solde(conn: &Connection, compte_id: &str) -> Resultat<i64> {
    Ok(conn.query_row(
        "SELECT COALESCE(SUM(montant), 0) FROM mouvements_tresorerie WHERE compte_id = ?1",
        params![compte_id],
        |r| r.get(0),
    )?)
}

pub fn lister_comptes(conn: &Connection) -> Resultat<Vec<Compte>> {
    let mut s = conn.prepare(
        "SELECT c.id, c.nom, c.type, c.operateur, c.employe_id, c.actif,
                (SELECT COALESCE(SUM(montant), 0) FROM mouvements_tresorerie m WHERE m.compte_id = c.id)
         FROM comptes_tresorerie c ORDER BY c.ordre, c.nom",
    )?;
    let v = s
        .query_map([], |r| {
            Ok(Compte {
                id: r.get(0)?,
                nom: r.get(1)?,
                type_: r.get(2)?,
                operateur: r.get(3)?,
                employe_id: r.get(4)?,
                actif: r.get(5)?,
                solde: r.get(6)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(v)
}

pub fn enregistrer_compte(db: &mut Db, acteur: &Acteur, c: &Compte) -> Resultat<String> {
    db.executer(acteur, |op| {
        op.exiger(perm::PARAMETRE_GERER)?;
        non_vide(&c.nom, "nom du compte")?;
        if !["especes", "mobile_money", "banque", "coffre", "livreur"].contains(&c.type_.as_str()) {
            return Err(Erreur::validation("Type de compte inconnu"));
        }
        let id = if c.id.is_empty() { op.nouvel_id() } else { c.id.clone() };
        op.execute(
            "INSERT INTO comptes_tresorerie(id, nom, type, operateur, employe_id, actif, modifie_le)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(id) DO UPDATE SET nom = excluded.nom, operateur = excluded.operateur, actif = excluded.actif,
               modifie_le = excluded.modifie_le",
            params![id, c.nom.trim(), c.type_, c.operateur, c.employe_id, c.actif, op.maintenant],
        )?;
        op.audit("compte.enregistrer", "compte_tresorerie", Some(&id), None, Some(json!(c)), None, None)?;
        Ok(id)
    })
}

fn type_compte(conn: &Connection, compte_id: &str) -> Resultat<String> {
    trouver(
        conn.query_row(
            "SELECT type FROM comptes_tresorerie WHERE id = ?1 AND actif = 1",
            params![compte_id],
            |r| r.get(0),
        ),
        "Compte de trésorerie",
    )
}

/// Écrit un mouvement de trésorerie. RG-CAI-11 : un compte espèces ne devient jamais négatif.
#[allow(clippy::too_many_arguments)]
pub(crate) fn mouvement(
    op: &Op,
    compte_id: &str,
    session_id: Option<&str>,
    type_: &str,
    montant: i64,
    reference: Option<(&str, &str)>,
    libelle: &str,
    autorise_par: Option<&str>,
) -> Resultat<String> {
    let t = type_compte(op, compte_id)?;
    if montant < 0 && t == "especes" && solde(op, compte_id)? + montant < 0 {
        return Err(Erreur::regle("RG-CAI-11", "Pas assez d'espèces dans la caisse pour cette sortie"));
    }
    let journee = crate::journee::ouverte(op)?.map(|j| j.id);
    let id = op.nouvel_id();
    op.execute(
        "INSERT INTO mouvements_tresorerie(id, compte_id, session_id, journee_id, type, montant, reference_type,
            reference_id, libelle, horodatage, utilisateur_id, autorise_par)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        params![
            id,
            compte_id,
            session_id,
            journee,
            type_,
            montant,
            reference.map(|r| r.0),
            reference.map(|r| r.1),
            libelle,
            op.maintenant,
            op.utilisateur(),
            autorise_par
        ],
    )?;
    op.outbox("mouvement_tresorerie", &id, "creer")?;
    Ok(id)
}

// ───────────── Sessions ─────────────

#[derive(Debug, Serialize, Clone)]
pub struct Session {
    pub id: String,
    pub compte_id: String,
    pub compte_nom: String,
    pub caissier_id: String,
    pub caissier_nom: String,
    pub journee_id: String,
    pub statut: String,
    pub ouverte_le: i64,
    pub fond_compte: i64,
    pub theorique_ouverture: i64,
    pub fermee_le: Option<i64>,
    pub compte_final: Option<i64>,
    pub theorique_cloture: Option<i64>,
    pub ecart: Option<i64>,
    pub motif_ecart: Option<String>,
    /// Solde théorique actuel du compte espèces.
    pub solde_actuel: i64,
}

const COLS_SESSION: &str = "s.id, s.compte_id, c.nom, s.caissier_id, u.nom, s.journee_id, s.statut, s.ouverte_le, s.fond_compte,
    s.theorique_ouverture, s.fermee_le, s.compte_final, s.theorique_cloture, s.ecart, s.motif_ecart";

fn session_depuis(conn: &Connection, sql_where: &str, p: &[&dyn rusqlite::ToSql]) -> Resultat<Option<Session>> {
    let s = conn
        .query_row(
            &format!(
                "SELECT {COLS_SESSION} FROM sessions_caisse s JOIN comptes_tresorerie c ON c.id = s.compte_id
                 JOIN utilisateurs u ON u.id = s.caissier_id WHERE {sql_where}"
            ),
            p,
            |r| {
                Ok(Session {
                    id: r.get(0)?,
                    compte_id: r.get(1)?,
                    compte_nom: r.get(2)?,
                    caissier_id: r.get(3)?,
                    caissier_nom: r.get(4)?,
                    journee_id: r.get(5)?,
                    statut: r.get(6)?,
                    ouverte_le: r.get(7)?,
                    fond_compte: r.get(8)?,
                    theorique_ouverture: r.get(9)?,
                    fermee_le: r.get(10)?,
                    compte_final: r.get(11)?,
                    theorique_cloture: r.get(12)?,
                    ecart: r.get(13)?,
                    motif_ecart: r.get(14)?,
                    solde_actuel: 0,
                })
            },
        )
        .optional()?;
    match s {
        Some(mut s) => {
            s.solde_actuel = solde(conn, &s.compte_id)?;
            Ok(Some(s))
        }
        None => Ok(None),
    }
}

pub fn session(conn: &Connection, id: &str) -> Resultat<Session> {
    session_depuis(conn, "s.id = ?1", &[&id])?.ok_or_else(|| Erreur::NonTrouve("Session de caisse".into()))
}

/// Session ouverte de l'utilisateur (RG-CAI-01).
pub fn session_utilisateur(conn: &Connection, utilisateur_id: &str) -> Resultat<Option<Session>> {
    session_depuis(conn, "s.caissier_id = ?1 AND s.statut = 'ouverte'", &[&utilisateur_id])
}

pub fn sessions_ouvertes(conn: &Connection) -> Resultat<Vec<Session>> {
    let ids: Vec<String> = conn
        .prepare("SELECT id FROM sessions_caisse WHERE statut = 'ouverte' ORDER BY ouverte_le")?
        .query_map([], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    ids.iter().map(|i| session(conn, i)).collect()
}

pub fn sessions_journee(conn: &Connection, journee_id: &str) -> Resultat<Vec<Session>> {
    let ids: Vec<String> = conn
        .prepare("SELECT id FROM sessions_caisse WHERE journee_id = ?1 ORDER BY ouverte_le")?
        .query_map(params![journee_id], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    ids.iter().map(|i| session(conn, i)).collect()
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct LigneBilletage {
    pub coupure: i64,
    pub nombre: i64,
}

fn total_billetage(b: &[LigneBilletage]) -> Resultat<i64> {
    let mut t = 0;
    for l in b {
        if l.coupure <= 0 || l.nombre < 0 {
            return Err(Erreur::validation("Billetage invalide"));
        }
        t += l.coupure * l.nombre;
    }
    Ok(t)
}

fn enregistrer_billetage(op: &Op, session_id: &str, moment: &str, b: &[LigneBilletage]) -> Resultat<()> {
    for l in b.iter().filter(|l| l.nombre > 0) {
        op.execute(
            "INSERT INTO billetages(id, session_id, moment, coupure, nombre) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![op.nouvel_id(), session_id, moment, l.coupure, l.nombre],
        )?;
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct OuvertureSession {
    #[serde(default)]
    pub compte_id: Option<String>,
    pub fond_compte: i64,
    #[serde(default)]
    pub billetage: Vec<LigneBilletage>,
    #[serde(default)]
    pub motif_ecart: String,
}

fn caisse_principale(conn: &Connection) -> Resultat<String> {
    trouver(
        conn.query_row(
            "SELECT id FROM comptes_tresorerie WHERE type = 'especes' AND actif = 1 ORDER BY ordre LIMIT 1",
            [],
            |r| r.get(0),
        ),
        "Caisse espèces",
    )
}

/// RG-CAI-08.
pub fn ouvrir_session(db: &mut Db, acteur: &Acteur, o: &OuvertureSession) -> Resultat<String> {
    db.executer(acteur, |op| {
        op.exiger(perm::CAISSE_SESSION)?;
        let journee = op.journee_ouverte()?;
        let uid = op.utilisateur().ok_or(Erreur::NonAuthentifie)?.to_string();
        if session_utilisateur(op, &uid)?.is_some() {
            return Err(Erreur::validation("Vous avez déjà une session de caisse ouverte"));
        }
        let compte = match &o.compte_id {
            Some(c) => c.clone(),
            None => caisse_principale(op)?,
        };
        if type_compte(op, &compte)? != "especes" {
            return Err(Erreur::validation("Une session s'ouvre sur une caisse espèces"));
        }
        let occupee: Option<String> = op
            .query_row(
                "SELECT u.nom FROM sessions_caisse s JOIN utilisateurs u ON u.id = s.caissier_id
                 WHERE s.compte_id = ?1 AND s.statut = 'ouverte'",
                params![compte],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(nom) = occupee {
            return Err(Erreur::validation(format!(
                "La caisse est tenue par {nom}. Passation : sa session doit être clôturée d'abord."
            )));
        }
        if o.fond_compte < 0 {
            return Err(Erreur::validation("Fond de caisse négatif"));
        }
        if !o.billetage.is_empty() && total_billetage(&o.billetage)? != o.fond_compte {
            return Err(Erreur::validation("Le billetage ne correspond pas au fond déclaré"));
        }
        let theorique = solde(op, &compte)?;
        let ecart = o.fond_compte - theorique;
        if ecart.abs() > op.params.seuil_ecart_caisse && o.motif_ecart.trim().is_empty() {
            return Err(Erreur::regle(
                "RG-CAI-08",
                format!("Écart de {ecart} FCFA avec le solde attendu ({theorique}) : motif obligatoire"),
            ));
        }
        let id = op.nouvel_id();
        op.execute(
            "INSERT INTO sessions_caisse(id, compte_id, journee_id, caissier_id, appareil_id, statut, ouverte_le,
                fond_compte, theorique_ouverture)
             VALUES (?1, ?2, ?3, ?4, ?5, 'ouverte', ?6, ?7, ?8)",
            params![id, compte, journee.id, uid, op.appareil_id, op.maintenant, o.fond_compte, theorique],
        )?;
        enregistrer_billetage(op, &id, "ouverture", &o.billetage)?;
        if ecart != 0 {
            mouvement(op, &compte, Some(&id), "ecart_ouverture", ecart, Some(("session_caisse", &id)), o.motif_ecart.trim(), None)?;
        }
        op.audit(
            "caisse.ouvrir",
            "session_caisse",
            Some(&id),
            None,
            Some(json!({ "fond": o.fond_compte, "theorique": theorique, "ecart": ecart })),
            Some(o.motif_ecart.trim()).filter(|m| !m.is_empty()),
            None,
        )?;
        op.evenement("caisse", Some(&id));
        Ok(id)
    })
}

#[derive(Debug, Deserialize)]
pub struct ClotureSession {
    pub compte_final: i64,
    #[serde(default)]
    pub billetage: Vec<LigneBilletage>,
    #[serde(default)]
    pub motif_ecart: String,
}

/// RG-CAI-09 : comptage, écart, mouvement d'alignement.
pub fn cloturer_session(db: &mut Db, acteur: &Acteur, session_id: &str, c: &ClotureSession) -> Resultat<Session> {
    db.executer(acteur, |op| {
        op.exiger(perm::CAISSE_SESSION)?;
        let s = session(op, session_id)?;
        if s.statut != "ouverte" {
            return Err(Erreur::validation("Session déjà clôturée"));
        }
        let autorise_par = if s.caissier_id.as_str() != op.utilisateur().unwrap_or_default() {
            op.exiger(perm::CAISSE_ECART)?
        } else {
            None
        };
        if c.compte_final < 0 {
            return Err(Erreur::validation("Montant compté négatif"));
        }
        if !c.billetage.is_empty() && total_billetage(&c.billetage)? != c.compte_final {
            return Err(Erreur::validation("Le billetage ne correspond pas au montant compté"));
        }
        let theorique = solde(op, &s.compte_id)?;
        let ecart = c.compte_final - theorique;
        if ecart.abs() > op.params.seuil_ecart_caisse && c.motif_ecart.trim().is_empty() {
            return Err(Erreur::regle(
                "RG-CAI-09",
                format!("Écart de {ecart} FCFA : motif obligatoire au-delà de {} FCFA", op.params.seuil_ecart_caisse),
            ));
        }
        if ecart != 0 {
            mouvement(op, &s.compte_id, Some(session_id), "ecart_cloture", ecart, Some(("session_caisse", session_id)), c.motif_ecart.trim(), autorise_par.as_deref())?;
        }
        enregistrer_billetage(op, session_id, "cloture", &c.billetage)?;
        op.execute(
            "UPDATE sessions_caisse SET statut = 'fermee', fermee_le = ?1, compte_final = ?2, theorique_cloture = ?3,
                ecart = ?4, motif_ecart = ?5 WHERE id = ?6",
            params![op.maintenant, c.compte_final, theorique, ecart, c.motif_ecart.trim(), session_id],
        )?;
        op.audit(
            "caisse.cloturer",
            "session_caisse",
            Some(session_id),
            None,
            Some(json!({ "compte": c.compte_final, "theorique": theorique, "ecart": ecart })),
            Some(c.motif_ecart.trim()).filter(|m| !m.is_empty()),
            autorise_par.as_deref(),
        )?;
        op.outbox("session_caisse", session_id, "cloturer")?;
        op.evenement("caisse", Some(session_id));
        Ok(())
    })?;
    session(db.conn(), session_id)
}

pub fn billetage(conn: &Connection, session_id: &str, moment: &str) -> Resultat<Vec<LigneBilletage>> {
    let mut s = conn.prepare(
        "SELECT coupure, nombre FROM billetages WHERE session_id = ?1 AND moment = ?2 ORDER BY coupure DESC",
    )?;
    let v = s
        .query_map(params![session_id, moment], |r| Ok(LigneBilletage { coupure: r.get(0)?, nombre: r.get(1)? }))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(v)
}

fn session_requise(op: &Op) -> Resultat<Session> {
    let uid = op.utilisateur().ok_or(Erreur::NonAuthentifie)?;
    session_utilisateur(op, uid)?
        .ok_or_else(|| Erreur::regle("RG-CAI-01", "Ouvrez votre session de caisse avant d'encaisser"))
}

// ───────────── Encaissement ─────────────

#[derive(Debug, Deserialize, Clone)]
pub struct PartSaisie {
    /// especes | mobile_money | virement | carte | credit
    pub moyen: String,
    pub montant: i64,
    #[serde(default)]
    pub compte_id: Option<String>,
    #[serde(default)]
    pub reference: Option<String>,
    #[serde(default)]
    pub numero_payeur: Option<String>,
    #[serde(default)]
    pub client_id: Option<String>,
    /// RG-LIV-02 : espèces encaissées par le livreur.
    #[serde(default)]
    pub par_livreur: bool,
}

#[derive(Debug, Deserialize)]
pub struct Encaissement {
    pub commande_id: String,
    pub parts: Vec<PartSaisie>,
    /// Espèces tendues par le client (pour le rendu monnaie).
    #[serde(default)]
    pub especes_recues: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ResultatEncaissement {
    pub paiement_id: String,
    pub numero: i64,
    pub montant: i64,
    /// RG-CAI-14 : espèces tendues par le client et monnaie rendue, conservées sur le paiement.
    pub especes_recues: i64,
    pub rendu: i64,
    pub reste: i64,
    pub commande_payee: bool,
}

/// RG-CAI-01 à RG-CAI-06, RG-CLI-02, RG-LIV-02.
pub fn encaisser(db: &mut Db, acteur: &Acteur, e: &Encaissement) -> Resultat<ResultatEncaissement> {
    db.executer(acteur, |op| encaisser_op(op, e))
}

pub(crate) fn encaisser_op(op: &mut Op, e: &Encaissement) -> Resultat<ResultatEncaissement> {
    op.exiger(perm::CAISSE_ENCAISSER)?;
    let session = session_requise(op)?;
    let journee = op.journee_ouverte()?;
    let etat = commandes::etat(op, &e.commande_id)?;
    if etat.statut != "ouverte" {
        return Err(Erreur::regle("RG-CMD-08", "Cette addition est déjà payée ou fermée"));
    }
    if etat.employe_id.is_some() {
        return Err(Erreur::regle("RG-CMD-07", "Consommation employé : utilisez « Imputer à l'employé »"));
    }
    let t = totaux(op, &e.commande_id)?;
    if e.parts.is_empty() {
        return Err(Erreur::validation("Aucun moyen de paiement"));
    }
    let montant: i64 = e.parts.iter().map(|p| p.montant).sum();
    if e.parts.iter().any(|p| p.montant <= 0) {
        return Err(Erreur::regle("RG-CAI-02", "Chaque part doit être positive"));
    }
    if montant > t.reste {
        return Err(Erreur::regle(
            "RG-CAI-02",
            format!("Le paiement ({montant}) dépasse le reste à payer ({})", t.reste),
        ));
    }
    let especes: i64 = e.parts.iter().filter(|p| p.moyen == "especes").map(|p| p.montant).sum();
    // Sans part en espèces, rien n'est reçu ni rendu (évite une monnaie rendue fictive).
    let recu = if especes > 0 { e.especes_recues.unwrap_or(especes) } else { 0 };
    if recu < especes {
        return Err(Erreur::validation("Espèces reçues insuffisantes"));
    }
    let rendu = recu - especes;
    let paiement_id = op.nouvel_id();
    let numero = op.sequence("recu")?;
    op.execute(
        "INSERT INTO paiements(id, numero, commande_id, journee_id, session_id, montant, recu, rendu, horodatage, utilisateur_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![paiement_id, numero, e.commande_id, journee.id, session.id, montant, recu, rendu, op.maintenant, op.utilisateur()],
    )?;
    let libelle = format!("Commande n°{}", etat.numero);
    for p in &e.parts {
        let part_id = op.nouvel_id();
        let (compte, client) = match p.moyen.as_str() {
            "especes" if p.par_livreur => (Some(compte_livreur(op, &e.commande_id)?), None),
            "especes" => (Some(session.compte_id.clone()), None),
            "mobile_money" => {
                let c = p.compte_id.clone().ok_or_else(|| Erreur::validation("Choisissez l'opérateur Mobile Money"))?;
                if type_compte(op, &c)? != "mobile_money" {
                    return Err(Erreur::validation("Ce compte n'est pas un compte Mobile Money"));
                }
                let reference = p.reference.as_deref().unwrap_or("").trim();
                if op.params.reference_mm_obligatoire && reference.is_empty() {
                    return Err(Erreur::regle("RG-CAI-04", "Référence de transaction Mobile Money obligatoire"));
                }
                if !reference.is_empty() {
                    let deja: i64 = op.query_row(
                        "SELECT COUNT(*) FROM parts_paiement WHERE compte_id = ?1 AND reference = ?2 AND montant > 0",
                        params![c, reference],
                        |r| r.get(0),
                    )?;
                    if deja > 0 {
                        return Err(Erreur::regle(
                            "RG-CAI-05",
                            format!("La référence {reference} a déjà été utilisée : possible fraude"),
                        ));
                    }
                }
                (Some(c), None)
            }
            "virement" | "carte" => {
                let c = p.compte_id.clone().ok_or_else(|| Erreur::validation("Choisissez le compte bancaire"))?;
                type_compte(op, &c)?;
                (Some(c), None)
            }
            "credit" => {
                let client = p
                    .client_id
                    .clone()
                    .or_else(|| {
                        op.query_row("SELECT client_id FROM commandes WHERE id = ?1", params![e.commande_id], |r| r.get(0))
                            .ok()
                            .flatten()
                    })
                    .ok_or_else(|| Erreur::validation("Choisissez le client pour une vente à crédit"))?;
                (None, Some(client))
            }
            _ => return Err(Erreur::validation(format!("Moyen de paiement inconnu : {}", p.moyen))),
        };
        let reference = p.reference.as_deref().map(str::trim).filter(|r| !r.is_empty());
        op.execute(
            "INSERT INTO parts_paiement(id, paiement_id, moyen, compte_id, client_id, montant, reference, numero_payeur)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![part_id, paiement_id, p.moyen, compte, client, p.montant, reference, p.numero_payeur],
        )?;
        if let Some(c) = &compte {
            mouvement(op, c, Some(&session.id), "vente", p.montant, Some(("paiement", &paiement_id)), &libelle, None)?;
        }
        if p.moyen == "mobile_money" {
            op.execute(
                "INSERT INTO verifications_mm(id, part_id, statut, horodatage, utilisateur_id) VALUES (?1, ?2, 'a_verifier', ?3, ?4)",
                params![op.nouvel_id(), part_id, op.maintenant, op.utilisateur()],
            )?;
        }
        if let Some(client) = client {
            crate::clients::vente_credit(op, &client, &e.commande_id, &paiement_id, p.montant)?;
        }
    }
    let reste = t.reste - montant;
    let payee = reste == 0;
    if payee {
        op.execute(
            "UPDATE commandes SET statut = 'payee', payee_le = ?1, modifie_le = ?1, version = version + 1 WHERE id = ?2",
            params![op.maintenant, e.commande_id],
        )?;
        // RG-CMD-11 : payer d'abord → l'envoi part dès que tout est payé.
        if etat.ordre_paiement == "avant" {
            commandes::envoyer_op(op, &e.commande_id)?;
        }
    }
    op.outbox("paiement", &paiement_id, "creer")?;
    op.evenement("paiement", Some(&e.commande_id));
    op.evenement("table", None);
    Ok(ResultatEncaissement { paiement_id, numero, montant, especes_recues: recu, rendu, reste, commande_payee: payee })
}

fn compte_livreur(op: &Op, commande_id: &str) -> Resultat<String> {
    let livreur: Option<String> =
        op.query_row("SELECT livreur_id FROM commandes WHERE id = ?1", params![commande_id], |r| r.get(0))?;
    let livreur = livreur.ok_or_else(|| Erreur::regle("RG-LIV-02", "Assignez d'abord un livreur à la commande"))?;
    crate::livraison::compte_du_livreur(op, &livreur)
}

/// RG-CAI-07 : contre-passation d'un paiement.
pub fn annuler_paiement(db: &mut Db, acteur: &Acteur, paiement_id: &str, motif: &str) -> Resultat<String> {
    db.executer(acteur, |op| {
        if motif.trim().is_empty() {
            return Err(Erreur::regle("RG-CAI-07", "Motif obligatoire"));
        }
        let autorise_par = op.exiger(perm::CAISSE_ANNULER_PAIEMENT)?;
        let (commande_id, montant, deja): (Option<String>, i64, i64) = trouver(
            op.query_row(
                "SELECT commande_id, montant, (SELECT COUNT(*) FROM paiements a WHERE a.annule_paiement_id = p.id)
                 FROM paiements p WHERE id = ?1 AND annule_paiement_id IS NULL",
                params![paiement_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            ),
            "Paiement",
        )?;
        if deja > 0 {
            return Err(Erreur::validation("Ce paiement est déjà annulé"));
        }
        let session = session_requise(op)?;
        let journee = op.journee_ouverte()?;
        let id = op.nouvel_id();
        let numero = op.sequence("recu")?;
        op.execute(
            "INSERT INTO paiements(id, numero, commande_id, journee_id, session_id, montant, annule_paiement_id, motif,
                horodatage, utilisateur_id, autorise_par)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![id, numero, commande_id, journee.id, session.id, -montant, paiement_id, motif.trim(), op.maintenant, op.utilisateur(), autorise_par],
        )?;
        let parts: Vec<(String, Option<String>, Option<String>, i64, Option<String>)> = op
            .prepare("SELECT moyen, compte_id, client_id, montant, reference FROM parts_paiement WHERE paiement_id = ?1")?
            .query_map(params![paiement_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)))?
            .collect::<Result<_, _>>()?;
        for (moyen, compte, client, m, reference) in parts {
            op.execute(
                "INSERT INTO parts_paiement(id, paiement_id, moyen, compte_id, client_id, montant, reference)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![op.nouvel_id(), id, moyen, compte, client, -m, reference],
            )?;
            if let Some(c) = compte {
                // Les espèces sortent de la caisse de la session courante.
                let c = if moyen == "especes" && type_compte(op, &c)? == "especes" { session.compte_id.clone() } else { c };
                mouvement(op, &c, Some(&session.id), "annulation_vente", -m, Some(("paiement", &id)), motif.trim(), autorise_par.as_deref())?;
            }
            if let Some(cl) = client {
                crate::clients::contrepasser_credit(op, &cl, commande_id.as_deref(), &id, m, motif.trim())?;
            }
        }
        if let Some(cid) = &commande_id {
            op.execute(
                "UPDATE commandes SET statut = 'ouverte', payee_le = NULL, modifie_le = ?1, version = version + 1
                 WHERE id = ?2 AND statut IN ('payee','cloturee')",
                params![op.maintenant, cid],
            )?;
            op.evenement("commande", Some(cid));
        }
        op.audit(
            "caisse.annuler_paiement",
            "paiement",
            Some(paiement_id),
            Some(json!({ "montant": montant })),
            Some(json!({ "contrepassation": id })),
            Some(motif.trim()),
            autorise_par.as_deref(),
        )?;
        op.evenement("paiement", None);
        Ok(id)
    })
}

// ───────────── Mouvements divers, transferts, dépenses ─────────────

#[derive(Debug, Deserialize)]
pub struct MouvementCaisse {
    /// entree_diverse | apport | retrait | retrait_proprietaire
    #[serde(rename = "type")]
    pub type_: String,
    pub montant: i64,
    #[serde(default)]
    pub libelle: String,
}

/// RG-CAI-10 / RG-CAI-11.
pub fn mouvement_caisse(db: &mut Db, acteur: &Acteur, m: &MouvementCaisse) -> Resultat<String> {
    db.executer(acteur, |op| {
        let session = session_requise(op)?;
        if m.montant <= 0 {
            return Err(Erreur::validation("Montant invalide"));
        }
        let (signe, autorise_par) = match m.type_.as_str() {
            "entree_diverse" | "apport" => (1, op.exiger(perm::CAISSE_MOUVEMENT)?),
            "retrait" => (-1, op.exiger(perm::CAISSE_MOUVEMENT)?),
            "retrait_proprietaire" => (-1, op.exiger(perm::CAISSE_RETRAIT_PROPRIETAIRE)?),
            _ => return Err(Erreur::validation("Type de mouvement inconnu")),
        };
        if m.type_ != "retrait_proprietaire" && m.libelle.trim().is_empty() {
            return Err(Erreur::validation("Indiquez le motif du mouvement"));
        }
        let id = mouvement(op, &session.compte_id, Some(&session.id), &m.type_, signe * m.montant, None, m.libelle.trim(), autorise_par.as_deref())?;
        // Le retrait propriétaire alimente le compte coffre s'il existe (traçabilité).
        if m.type_ == "retrait_proprietaire" {
            if let Some(coffre) = op
                .query_row("SELECT id FROM comptes_tresorerie WHERE type = 'coffre' AND actif = 1 LIMIT 1", [], |r| r.get::<_, String>(0))
                .optional()?
            {
                mouvement(op, &coffre, None, "retrait_proprietaire", m.montant, Some(("mouvement_tresorerie", &id)), m.libelle.trim(), autorise_par.as_deref())?;
            }
        }
        op.audit("caisse.mouvement", "mouvement_tresorerie", Some(&id), None, Some(json!({ "type": m.type_, "montant": m.montant })), Some(m.libelle.trim()), autorise_par.as_deref())?;
        op.evenement("caisse", Some(&session.id));
        Ok(id)
    })
}

#[derive(Debug, Deserialize)]
pub struct Transfert {
    pub de: String,
    pub vers: String,
    pub montant: i64,
    #[serde(default)]
    pub frais: i64,
    #[serde(default)]
    pub libelle: String,
}

/// RG-CAI-12.
pub fn transferer(db: &mut Db, acteur: &Acteur, t: &Transfert) -> Resultat<()> {
    db.executer(acteur, |op| {
        let autorise_par = op.exiger(perm::CAISSE_MOUVEMENT)?;
        if t.montant <= 0 || t.frais < 0 || t.de == t.vers {
            return Err(Erreur::validation("Transfert invalide"));
        }
        let session = uid_session(op)?;
        let sortie = mouvement(op, &t.de, session.as_deref(), "transfert_sortant", -t.montant, None, t.libelle.trim(), autorise_par.as_deref())?;
        mouvement(op, &t.vers, session.as_deref(), "transfert_entrant", t.montant - t.frais, Some(("mouvement_tresorerie", &sortie)), t.libelle.trim(), autorise_par.as_deref())?;
        if t.frais > 0 {
            let cat: String = op.query_row(
                "SELECT id FROM categories_depense WHERE nom LIKE 'Autres%' ORDER BY nom LIMIT 1",
                [],
                |r| r.get(0),
            )?;
            // Les frais sont prélevés sur le montant transféré : dépense sans mouvement supplémentaire.
            op.execute(
                "INSERT INTO depenses(id, journee_id, categorie_id, compte_id, mouvement_id, montant, beneficiaire, libelle, horodatage, utilisateur_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'Opérateur', 'Frais de transfert', ?7, ?8)",
                params![op.nouvel_id(), op.journee_ouverte()?.id, cat, t.de, sortie, t.frais, op.maintenant, op.utilisateur()],
            )?;
        }
        op.audit("caisse.transfert", "compte_tresorerie", Some(&t.de), None, Some(json!({ "vers": t.vers, "montant": t.montant, "frais": t.frais })), None, autorise_par.as_deref())?;
        op.evenement("caisse", None);
        Ok(())
    })
}

fn uid_session(op: &Op) -> Resultat<Option<String>> {
    Ok(match op.utilisateur() {
        Some(u) => session_utilisateur(op, u)?.map(|s| s.id),
        None => None,
    })
}

#[derive(Debug, Deserialize)]
pub struct NouvelleDepense {
    pub categorie_id: String,
    pub montant: i64,
    /// Par défaut : la caisse de la session de l'utilisateur.
    #[serde(default)]
    pub compte_id: Option<String>,
    #[serde(default)]
    pub beneficiaire: String,
    #[serde(default)]
    pub libelle: String,
    #[serde(default)]
    pub justificatif: String,
}

/// RG-CAI-13.
pub fn depenser(db: &mut Db, acteur: &Acteur, d: &NouvelleDepense) -> Resultat<String> {
    db.executer(acteur, |op| depenser_op(op, d))
}

pub(crate) fn depenser_op(op: &mut Op, d: &NouvelleDepense) -> Resultat<String> {
    let autorise_par = op.exiger(perm::DEPENSE_CREER)?;
    let journee = op.journee_ouverte()?;
    if d.montant <= 0 {
        return Err(Erreur::validation("Montant invalide"));
    }
    let session = uid_session(op)?;
    let compte = match &d.compte_id {
        Some(c) => c.clone(),
        None => match &session {
            Some(s) => self::session(op, s)?.compte_id,
            None => return Err(Erreur::regle("RG-CAI-01", "Ouvrez une session de caisse ou choisissez un compte")),
        },
    };
    let libelle = if d.libelle.trim().is_empty() { d.beneficiaire.trim() } else { d.libelle.trim() };
    let id = op.nouvel_id();
    let mvt = mouvement(op, &compte, session.as_deref(), "depense", -d.montant, Some(("depense", &id)), libelle, autorise_par.as_deref())?;
    op.execute(
        "INSERT INTO depenses(id, journee_id, categorie_id, compte_id, mouvement_id, montant, beneficiaire, libelle,
            justificatif, horodatage, utilisateur_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![id, journee.id, d.categorie_id, compte, mvt, d.montant, d.beneficiaire.trim(), d.libelle.trim(), d.justificatif, op.maintenant, op.utilisateur()],
    )?;
    op.audit("depense.creer", "depense", Some(&id), None, Some(json!({ "montant": d.montant, "libelle": libelle })), None, autorise_par.as_deref())?;
    op.outbox("depense", &id, "creer")?;
    op.evenement("caisse", None);
    Ok(id)
}

/// Contre-passation d'une dépense saisie par erreur.
pub fn annuler_depense(db: &mut Db, acteur: &Acteur, depense_id: &str, motif: &str) -> Resultat<()> {
    db.executer(acteur, |op| {
        let autorise_par = op.exiger(perm::CAISSE_ANNULER_PAIEMENT)?;
        if motif.trim().is_empty() {
            return Err(Erreur::validation("Motif obligatoire"));
        }
        let (cat, compte, montant, deja): (String, String, i64, i64) = trouver(
            op.query_row(
                "SELECT categorie_id, compte_id, montant, (SELECT COUNT(*) FROM depenses a WHERE a.annule_depense_id = d.id)
                 FROM depenses d WHERE id = ?1 AND annule_depense_id IS NULL AND montant > 0",
                params![depense_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            ),
            "Dépense",
        )?;
        if deja > 0 {
            return Err(Erreur::validation("Dépense déjà annulée"));
        }
        let journee = op.journee_ouverte()?;
        let session = uid_session(op)?;
        let mvt = mouvement(op, &compte, session.as_deref(), "annulation_depense", montant, Some(("depense", depense_id)), motif.trim(), autorise_par.as_deref())?;
        // Montant de contre-passation : la contrainte CHECK (montant > 0) impose une ligne de même signe,
        // le lien annule_depense_id indique qu'elle neutralise l'originale.
        op.execute(
            "INSERT INTO depenses(id, journee_id, categorie_id, compte_id, mouvement_id, montant, libelle, annule_depense_id,
                horodatage, utilisateur_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![op.nouvel_id(), journee.id, cat, compte, mvt, montant, motif.trim(), depense_id, op.maintenant, op.utilisateur()],
        )?;
        op.audit("depense.annuler", "depense", Some(depense_id), None, None, Some(motif.trim()), autorise_par.as_deref())?;
        op.evenement("caisse", None);
        Ok(())
    })
}

#[derive(Debug, Serialize)]
pub struct DepenseLue {
    pub id: String,
    pub categorie: String,
    pub montant: i64,
    pub beneficiaire: String,
    pub libelle: String,
    pub compte: String,
    pub horodatage: i64,
    pub annulee: bool,
    pub est_annulation: bool,
}

pub fn depenses_journee(conn: &Connection, journee_id: &str) -> Resultat<Vec<DepenseLue>> {
    let mut s = conn.prepare(
        "SELECT d.id, c.nom, d.montant, d.beneficiaire, d.libelle, t.nom, d.horodatage,
                EXISTS(SELECT 1 FROM depenses a WHERE a.annule_depense_id = d.id), d.annule_depense_id IS NOT NULL
         FROM depenses d JOIN categories_depense c ON c.id = d.categorie_id JOIN comptes_tresorerie t ON t.id = d.compte_id
         WHERE d.journee_id = ?1 ORDER BY d.horodatage DESC",
    )?;
    let v = s
        .query_map(params![journee_id], |r| {
            Ok(DepenseLue {
                id: r.get(0)?,
                categorie: r.get(1)?,
                montant: r.get(2)?,
                beneficiaire: r.get(3)?,
                libelle: r.get(4)?,
                compte: r.get(5)?,
                horodatage: r.get(6)?,
                annulee: r.get(7)?,
                est_annulation: r.get(8)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(v)
}

pub fn categories_depense(conn: &Connection) -> Resultat<Vec<(String, String)>> {
    let mut s = conn.prepare("SELECT id, nom FROM categories_depense WHERE actif = 1 ORDER BY nom")?;
    let v = s.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?.collect::<Result<Vec<_>, _>>()?;
    Ok(v)
}

pub fn ajouter_categorie_depense(db: &mut Db, acteur: &Acteur, nom: &str) -> Resultat<String> {
    db.executer(acteur, |op| {
        op.exiger(perm::PARAMETRE_GERER)?;
        non_vide(nom, "catégorie")?;
        let id = op.nouvel_id();
        op.execute("INSERT INTO categories_depense(id, nom) VALUES (?1, ?2)", params![id, nom.trim()])?;
        Ok(id)
    })
}

// ───────────── Mobile Money ─────────────

#[derive(Debug, Serialize)]
pub struct PartMobileMoney {
    pub part_id: String,
    pub compte: String,
    pub montant: i64,
    pub reference: Option<String>,
    pub numero_payeur: Option<String>,
    pub horodatage: i64,
    pub commande_numero: Option<i64>,
    pub statut: String,
    pub caissier: Option<String>,
}

/// Paiements Mobile Money, filtrés par statut de vérification (RG-CAI-04).
pub fn parts_mobile_money(conn: &Connection, statut: Option<&str>) -> Resultat<Vec<PartMobileMoney>> {
    let mut s = conn.prepare(
        "SELECT pp.id, c.nom, pp.montant, pp.reference, pp.numero_payeur, p.horodatage, cm.numero,
                (SELECT v.statut FROM verifications_mm v WHERE v.part_id = pp.id ORDER BY v.horodatage DESC, v.rowid DESC LIMIT 1),
                u.nom
         FROM parts_paiement pp JOIN paiements p ON p.id = pp.paiement_id
         JOIN comptes_tresorerie c ON c.id = pp.compte_id
         LEFT JOIN commandes cm ON cm.id = p.commande_id
         LEFT JOIN utilisateurs u ON u.id = p.utilisateur_id
         WHERE pp.moyen = 'mobile_money' AND pp.montant > 0
         ORDER BY p.horodatage DESC LIMIT 500",
    )?;
    let v = s
        .query_map([], |r| {
            Ok(PartMobileMoney {
                part_id: r.get(0)?,
                compte: r.get(1)?,
                montant: r.get(2)?,
                reference: r.get(3)?,
                numero_payeur: r.get(4)?,
                horodatage: r.get(5)?,
                commande_numero: r.get(6)?,
                statut: r.get::<_, Option<String>>(7)?.unwrap_or_else(|| "a_verifier".into()),
                caissier: r.get(8)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(v.into_iter().filter(|p| statut.is_none_or(|s| p.statut == s)).collect())
}

pub fn verifier_mobile_money(db: &mut Db, acteur: &Acteur, part_id: &str, statut: &str, note: &str) -> Resultat<()> {
    db.executer(acteur, |op| {
        let autorise_par = op.exiger(perm::CAISSE_VERIFIER_MM)?;
        if !["verifie", "rejete", "a_verifier"].contains(&statut) {
            return Err(Erreur::validation("Statut inconnu"));
        }
        op.execute(
            "INSERT INTO verifications_mm(id, part_id, statut, note, horodatage, utilisateur_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![op.nouvel_id(), part_id, statut, note.trim(), op.maintenant, op.utilisateur()],
        )?;
        op.audit("mobile_money.verifier", "part_paiement", Some(part_id), None, Some(json!({ "statut": statut })), Some(note.trim()).filter(|n| !n.is_empty()), autorise_par.as_deref())?;
        Ok(())
    })
}

#[derive(Debug, Serialize)]
pub struct MouvementLu {
    pub id: String,
    pub compte: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub montant: i64,
    pub libelle: String,
    pub horodatage: i64,
    pub utilisateur: Option<String>,
}

pub fn mouvements(conn: &Connection, session_id: Option<&str>, compte_id: Option<&str>, limite: i64) -> Resultat<Vec<MouvementLu>> {
    let mut s = conn.prepare(
        "SELECT m.id, c.nom, m.type, m.montant, m.libelle, m.horodatage, u.nom FROM mouvements_tresorerie m
         JOIN comptes_tresorerie c ON c.id = m.compte_id LEFT JOIN utilisateurs u ON u.id = m.utilisateur_id
         WHERE (?1 IS NULL OR m.session_id = ?1) AND (?2 IS NULL OR m.compte_id = ?2)
         ORDER BY m.horodatage DESC, m.rowid DESC LIMIT ?3",
    )?;
    let v = s
        .query_map(params![session_id, compte_id, limite], |r| {
            Ok(MouvementLu {
                id: r.get(0)?,
                compte: r.get(1)?,
                type_: r.get(2)?,
                montant: r.get(3)?,
                libelle: r.get(4)?,
                horodatage: r.get(5)?,
                utilisateur: r.get(6)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(v)
}

/// Paiements d'une commande (pour l'annulation).
#[derive(Debug, Serialize)]
pub struct PaiementLu {
    pub id: String,
    pub numero: i64,
    pub montant: i64,
    /// Espèces tendues par le client (0 sans espèces).
    pub recu: i64,
    pub rendu: i64,
    pub horodatage: i64,
    pub annule: bool,
    pub est_annulation: bool,
    pub parts: Vec<(String, i64, Option<String>)>,
}

pub fn paiements_commande(conn: &Connection, commande_id: &str) -> Resultat<Vec<PaiementLu>> {
    let mut s = conn.prepare(
        "SELECT p.id, p.numero, p.montant, p.rendu, p.horodatage,
                EXISTS(SELECT 1 FROM paiements a WHERE a.annule_paiement_id = p.id), p.annule_paiement_id IS NOT NULL, p.recu
         FROM paiements p WHERE p.commande_id = ?1 ORDER BY p.horodatage",
    )?;
    let base = s
        .query_map(params![commande_id], |r| {
            Ok(PaiementLu {
                id: r.get(0)?,
                numero: r.get(1)?,
                montant: r.get(2)?,
                rendu: r.get(3)?,
                horodatage: r.get(4)?,
                annule: r.get(5)?,
                est_annulation: r.get(6)?,
                recu: r.get(7)?,
                parts: vec![],
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let mut v = Vec::new();
    for mut p in base {
        let mut s = conn.prepare_cached(
            "SELECT pp.moyen, pp.montant, c.nom FROM parts_paiement pp LEFT JOIN comptes_tresorerie c ON c.id = pp.compte_id
             WHERE pp.paiement_id = ?1",
        )?;
        p.parts = s.query_map(params![p.id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?.collect::<Result<_, _>>()?;
        v.push(p);
    }
    Ok(v)
}
