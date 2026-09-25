use std::collections::BTreeMap;

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::db::{trouver, Acteur, Db, Op};
use crate::erreur::{Erreur, Resultat};
use crate::parametres::arrondir;
use crate::permissions as perm;
use crate::{catalogue, impression, salle, stock};

#[derive(Debug, Deserialize)]
pub struct NouvelleCommande {
    /// sur_place | comptoir | emporter | livraison
    #[serde(rename = "type")]
    pub type_: String,
    #[serde(default)]
    pub table_id: Option<String>,
    #[serde(default)]
    pub client_id: Option<String>,
    /// Consommation personnelle d'un employé (RG-CMD-07).
    #[serde(default)]
    pub employe_id: Option<String>,
    #[serde(default)]
    pub couverts: i64,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub livraison: Option<InfosLivraison>,
    /// serveur (menu papier, défaut) | telephone. Les canaux QR et en ligne passent par `entrantes`.
    #[serde(default)]
    pub canal: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct InfosLivraison {
    pub quartier: String,
    #[serde(default)]
    pub repere: String,
    pub telephone: String,
    #[serde(default)]
    pub frais: Option<i64>,
    /// Position GPS facultative (microdegrés).
    #[serde(default)]
    pub lat: Option<i64>,
    #[serde(default)]
    pub lon: Option<i64>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LigneSaisie {
    pub produit_id: String,
    #[serde(default = "un")]
    pub quantite: i64,
    #[serde(default)]
    pub options: Vec<String>,
    #[serde(default)]
    pub commentaire: String,
}

fn un() -> i64 {
    1
}

// ───────────── Lecture ─────────────

#[derive(Debug, Serialize, Clone)]
pub struct Ligne {
    pub id: String,
    pub produit_id: String,
    pub libelle: String,
    pub quantite: i64,
    pub quantite_annulee: i64,
    pub prix_unitaire: i64,
    pub options: serde_json::Value,
    pub montant_options: i64,
    pub commentaire: String,
    pub poste_id: Option<String>,
    pub envoi_id: Option<String>,
    pub statut: String,
    pub offert: bool,
    pub offert_motif: Option<String>,
    pub montant: i64,
}

#[derive(Debug, Serialize, Clone)]
pub struct Envoi {
    pub id: String,
    pub numero: i64,
    pub poste_id: Option<String>,
    pub poste_nom: Option<String>,
    pub statut: String,
    pub message: String,
    pub cree_le: i64,
}

#[derive(Debug, Serialize, Clone)]
pub struct RemiseLue {
    pub id: String,
    pub ligne_id: Option<String>,
    pub montant: i64,
    pub motif: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct Commande {
    pub id: String,
    pub numero: i64,
    pub journee_id: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub ordre_paiement: String,
    pub table_id: Option<String>,
    pub table_nom: Option<String>,
    pub zone_id: Option<String>,
    pub serveur_id: Option<String>,
    pub serveur_nom: Option<String>,
    pub client_id: Option<String>,
    pub client_nom: Option<String>,
    pub employe_id: Option<String>,
    pub couverts: i64,
    pub note: String,
    pub statut: String,
    pub livraison_statut: Option<String>,
    pub livraison_quartier: Option<String>,
    pub livraison_repere: Option<String>,
    pub livraison_telephone: Option<String>,
    pub livraison_frais: i64,
    pub livreur_id: Option<String>,
    pub cree_le: i64,
    pub lignes: Vec<Ligne>,
    pub envois: Vec<Envoi>,
    pub remises: Vec<RemiseLue>,
    pub totaux: Totaux,
}

#[derive(Debug, Serialize, Clone, Default, PartialEq, Eq)]
pub struct Totaux {
    /// Σ lignes non offertes, quantité effective.
    pub brut: i64,
    pub remises: i64,
    pub offerts: i64,
    pub frais_livraison: i64,
    /// RG-CMD-10.
    pub total: i64,
    pub paye: i64,
    pub reste: i64,
}

fn montant_ligne(quantite: i64, annulee: i64, prix: i64, options: i64) -> i64 {
    (quantite - annulee) * (prix + options)
}

pub fn totaux(conn: &Connection, commande_id: &str) -> Resultat<Totaux> {
    let (brut, offerts): (i64, i64) = conn.query_row(
        "SELECT COALESCE(SUM(CASE WHEN offert = 0 THEN (quantite - quantite_annulee) * (prix_unitaire + montant_options) END), 0),
                COALESCE(SUM(CASE WHEN offert = 1 THEN (quantite - quantite_annulee) * (prix_unitaire + montant_options) END), 0)
         FROM lignes_commande WHERE commande_id = ?1",
        params![commande_id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    let remises: i64 = conn.query_row(
        "SELECT COALESCE(SUM(montant), 0) FROM remises WHERE commande_id = ?1",
        params![commande_id],
        |r| r.get(0),
    )?;
    let frais: i64 =
        conn.query_row("SELECT livraison_frais FROM commandes WHERE id = ?1", params![commande_id], |r| r.get(0))?;
    let paye: i64 = conn.query_row(
        "SELECT COALESCE(SUM(montant), 0) FROM paiements WHERE commande_id = ?1",
        params![commande_id],
        |r| r.get(0),
    )?;
    let total = (brut - remises).max(0) + frais;
    Ok(Totaux { brut, remises, offerts, frais_livraison: frais, total, paye, reste: total - paye })
}

pub fn detail(conn: &Connection, id: &str) -> Resultat<Commande> {
    let mut c = trouver(
        conn.query_row(
            "SELECT c.id, c.numero, c.journee_id, c.type, c.ordre_paiement, c.table_id, t.nom, c.zone_id, c.serveur_id,
                    u.nom, c.client_id, cl.nom, c.employe_id, c.couverts, c.note, c.statut, c.livraison_statut,
                    c.livraison_quartier, c.livraison_repere, c.livraison_telephone, c.livraison_frais, c.livreur_id, c.cree_le
             FROM commandes c
             LEFT JOIN tables_salle t ON t.id = c.table_id
             LEFT JOIN utilisateurs u ON u.id = c.serveur_id
             LEFT JOIN clients cl ON cl.id = c.client_id
             WHERE c.id = ?1",
            params![id],
            |r| {
                Ok(Commande {
                    id: r.get(0)?,
                    numero: r.get(1)?,
                    journee_id: r.get(2)?,
                    type_: r.get(3)?,
                    ordre_paiement: r.get(4)?,
                    table_id: r.get(5)?,
                    table_nom: r.get(6)?,
                    zone_id: r.get(7)?,
                    serveur_id: r.get(8)?,
                    serveur_nom: r.get(9)?,
                    client_id: r.get(10)?,
                    client_nom: r.get(11)?,
                    employe_id: r.get(12)?,
                    couverts: r.get(13)?,
                    note: r.get(14)?,
                    statut: r.get(15)?,
                    livraison_statut: r.get(16)?,
                    livraison_quartier: r.get(17)?,
                    livraison_repere: r.get(18)?,
                    livraison_telephone: r.get(19)?,
                    livraison_frais: r.get(20)?,
                    livreur_id: r.get(21)?,
                    cree_le: r.get(22)?,
                    lignes: vec![],
                    envois: vec![],
                    remises: vec![],
                    totaux: Totaux::default(),
                })
            },
        ),
        "Commande",
    )?;
    let mut s = conn.prepare_cached(
        "SELECT id, produit_id, libelle, quantite, quantite_annulee, prix_unitaire, options_json, montant_options,
                commentaire, poste_id, envoi_id, statut, offert, offert_motif
         FROM lignes_commande WHERE commande_id = ?1 ORDER BY cree_le, rowid",
    )?;
    c.lignes = s
        .query_map(params![id], |r| {
            let q: i64 = r.get(3)?;
            let qa: i64 = r.get(4)?;
            let pu: i64 = r.get(5)?;
            let mo: i64 = r.get(7)?;
            let opts: String = r.get(6)?;
            Ok(Ligne {
                id: r.get(0)?,
                produit_id: r.get(1)?,
                libelle: r.get(2)?,
                quantite: q,
                quantite_annulee: qa,
                prix_unitaire: pu,
                options: serde_json::from_str(&opts).unwrap_or(json!([])),
                montant_options: mo,
                commentaire: r.get(8)?,
                poste_id: r.get(9)?,
                envoi_id: r.get(10)?,
                statut: r.get(11)?,
                offert: r.get(12)?,
                offert_motif: r.get(13)?,
                montant: montant_ligne(q, qa, pu, mo),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let mut s = conn.prepare_cached(
        "SELECT e.id, e.numero, e.poste_id, p.nom, e.statut, e.message, e.cree_le
         FROM envois e LEFT JOIN postes_preparation p ON p.id = e.poste_id
         WHERE e.commande_id = ?1 ORDER BY e.numero, p.nom",
    )?;
    c.envois = s
        .query_map(params![id], |r| {
            Ok(Envoi {
                id: r.get(0)?,
                numero: r.get(1)?,
                poste_id: r.get(2)?,
                poste_nom: r.get(3)?,
                statut: r.get(4)?,
                message: r.get(5)?,
                cree_le: r.get(6)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let mut s = conn.prepare_cached(
        "SELECT id, ligne_id, montant, motif FROM remises WHERE commande_id = ?1 ORDER BY horodatage",
    )?;
    c.remises = s
        .query_map(params![id], |r| Ok(RemiseLue { id: r.get(0)?, ligne_id: r.get(1)?, montant: r.get(2)?, motif: r.get(3)? }))?
        .collect::<Result<Vec<_>, _>>()?;
    c.totaux = totaux(conn, id)?;
    Ok(c)
}

#[derive(Debug, Serialize)]
pub struct CommandeResume {
    pub id: String,
    pub numero: i64,
    #[serde(rename = "type")]
    pub type_: String,
    pub table_nom: Option<String>,
    pub serveur_nom: Option<String>,
    pub client_nom: Option<String>,
    pub statut: String,
    pub livraison_statut: Option<String>,
    pub cree_le: i64,
    pub total: i64,
    pub reste: i64,
}

/// Commandes de la journée ouverte (filtrées par statut si fourni).
pub fn lister(conn: &Connection, journee_id: &str, statut: Option<&str>) -> Resultat<Vec<CommandeResume>> {
    let mut s = conn.prepare(
        "SELECT c.id, c.numero, c.type, t.nom, u.nom, cl.nom, c.statut, c.livraison_statut, c.cree_le
         FROM commandes c
         LEFT JOIN tables_salle t ON t.id = c.table_id
         LEFT JOIN utilisateurs u ON u.id = c.serveur_id
         LEFT JOIN clients cl ON cl.id = c.client_id
         WHERE c.journee_id = ?1 AND (?2 IS NULL OR c.statut = ?2) AND COALESCE(c.validation, '') NOT IN ('en_attente','refusee')
         ORDER BY c.numero DESC",
    )?;
    let lignes = s
        .query_map(params![journee_id, statut], |r| {
            Ok(CommandeResume {
                id: r.get(0)?,
                numero: r.get(1)?,
                type_: r.get(2)?,
                table_nom: r.get(3)?,
                serveur_nom: r.get(4)?,
                client_nom: r.get(5)?,
                statut: r.get(6)?,
                livraison_statut: r.get(7)?,
                cree_le: r.get(8)?,
                total: 0,
                reste: 0,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let mut v = Vec::with_capacity(lignes.len());
    for mut c in lignes {
        let t = totaux(conn, &c.id)?;
        c.total = t.total;
        c.reste = t.reste;
        v.push(c);
    }
    Ok(v)
}

// ───────────── Écriture ─────────────

pub(crate) struct EtatCommande {
    pub statut: String,
    pub ordre_paiement: String,
    pub zone_id: Option<String>,
    pub serveur_id: Option<String>,
    pub employe_id: Option<String>,
    pub numero: i64,
    pub table_nom: Option<String>,
    pub type_: String,
}

pub(crate) fn etat(conn: &Connection, id: &str) -> Resultat<EtatCommande> {
    trouver(
        conn.query_row(
            "SELECT c.statut, c.ordre_paiement, c.zone_id, c.serveur_id, c.employe_id, c.numero, t.nom, c.type
             FROM commandes c LEFT JOIN tables_salle t ON t.id = c.table_id WHERE c.id = ?1",
            params![id],
            |r| {
                Ok(EtatCommande {
                    statut: r.get(0)?,
                    ordre_paiement: r.get(1)?,
                    zone_id: r.get(2)?,
                    serveur_id: r.get(3)?,
                    employe_id: r.get(4)?,
                    numero: r.get(5)?,
                    table_nom: r.get(6)?,
                    type_: r.get(7)?,
                })
            },
        ),
        "Commande",
    )
}

/// RG-CMD-08 : seule une commande ouverte se modifie.
fn exiger_ouverte(e: &EtatCommande) -> Resultat<()> {
    if e.statut != "ouverte" {
        return Err(Erreur::regle(
            "RG-CMD-08",
            "Cette addition est déjà encaissée ou fermée. Annulez le paiement pour la corriger.",
        ));
    }
    Ok(())
}

/// Un serveur ne touche que ses propres additions, sauf permission.
fn exiger_proprietaire(op: &Op, e: &EtatCommande) -> Resultat<()> {
    if e.serveur_id.is_some() && e.serveur_id.as_deref() != op.utilisateur() && !op.a_permission(perm::CAISSE_ENCAISSER) {
        op.exiger(perm::COMMANDE_MODIFIER_AUTRES)?;
    }
    Ok(())
}

/// Ouvre une addition. RG-CMD-01 : si la table a déjà une addition ouverte, on la renvoie (nouvelle tournée).
pub fn ouvrir(db: &mut Db, acteur: &Acteur, n: &NouvelleCommande) -> Resultat<String> {
    db.executer(acteur, |op| ouvrir_op(op, n))
}

pub(crate) fn ouvrir_op(op: &mut Op, n: &NouvelleCommande) -> Resultat<String> {
    op.exiger(perm::COMMANDE_CREER)?;
    let journee = op.journee_ouverte()?;
    // RG-CAN-01 : le menu papier est toujours ouvert ; le téléphone est un canal activable.
    let canal = n.canal.as_deref().unwrap_or("serveur");
    match canal {
        "serveur" => {}
        "telephone" if op.params.canaux.telephone => {}
        "telephone" => return Err(Erreur::regle("RG-CAN-01", "Les commandes par téléphone sont désactivées")),
        _ => return Err(Erreur::validation("Canal réservé aux commandes reçues (QR, en ligne)")),
    }
    let mut ordre = match n.type_.as_str() {
        // Consommation employé : pas d'encaissement, imputation à la fin (RG-CMD-07).
        _ if n.employe_id.is_some() => "apres",
        "sur_place" => "apres",
        "comptoir" if op.params.paiement_avant_comptoir => "avant",
        "emporter" if op.params.paiement_avant_emporter => "avant",
        "comptoir" | "emporter" | "livraison" => "apres",
        _ => return Err(Erreur::validation("Type de commande inconnu")),
    };
    let mut zone_id = None;
    if let Some(t) = &n.table_id {
        if let Some(existante) = op
            .query_row(
                "SELECT id FROM commandes WHERE table_id = ?1 AND statut = 'ouverte'",
                params![t],
                |r| r.get::<_, String>(0),
            )
            .optional()?
        {
            return Ok(existante);
        }
        zone_id = Some(salle::zone_de_table(op, t)?);
        op.execute("UPDATE tables_salle SET reservee = 0, a_nettoyer = 0 WHERE id = ?1", params![t])?;
    } else if n.type_ == "sur_place" {
        return Err(Erreur::validation("Choisissez une table"));
    }
    if n.employe_id.is_some() {
        op.exiger(perm::COMMANDE_CONSO_EMPLOYE)?;
    }
    let (statut_liv, quartier, repere, tel, frais) = match (&n.type_[..], &n.livraison) {
        ("livraison", Some(l)) => {
            if l.telephone.trim().is_empty() || l.quartier.trim().is_empty() {
                return Err(Erreur::regle("RG-LIV-01", "Quartier et téléphone obligatoires pour une livraison"));
            }
            let frais = l.frais.unwrap_or_else(|| {
                op.params.quartiers.iter().find(|q| q.nom == l.quartier).map(|q| q.frais).unwrap_or(0)
            });
            (Some("nouvelle"), Some(l.quartier.clone()), Some(l.repere.clone()), Some(l.telephone.clone()), frais)
        }
        ("livraison", None) => return Err(Erreur::regle("RG-LIV-01", "Adresse de livraison manquante")),
        _ => (None, None, None, None, 0),
    };
    let position = n.livraison.as_ref().and_then(|l| l.lat.zip(l.lon));
    // RG-CAN-03 : numéro en liste noire → accord d'un responsable.
    if let Some(t) = &tel {
        if let Some(motif) = crate::zones_risque::est_bloque(op, t)? {
            let par = op.exiger(perm::ZONE_OUTREPASSER).map_err(|e| match e {
                Erreur::AutorisationRequise(p) => Erreur::AutorisationRequise(format!("{p} — numéro bloqué : {motif}")),
                e => e,
            })?;
            op.audit("numero.outrepasser", "commande", None, None, Some(json!({ "telephone": t })), Some(&motif), par.as_deref())?;
        }
    }
    // RG-ZON-03 : zones à risque pour les commandes saisies par le personnel.
    if n.type_ == "livraison" {
        if let Some(d) = crate::zones_risque::evaluer(op, quartier.as_deref(), position, op.maintenant)? {
            match d.action.as_str() {
                "paiement_avance" => ordre = "avant",
                _ => {
                    let par = op.exiger(perm::ZONE_OUTREPASSER).map_err(|e| match e {
                        Erreur::AutorisationRequise(p) => Erreur::AutorisationRequise(format!("{p} — {}", d.message)),
                        e => e,
                    })?;
                    op.audit("zone.outrepasser", "commande", None, None, Some(json!(d)), None, par.as_deref())?;
                }
            }
        }
    }
    let id = op.nouvel_id();
    let numero = op.sequence("commande")?;
    op.execute(
        "INSERT INTO commandes(id, numero, journee_id, type, ordre_paiement, table_id, zone_id, serveur_id, client_id,
            employe_id, couverts, note, statut, livraison_statut, livraison_quartier, livraison_repere,
            livraison_telephone, livraison_frais, cree_le, cree_par, modifie_le, canal, livraison_lat, livraison_lon, client_telephone)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, 'ouverte', ?13, ?14, ?15, ?16, ?17, ?18, ?8, ?18, ?19, ?20, ?21, ?16)",
        params![
            id, numero, journee.id, n.type_, ordre, n.table_id, zone_id, op.utilisateur(), n.client_id, n.employe_id,
            n.couverts, n.note, statut_liv, quartier, repere, tel, frais, op.maintenant, canal,
            position.map(|p| p.0), position.map(|p| p.1)
        ],
    )?;
    op.outbox("commande", &id, "creer")?;
    op.evenement("commande", Some(&id));
    Ok(id)
}

/// Ajoute des articles en brouillon (nouvelle tournée si déjà des envois).
pub fn ajouter_lignes(db: &mut Db, acteur: &Acteur, commande_id: &str, lignes: &[LigneSaisie]) -> Resultat<Vec<String>> {
    db.executer(acteur, |op| ajouter_lignes_op(op, commande_id, lignes))
}

pub(crate) fn ajouter_lignes_op(op: &mut Op, commande_id: &str, lignes: &[LigneSaisie]) -> Resultat<Vec<String>> {
    op.exiger(perm::COMMANDE_CREER)?;
    let e = etat(op, commande_id)?;
    exiger_ouverte(&e)?;
    exiger_proprietaire(op, &e)?;
    let mut ids = Vec::new();
    for l in lignes {
        if l.quantite <= 0 || l.quantite > 999 {
            return Err(Erreur::validation("Quantité invalide"));
        }
        let p = catalogue::produit(op, &l.produit_id)?;
        if !p.actif || !p.disponible {
            return Err(Erreur::regle("RG-CAT-05", format!("« {} » est indisponible aujourd'hui", p.nom)));
        }
        // RG-CAT-04 : options.
        let mut choisies = Vec::new();
        let mut supplement = 0;
        for g in &p.groupes_options {
            let dans_groupe: Vec<_> = g.options.iter().filter(|o| l.options.contains(&o.id)).collect();
            let n = dans_groupe.len() as i64;
            if n < g.min_choix || n > g.max_choix {
                return Err(Erreur::regle(
                    "RG-CAT-04",
                    format!("« {} » : choisissez entre {} et {} option(s) pour {}", p.nom, g.min_choix, g.max_choix, g.nom),
                ));
            }
            for o in dans_groupe {
                supplement += o.supplement;
                choisies.push(json!({ "id": o.id, "nom": o.nom, "supplement": o.supplement }));
            }
        }
        let connues: usize = p.groupes_options.iter().map(|g| g.options.iter().filter(|o| l.options.contains(&o.id)).count()).sum();
        if connues != l.options.len() {
            return Err(Erreur::validation("Option inconnue pour ce produit"));
        }
        // RG-CAT-01/03 : prix copié, grille de la zone ; RG-PRO-02 : promotion en cours (happy hour).
        let moment = crate::promotions::prix_du_moment(op, &p.id, e.zone_id.as_deref(), op.maintenant)?;
        let prix = moment.prix;
        let libelle = if p.nom_court.is_empty() { p.nom.clone() } else { p.nom_court.clone() };
        let id = op.nouvel_id();
        op.execute(
            "INSERT INTO lignes_commande(id, commande_id, produit_id, libelle, quantite, prix_unitaire, options_json,
                montant_options, commentaire, poste_id, statut, cree_le, cree_par, promotion_id, prix_normal)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 'brouillon', ?11, ?12, ?13, ?14)",
            params![
                id,
                commande_id,
                p.id,
                libelle,
                l.quantite,
                prix,
                serde_json::Value::Array(choisies).to_string(),
                supplement,
                l.commentaire.trim(),
                p.poste_id,
                op.maintenant,
                op.utilisateur(),
                moment.promotion_id,
                moment.promotion_id.is_some().then_some(moment.prix_normal)
            ],
        )?;
        ids.push(id);
    }
    toucher(op, commande_id)?;
    Ok(ids)
}

fn toucher(op: &mut Op, commande_id: &str) -> Resultat<()> {
    op.execute(
        "UPDATE commandes SET modifie_le = ?1, version = version + 1 WHERE id = ?2",
        params![op.maintenant, commande_id],
    )?;
    op.outbox("commande", commande_id, "modifier")?;
    op.evenement("commande", Some(commande_id));
    Ok(())
}

/// RG-CMD-02 : modification libre d'une ligne non envoyée.
pub fn modifier_brouillon(db: &mut Db, acteur: &Acteur, ligne_id: &str, quantite: i64, commentaire: Option<&str>) -> Resultat<()> {
    db.executer(acteur, |op| {
        let (cid, statut): (String, String) = trouver(
            op.query_row("SELECT commande_id, statut FROM lignes_commande WHERE id = ?1", params![ligne_id], |r| {
                Ok((r.get(0)?, r.get(1)?))
            }),
            "Ligne",
        )?;
        let e = etat(op, &cid)?;
        exiger_ouverte(&e)?;
        exiger_proprietaire(op, &e)?;
        if statut != "brouillon" {
            return Err(Erreur::regle("RG-CMD-04", "Article déjà envoyé : utilisez l'annulation avec motif"));
        }
        if quantite <= 0 {
            op.execute("DELETE FROM lignes_commande WHERE id = ?1", params![ligne_id])?;
        } else {
            op.execute("UPDATE lignes_commande SET quantite = ?1 WHERE id = ?2", params![quantite, ligne_id])?;
        }
        if let Some(c) = commentaire {
            op.execute("UPDATE lignes_commande SET commentaire = ?1 WHERE id = ?2", params![c.trim(), ligne_id])?;
        }
        toucher(op, &cid)
    })
}

/// RG-CMD-03 : répartit les lignes brouillon par poste, un envoi (et un ticket) par poste.
pub fn envoyer(db: &mut Db, acteur: &Acteur, commande_id: &str) -> Resultat<Vec<String>> {
    db.executer(acteur, |op| envoyer_op(op, commande_id))
}

pub(crate) fn envoyer_op(op: &mut Op, commande_id: &str) -> Resultat<Vec<String>> {
    op.exiger(perm::COMMANDE_CREER)?;
    let e = etat(op, commande_id)?;
    let avant_paye = e.ordre_paiement == "avant" && e.statut == "payee";
    if !avant_paye {
        exiger_ouverte(&e)?;
    }
    if e.ordre_paiement == "avant" && e.statut == "ouverte" {
        // RG-CMD-11 : payer d'abord. Le brouillon peut dépasser ce qui est payé → on vérifie le reste dû.
        let t = totaux(op, commande_id)?;
        if t.reste > 0 {
            return Err(Erreur::regle("RG-CMD-11", "Commande à payer d'abord : encaissez avant l'envoi"));
        }
    }
    let lignes: Vec<(String, Option<String>, String, i64, String, String)> = op
        .prepare(
            "SELECT id, poste_id, libelle, quantite, commentaire, options_json FROM lignes_commande
             WHERE commande_id = ?1 AND statut = 'brouillon' ORDER BY cree_le, rowid",
        )?
        .query_map(params![commande_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)))?
        .collect::<Result<_, _>>()?;
    if lignes.is_empty() {
        return Ok(vec![]);
    }
    let numero: i64 = op.query_row(
        "SELECT COALESCE(MAX(numero), 0) + 1 FROM envois WHERE commande_id = ?1",
        params![commande_id],
        |r| r.get(0),
    )?;
    let mut par_poste: BTreeMap<Option<String>, Vec<_>> = BTreeMap::new();
    for l in lignes {
        par_poste.entry(l.1.clone()).or_default().push(l);
    }
    let serveur: Option<String> = match op.utilisateur() {
        Some(uid) => op
            .query_row("SELECT nom FROM utilisateurs WHERE id = ?1", params![uid], |r| r.get(0))
            .optional()?,
        None => None,
    };
    let mut envois = Vec::new();
    for (poste, lignes) in par_poste {
        let eid = op.nouvel_id();
        op.execute(
            "INSERT INTO envois(id, commande_id, numero, poste_id, statut, cree_le, cree_par, modifie_le)
             VALUES (?1, ?2, ?3, ?4, 'recu', ?5, ?6, ?5)",
            params![eid, commande_id, numero, poste, op.maintenant, op.utilisateur()],
        )?;
        for l in &lignes {
            op.execute(
                "UPDATE lignes_commande SET envoi_id = ?1, statut = 'envoyee' WHERE id = ?2",
                params![eid, l.0],
            )?;
            stock::sortie_vente(op, &l.0)?;
        }
        if let Some(p) = &poste {
            let ticket = impression::TicketEnvoi {
                titre: titre_commande(&e),
                numero_envoi: numero,
                heure: op.maintenant,
                serveur: serveur.clone().unwrap_or_default(),
                lignes: lignes
                    .iter()
                    .map(|l| impression::LigneTicket {
                        quantite: l.3,
                        libelle: l.2.clone(),
                        options: options_texte(&l.5),
                        commentaire: l.4.clone(),
                    })
                    .collect(),
                annulation: false,
            };
            impression::mettre_en_file(op, Some(p), "envoi", Some(&eid), &impression::rendu_envoi(&ticket, op.params.largeur_ticket, op.params.fuseau_minutes))?;
        }
        envois.push(eid);
    }
    toucher(op, commande_id)?;
    op.evenement("envoi", Some(commande_id));
    Ok(envois)
}

fn options_texte(json: &str) -> Vec<String> {
    serde_json::from_str::<Vec<serde_json::Value>>(json)
        .unwrap_or_default()
        .iter()
        .filter_map(|o| o.get("nom").and_then(|n| n.as_str()).map(str::to_owned))
        .collect()
}

pub(crate) fn titre_commande(e: &EtatCommande) -> String {
    match (&e.table_nom, e.type_.as_str()) {
        (Some(t), _) => format!("TABLE {t}"),
        (None, "comptoir") => format!("COMPTOIR n°{}", e.numero),
        (None, "emporter") => format!("À EMPORTER n°{}", e.numero),
        (None, "livraison") => format!("LIVRAISON n°{}", e.numero),
        _ => format!("COMMANDE n°{}", e.numero),
    }
}

/// RG-CMD-04 / RG-STK-02 : annulation d'articles.
pub fn annuler_ligne(
    db: &mut Db,
    acteur: &Acteur,
    ligne_id: &str,
    quantite: i64,
    motif: &str,
    perte: bool,
) -> Resultat<()> {
    db.executer(acteur, |op| {
        let (cid, statut, q, qa, pu, mo, libelle, poste, commentaire): (
            String,
            String,
            i64,
            i64,
            i64,
            i64,
            String,
            Option<String>,
            String,
        ) = trouver(
            op.query_row(
                "SELECT commande_id, statut, quantite, quantite_annulee, prix_unitaire, montant_options, libelle, poste_id, commentaire
                 FROM lignes_commande WHERE id = ?1",
                params![ligne_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?, r.get(8)?)),
            ),
            "Ligne",
        )?;
        let e = etat(op, &cid)?;
        exiger_ouverte(&e)?;
        exiger_proprietaire(op, &e)?;
        if quantite <= 0 || quantite > q - qa {
            return Err(Erreur::validation("Quantité à annuler invalide"));
        }
        if statut == "brouillon" {
            // RG-CMD-02 : libre.
            if quantite == q {
                op.execute("DELETE FROM lignes_commande WHERE id = ?1", params![ligne_id])?;
            } else {
                op.execute("UPDATE lignes_commande SET quantite = quantite - ?1 WHERE id = ?2", params![quantite, ligne_id])?;
            }
            return toucher(op, &cid);
        }
        if motif.trim().is_empty() {
            return Err(Erreur::regle("RG-CMD-04", "Motif obligatoire pour annuler un article envoyé"));
        }
        let autorise_par = op.exiger(perm::COMMANDE_ANNULER_ENVOYE)?;
        op.execute(
            "UPDATE lignes_commande SET quantite_annulee = quantite_annulee + ?1 WHERE id = ?2",
            params![quantite, ligne_id],
        )?;
        let montant = quantite * (pu + mo);
        op.execute(
            "INSERT INTO annulations(id, commande_id, ligne_id, quantite, montant, apres_envoi, perte, motif, horodatage,
                utilisateur_id, autorise_par)
             VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6, ?7, ?8, ?9, ?10)",
            params![op.nouvel_id(), cid, ligne_id, quantite, montant, perte, motif.trim(), op.maintenant, op.utilisateur(), autorise_par],
        )?;
        stock::retour_annulation(op, ligne_id, quantite, perte)?;
        op.audit(
            "commande.annuler_article",
            "ligne_commande",
            Some(ligne_id),
            Some(json!({ "quantite": q - qa })),
            Some(json!({ "quantite": q - qa - quantite, "libelle": libelle, "montant": montant })),
            Some(motif.trim()),
            autorise_par.as_deref(),
        )?;
        if let Some(p) = &poste {
            let ticket = impression::TicketEnvoi {
                titre: titre_commande(&e),
                numero_envoi: 0,
                heure: op.maintenant,
                serveur: String::new(),
                lignes: vec![impression::LigneTicket {
                    quantite,
                    libelle,
                    options: vec![],
                    commentaire: format!("{} {}", motif.trim(), commentaire).trim().to_string(),
                }],
                annulation: true,
            };
            impression::mettre_en_file(op, Some(p), "annulation", Some(ligne_id), &impression::rendu_envoi(&ticket, op.params.largeur_ticket, op.params.fuseau_minutes))?;
        }
        toucher(op, &cid)
    })
}

/// RG-CMD-05. `montant` en FCFA, ou `pourcentage` (0–100) du montant concerné.
pub fn remise(
    db: &mut Db,
    acteur: &Acteur,
    commande_id: &str,
    ligne_id: Option<&str>,
    montant: Option<i64>,
    pourcentage: Option<i64>,
    motif: &str,
) -> Resultat<()> {
    db.executer(acteur, |op| {
        let e = etat(op, commande_id)?;
        exiger_ouverte(&e)?;
        if motif.trim().is_empty() {
            return Err(Erreur::regle("RG-CMD-05", "Motif obligatoire pour une remise"));
        }
        let autorise_perm = op.exiger(perm::COMMANDE_REMISE)?;
        let t = totaux(op, commande_id)?;
        let base = match ligne_id {
            Some(l) => {
                let (q, qa, pu, mo, offert): (i64, i64, i64, i64, bool) = trouver(
                    op.query_row(
                        "SELECT quantite, quantite_annulee, prix_unitaire, montant_options, offert FROM lignes_commande
                         WHERE id = ?1 AND commande_id = ?2",
                        params![l, commande_id],
                        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
                    ),
                    "Ligne",
                )?;
                if offert {
                    return Err(Erreur::validation("Article déjà offert"));
                }
                montant_ligne(q, qa, pu, mo)
            }
            None => t.brut - t.remises,
        };
        let m = match (montant, pourcentage) {
            (Some(m), _) => m,
            (None, Some(p)) if (0..=100).contains(&p) => arrondir(base * p / 100, op.params.arrondi),
            _ => return Err(Erreur::validation("Indiquez un montant ou un pourcentage")),
        };
        if m <= 0 || m > base {
            return Err(Erreur::validation("Montant de remise invalide"));
        }
        // Plafond : pourcentage de la base, par rapport au rôle (ou à l'autorisateur).
        let (plafond, par) = op.plafond_remise();
        let pct_x100 = m * 10_000 / base.max(1);
        if pct_x100 > plafond * 100 {
            return Err(Erreur::AutorisationRequise(format!("{} (plafond {plafond} %)", perm::COMMANDE_REMISE)));
        }
        let autorise_par = autorise_perm.or(par);
        let id = op.nouvel_id();
        op.execute(
            "INSERT INTO remises(id, commande_id, ligne_id, montant, motif, horodatage, utilisateur_id, autorise_par)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![id, commande_id, ligne_id, m, motif.trim(), op.maintenant, op.utilisateur(), autorise_par],
        )?;
        op.audit(
            "commande.remise",
            "commande",
            Some(commande_id),
            None,
            Some(json!({ "montant": m, "ligne_id": ligne_id })),
            Some(motif.trim()),
            autorise_par.as_deref(),
        )?;
        toucher(op, commande_id)
    })
}

/// RG-CMD-06.
pub fn offrir(db: &mut Db, acteur: &Acteur, ligne_id: &str, motif: &str) -> Resultat<()> {
    db.executer(acteur, |op| {
        let cid: String = trouver(
            op.query_row("SELECT commande_id FROM lignes_commande WHERE id = ?1", params![ligne_id], |r| r.get(0)),
            "Ligne",
        )?;
        let e = etat(op, &cid)?;
        exiger_ouverte(&e)?;
        if motif.trim().is_empty() {
            return Err(Erreur::regle("RG-CMD-06", "Motif obligatoire pour un article offert"));
        }
        let autorise_par = op.exiger(perm::COMMANDE_OFFRIR)?;
        let remises: i64 = op.query_row("SELECT COUNT(*) FROM remises WHERE ligne_id = ?1", params![ligne_id], |r| r.get(0))?;
        if remises > 0 {
            return Err(Erreur::validation("Cet article a déjà une remise"));
        }
        op.execute(
            "UPDATE lignes_commande SET offert = 1, offert_motif = ?1 WHERE id = ?2",
            params![motif.trim(), ligne_id],
        )?;
        op.audit("commande.offrir", "ligne_commande", Some(ligne_id), None, None, Some(motif.trim()), autorise_par.as_deref())?;
        toucher(op, &cid)
    })
}

/// RG-CMD-09 : transfert vers une table libre.
pub fn transferer(db: &mut Db, acteur: &Acteur, commande_id: &str, table_cible: &str) -> Resultat<()> {
    db.executer(acteur, |op| {
        let e = etat(op, commande_id)?;
        exiger_ouverte(&e)?;
        let autorise_par = op.exiger(perm::COMMANDE_TRANSFERER)?;
        let occupee: i64 = op.query_row(
            "SELECT COUNT(*) FROM commandes WHERE table_id = ?1 AND statut = 'ouverte'",
            params![table_cible],
            |r| r.get(0),
        )?;
        if occupee > 0 {
            return Err(Erreur::regle("RG-CMD-09", "La table cible est occupée : utilisez la fusion"));
        }
        let zone = salle::zone_de_table(op, table_cible)?;
        op.execute(
            "UPDATE commandes SET table_id = ?1, zone_id = ?2, type = 'sur_place' WHERE id = ?3",
            params![table_cible, zone, commande_id],
        )?;
        op.audit("commande.transferer", "commande", Some(commande_id), None, Some(json!({ "table": table_cible })), None, autorise_par.as_deref())?;
        op.evenement("table", None);
        toucher(op, commande_id)
    })
}

/// RG-CMD-09 : fusion de la commande source dans la cible.
pub fn fusionner(db: &mut Db, acteur: &Acteur, source_id: &str, cible_id: &str) -> Resultat<()> {
    db.executer(acteur, |op| {
        if source_id == cible_id {
            return Err(Erreur::validation("Choisissez deux additions différentes"));
        }
        let s = etat(op, source_id)?;
        let c = etat(op, cible_id)?;
        exiger_ouverte(&s)?;
        exiger_ouverte(&c)?;
        let autorise_par = op.exiger(perm::COMMANDE_TRANSFERER)?;
        let paye: i64 = op.query_row(
            "SELECT COALESCE(SUM(montant), 0) FROM paiements WHERE commande_id = ?1",
            params![source_id],
            |r| r.get(0),
        )?;
        if paye != 0 {
            return Err(Erreur::regle("RG-CMD-09", "L'addition source a déjà un paiement partiel"));
        }
        op.execute("UPDATE lignes_commande SET commande_id = ?1 WHERE commande_id = ?2", params![cible_id, source_id])?;
        op.execute("UPDATE envois SET commande_id = ?1 WHERE commande_id = ?2", params![cible_id, source_id])?;
        let remises: Vec<(i64, String, Option<String>)> = op
            .prepare("SELECT montant, motif, ligne_id FROM remises WHERE commande_id = ?1")?
            .query_map(params![source_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
            .collect::<Result<_, _>>()?;
        for (m, motif, ligne) in remises {
            // Remises en ajout seul : on les contre-passe sur la source et on les recrée sur la cible.
            op.execute(
                "INSERT INTO remises(id, commande_id, ligne_id, montant, motif, horodatage, utilisateur_id)
                 VALUES (?1, ?2, NULL, ?3, 'Fusion', ?4, ?5)",
                params![op.nouvel_id(), source_id, -m, op.maintenant, op.utilisateur()],
            )?;
            op.execute(
                "INSERT INTO remises(id, commande_id, ligne_id, montant, motif, horodatage, utilisateur_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![op.nouvel_id(), cible_id, ligne, m, motif, op.maintenant, op.utilisateur()],
            )?;
        }
        op.execute(
            "UPDATE commandes SET statut = 'annulee', cloturee_le = ?1, note = note || ' [fusionnée]' WHERE id = ?2",
            params![op.maintenant, source_id],
        )?;
        op.audit("commande.fusionner", "commande", Some(source_id), None, Some(json!({ "vers": cible_id })), None, autorise_par.as_deref())?;
        op.evenement("table", None);
        toucher(op, cible_id)
    })
}

/// Abandon d'une addition vide (aucun article envoyé, aucun paiement).
pub fn abandonner(db: &mut Db, acteur: &Acteur, commande_id: &str) -> Resultat<()> {
    db.executer(acteur, |op| {
        let e = etat(op, commande_id)?;
        exiger_ouverte(&e)?;
        exiger_proprietaire(op, &e)?;
        let envoyees: i64 = op.query_row(
            "SELECT COUNT(*) FROM lignes_commande WHERE commande_id = ?1 AND statut <> 'brouillon' AND quantite > quantite_annulee",
            params![commande_id],
            |r| r.get(0),
        )?;
        let paye: i64 = op.query_row(
            "SELECT COUNT(*) FROM paiements WHERE commande_id = ?1",
            params![commande_id],
            |r| r.get(0),
        )?;
        if envoyees > 0 || paye > 0 {
            return Err(Erreur::regle("RG-CMD-04", "Des articles ont été envoyés : annulez-les avec motif d'abord"));
        }
        op.execute("DELETE FROM lignes_commande WHERE commande_id = ?1 AND statut = 'brouillon'", params![commande_id])?;
        op.execute(
            "UPDATE commandes SET statut = 'annulee', cloturee_le = ?1 WHERE id = ?2",
            params![op.maintenant, commande_id],
        )?;
        op.audit("commande.abandonner", "commande", Some(commande_id), None, None, None, None)?;
        op.evenement("table", None);
        toucher(op, commande_id)
    })
}

// ───────────── Cuisine ─────────────

#[derive(Debug, Serialize)]
pub struct EnvoiCuisine {
    pub id: String,
    pub commande_id: String,
    pub titre: String,
    pub numero: i64,
    pub poste_id: Option<String>,
    pub statut: String,
    pub message: String,
    pub cree_le: i64,
    pub serveur: Option<String>,
    pub lignes: Vec<Ligne>,
}

/// Envois en cours pour l'écran cuisine / bar.
pub fn envois_actifs(conn: &Connection, poste_id: Option<&str>) -> Resultat<Vec<EnvoiCuisine>> {
    let mut s = conn.prepare(
        "SELECT e.id, e.commande_id, e.numero, e.poste_id, e.statut, e.message, e.cree_le, u.nom
         FROM envois e JOIN commandes c ON c.id = e.commande_id
         LEFT JOIN utilisateurs u ON u.id = e.cree_par
         WHERE e.statut IN ('recu','en_preparation','pret','probleme') AND (?1 IS NULL OR e.poste_id = ?1)
           AND e.poste_id IS NOT NULL
         ORDER BY e.cree_le",
    )?;
    let base = s
        .query_map(params![poste_id], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, Option<String>>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, String>(5)?,
                r.get::<_, i64>(6)?,
                r.get::<_, Option<String>>(7)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let mut v = Vec::new();
    for (id, cid, numero, poste, statut, message, cree_le, serveur) in base {
        let e = etat(conn, &cid)?;
        let lignes = detail(conn, &cid)?.lignes.into_iter().filter(|l| l.envoi_id.as_deref() == Some(&id)).collect();
        v.push(EnvoiCuisine { id, commande_id: cid, titre: titre_commande(&e), numero, poste_id: poste, statut, message, cree_le, serveur, lignes });
    }
    Ok(v)
}

/// Statut d'un envoi côté cuisine : recu → en_preparation → pret → servi ; probleme (message au serveur).
pub fn statut_envoi(db: &mut Db, acteur: &Acteur, envoi_id: &str, statut: &str, message: &str) -> Resultat<()> {
    db.executer(acteur, |op| {
        op.exiger(perm::CUISINE_VOIR)?;
        let statut_ligne = match statut {
            "en_preparation" => "en_preparation",
            "pret" => "prete",
            "servi" => "servie",
            "recu" => "envoyee",
            "probleme" => "",
            _ => return Err(Erreur::validation("Statut d'envoi inconnu")),
        };
        let cid: String = trouver(
            op.query_row("SELECT commande_id FROM envois WHERE id = ?1", params![envoi_id], |r| r.get(0)),
            "Envoi",
        )?;
        op.execute(
            "UPDATE envois SET statut = ?1, message = ?2, modifie_le = ?3 WHERE id = ?4",
            params![statut, message, op.maintenant, envoi_id],
        )?;
        if !statut_ligne.is_empty() {
            op.execute(
                "UPDATE lignes_commande SET statut = ?1 WHERE envoi_id = ?2 AND statut <> 'brouillon'",
                params![statut_ligne, envoi_id],
            )?;
        }
        op.evenement(if statut == "pret" { "envoi_pret" } else { "envoi" }, Some(envoi_id));
        op.evenement("commande", Some(&cid));
        Ok(())
    })
}

// ───────────── Division d'addition ─────────────

/// RG-CMD-12 : parts égales arrondies ; l'écart d'arrondi va sur la dernière part.
pub fn diviser_parts_egales(total: i64, parts: i64, arrondi: i64) -> Vec<i64> {
    if parts <= 0 || total <= 0 {
        return vec![];
    }
    let part = arrondir(total / parts, arrondi);
    let mut v = vec![part; (parts - 1) as usize];
    v.push(total - part * (parts - 1));
    v
}

/// Montant des lignes sélectionnées (division par articles), remises de ligne déduites.
pub fn montant_lignes(conn: &Connection, commande_id: &str, lignes: &[String]) -> Resultat<i64> {
    let c = detail(conn, commande_id)?;
    let mut total = 0;
    for l in c.lignes.iter().filter(|l| lignes.contains(&l.id) && !l.offert) {
        let r: i64 = c.remises.iter().filter(|r| r.ligne_id.as_deref() == Some(&l.id)).map(|r| r.montant).sum();
        total += l.montant - r;
    }
    Ok(total)
}

/// Associe (ou retire) un client à l'addition : crédit, historique, livraison.
pub fn definir_client(db: &mut Db, acteur: &Acteur, commande_id: &str, client_id: Option<&str>) -> Resultat<()> {
    db.executer(acteur, |op| {
        op.exiger(perm::COMMANDE_CREER)?;
        let e = etat(op, commande_id)?;
        exiger_ouverte(&e)?;
        if let Some(c) = client_id {
            crate::clients::client(op, c)?;
        }
        op.execute("UPDATE commandes SET client_id = ?1 WHERE id = ?2", params![client_id, commande_id])?;
        toucher(op, commande_id)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parts_egales_ecart_sur_derniere() {
        assert_eq!(diviser_parts_egales(12_500, 3, 25), vec![4_175, 4_175, 4_150]);
        assert_eq!(diviser_parts_egales(10_000, 4, 25), vec![2_500; 4]);
        assert_eq!(diviser_parts_egales(1_000, 3, 1), vec![333, 333, 334]);
        assert_eq!(diviser_parts_egales(12_500, 3, 25).iter().sum::<i64>(), 12_500);
    }
}
