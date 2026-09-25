//! Commandes reçues sans passer par le personnel : QR sur la table, commandes en ligne (fiche 0013).
//! Elles arrivent dans une file ; un membre du personnel les accepte avant tout envoi en cuisine.
//! Les prix sont toujours recalculés ici : on ne fait jamais confiance au prix envoyé par le client.

use rand::Rng;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::catalogue::{self, GroupeOptions};
use crate::commandes::{self, InfosLivraison, LigneSaisie};
use crate::db::{trouver, Acteur, Db, Op};
use crate::erreur::{Erreur, Resultat};
use crate::permissions as perm;
use crate::zones_risque::{self, normaliser_telephone};

const ALPHABET: &[u8] = b"ABCDEFGHJKMNPQRSTUVWXYZ23456789";

/// Code produit par `code_aleatoire` (alphabet sans caractères ambigus).
pub fn code_valide(c: &str, n: usize) -> bool {
    c.len() == n && c.bytes().all(|b| ALPHABET.contains(&b))
}

pub fn code_aleatoire(n: usize) -> String {
    let mut r = rand::thread_rng();
    (0..n).map(|_| ALPHABET[r.gen_range(0..ALPHABET.len())] as char).collect()
}

// ───────────── Menu public ─────────────

#[derive(Debug, Serialize)]
pub struct ProduitPublic {
    pub id: String,
    pub categorie_id: String,
    pub nom: String,
    pub description: String,
    pub photo: String,
    pub prix: i64,
    pub groupes_options: Vec<GroupeOptions>,
}

#[derive(Debug, Serialize)]
pub struct CategoriePublique {
    pub id: String,
    pub nom: String,
    pub icone: String,
    pub couleur: String,
}

#[derive(Debug, Serialize)]
pub struct MenuPublic {
    pub restaurant: String,
    pub telephone: String,
    /// Journée ouverte : les commandes sont possibles.
    pub ouvert: bool,
    pub table: Option<String>,
    pub qr_table: bool,
    pub en_ligne: bool,
    pub paiement_avance: bool,
    pub paiement_a_la_livraison: bool,
    pub verification_numero: String,
    pub operateurs: Vec<String>,
    pub quartiers: Vec<crate::parametres::QuartierLivraison>,
    pub categories: Vec<CategoriePublique>,
    pub produits: Vec<ProduitPublic>,
}

/// Menu consultable sans connexion (QR sur la table, page en ligne). Prix de la zone de la table.
/// `ms` : instant de consultation (prix du happy hour en cours).
pub fn menu_public(conn: &Connection, code_table: Option<&str>, ms: i64) -> Resultat<MenuPublic> {
    let p = crate::parametres::lire(conn)?;
    let r = crate::parametres::restaurant(conn)?;
    let table = match code_table {
        Some(c) => Some(table_par_code(conn, c)?),
        None => None,
    };
    let categories = catalogue::lister_categories(conn)?
        .into_iter()
        .filter(|c| c.actif)
        .map(|c| CategoriePublique { id: c.id, nom: c.nom, icone: c.icone, couleur: c.couleur })
        .collect();
    let mut produits = Vec::new();
    for pr in catalogue::lister_produits(conn, true)?.into_iter().filter(|p| p.disponible) {
        // Happy hour en cours : le client voit le prix qu'il paiera.
        let prix = crate::promotions::prix_du_moment(conn, &pr.id, table.as_ref().map(|t| t.2.as_str()), ms)?.prix;
        produits.push(ProduitPublic {
            id: pr.id,
            categorie_id: pr.categorie_id,
            nom: pr.nom,
            description: pr.description,
            photo: pr.photo,
            prix,
            groupes_options: pr.groupes_options,
        });
    }
    let operateurs = crate::caisse::lister_comptes(conn)?
        .into_iter()
        .filter(|c| c.actif && c.type_ == "mobile_money")
        .map(|c| c.nom)
        .collect();
    Ok(MenuPublic {
        restaurant: r.nom,
        telephone: r.telephone,
        ouvert: crate::journee::ouverte(conn)?.is_some(),
        table: table.map(|t| t.1),
        qr_table: p.canaux.qr_table,
        en_ligne: p.canaux.en_ligne,
        paiement_avance: p.canaux.paiement_avance,
        paiement_a_la_livraison: p.canaux.paiement_a_la_livraison,
        verification_numero: p.canaux.verification_numero.clone(),
        operateurs,
        quartiers: p.quartiers.clone(),
        categories,
        produits,
    })
}

/// (id, nom, zone_id) de la table dont le QR porte ce code.
fn table_par_code(conn: &Connection, code: &str) -> Resultat<(String, String, String)> {
    conn.query_row(
        "SELECT id, nom, zone_id FROM tables_salle WHERE code_qr = ?1 AND actif = 1",
        params![code.trim().to_uppercase()],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )
    .optional()?
    .ok_or_else(|| Erreur::regle("RG-CAN-02", "QR code inconnu : demandez au serveur"))
}

/// Attribue un code QR aux tables qui n'en ont pas (à imprimer et coller sur la table).
pub fn generer_codes_qr(db: &mut Db, acteur: &Acteur, regenerer: bool) -> Resultat<usize> {
    db.executer(acteur, |op| {
        op.exiger(perm::SALLE_GERER)?;
        let ids: Vec<String> = op
            .prepare(if regenerer { "SELECT id FROM tables_salle" } else { "SELECT id FROM tables_salle WHERE code_qr IS NULL" })?
            .query_map([], |r| r.get(0))?
            .collect::<Result<_, _>>()?;
        for id in &ids {
            op.execute("UPDATE tables_salle SET code_qr = ?1 WHERE id = ?2", params![code_aleatoire(6), id])?;
        }
        op.audit("table.codes_qr", "table", None, None, Some(json!({ "nombre": ids.len(), "regenerer": regenerer })), None, None)?;
        Ok(ids.len())
    })
}

pub fn codes_qr(conn: &Connection) -> Resultat<Vec<(String, String, Option<String>)>> {
    let mut s = conn.prepare(
        "SELECT t.id, z.nom || ' — ' || t.nom, t.code_qr FROM tables_salle t JOIN zones z ON z.id = t.zone_id
         WHERE t.actif = 1 ORDER BY z.ordre, length(t.nom), t.nom",
    )?;
    let v = s.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?.collect::<Result<Vec<_>, _>>()?;
    Ok(v)
}

// ───────────── Réception ─────────────

#[derive(Debug, Deserialize, Clone)]
pub struct CommandeEntrante {
    /// Identifiant attribué par le relais : une commande renvoyée deux fois n'est créée qu'une fois.
    #[serde(default)]
    pub origine_id: Option<String>,
    /// Code de suivi déjà communiqué au client par le relais (sinon attribué ici).
    #[serde(default)]
    pub code_suivi: Option<String>,
    /// qr_table | en_ligne
    pub canal: String,
    #[serde(default)]
    pub code_table: Option<String>,
    /// En ligne : livraison | emporter.
    #[serde(rename = "type", default)]
    pub type_: Option<String>,
    #[serde(default)]
    pub client_nom: String,
    #[serde(default)]
    pub telephone: String,
    /// Numéro confirmé par code SMS (relais).
    #[serde(default)]
    pub telephone_verifie: bool,
    #[serde(default)]
    pub livraison: Option<InfosLivraison>,
    /// avance | a_la_livraison | sur_place
    #[serde(default)]
    pub paiement_mode: Option<String>,
    #[serde(default)]
    pub paiement_operateur: Option<String>,
    #[serde(default)]
    pub paiement_reference: Option<String>,
    pub lignes: Vec<LigneSaisie>,
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct Reponse {
    /// en_attente | refusee
    pub statut: String,
    pub message: String,
    pub numero: Option<i64>,
    pub code_suivi: Option<String>,
    pub total: i64,
}

fn refus(message: impl Into<String>) -> Reponse {
    Reponse { statut: "refusee".into(), message: message.into(), numero: None, code_suivi: None, total: 0 }
}

/// Reçoit une commande QR ou en ligne (RG-CAN-01 à 05, RG-ZON-02). Les refus ne créent rien mais sont journalisés.
pub fn recevoir(db: &mut Db, e: &CommandeEntrante) -> Resultat<Reponse> {
    db.executer(&Acteur::systeme(), |op| {
        if let Some(o) = &e.origine_id {
            let deja: Option<(i64, String, String)> = op
                .query_row(
                    "SELECT numero, code_suivi, COALESCE(validation, '') FROM commandes WHERE origine_id = ?1",
                    params![o],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                )
                .optional()?;
            if let Some((numero, code, v)) = deja {
                let total = crate::commandes::totaux(op, &id_par_code(op, &code)?)?.total;
                return Ok(Reponse { statut: if v == "refusee" { "refusee".into() } else { "en_attente".into() }, message: String::new(), numero: Some(numero), code_suivi: Some(code), total });
            }
        }
        let r = recevoir_op(op, e)?;
        if r.statut == "refusee" {
            op.audit("commande_entrante.refusee", "commande", None, None, Some(json!({ "canal": e.canal, "telephone": e.telephone })), Some(&r.message), None)?;
        }
        Ok(r)
    })
}

fn id_par_code(conn: &Connection, code: &str) -> Resultat<String> {
    trouver(conn.query_row("SELECT id FROM commandes WHERE code_suivi = ?1", params![code], |r| r.get(0)), "Commande")
}

fn recevoir_op(op: &mut Op, e: &CommandeEntrante) -> Resultat<Reponse> {
    let c = op.params.canaux.clone();
    let Some(journee) = crate::journee::ouverte(op)? else {
        return Ok(refus("Le restaurant est fermé pour le moment"));
    };
    if e.lignes.is_empty() {
        return Ok(refus("Votre panier est vide"));
    }
    if e.lignes.len() > 50 || e.note.len() > 500 || e.client_nom.len() > 100 {
        return Ok(refus("Commande trop longue : appelez le restaurant"));
    }
    let telephone = normaliser_telephone(&e.telephone);
    let (type_, table, paiement_mode) = match e.canal.as_str() {
        "qr_table" => {
            if !c.qr_table {
                return Ok(refus("La commande par QR code n'est pas active dans ce restaurant"));
            }
            let t = match table_par_code(op, e.code_table.as_deref().unwrap_or("")) {
                Ok(t) => t,
                Err(err) => return Ok(refus(err.to_string())),
            };
            ("sur_place".to_string(), Some(t), "sur_place".to_string())
        }
        "en_ligne" => {
            if !c.en_ligne {
                return Ok(refus("La commande en ligne n'est pas active dans ce restaurant"));
            }
            let t = e.type_.clone().unwrap_or_else(|| "livraison".into());
            if t != "livraison" && t != "emporter" {
                return Ok(refus("Type de commande inconnu"));
            }
            if telephone.len() < 8 {
                return Ok(refus("Numéro de téléphone obligatoire"));
            }
            // RG-CAN-04 : numéro vérifié par SMS quand ce mode est choisi.
            if c.verification_numero == "sms" && !e.telephone_verifie {
                return Ok(refus("Numéro de téléphone non vérifié"));
            }
            let mode = e.paiement_mode.clone().unwrap_or_else(|| "a_la_livraison".into());
            (t, None, mode)
        }
        _ => return Ok(refus("Canal inconnu")),
    };
    // RG-CAN-03 : liste noire.
    if !telephone.is_empty() && zones_risque::est_bloque(op, &telephone)?.is_some() {
        return Ok(refus("Commande impossible depuis ce numéro. Appelez le restaurant."));
    }
    let mut validation_responsable = false;
    let mut exige_avance = false;
    // RG-ZON-02 : zones à risque.
    let (quartier, repere, position) = match (&type_[..], &e.livraison) {
        ("livraison", Some(l)) => {
            if l.quartier.trim().is_empty() {
                return Ok(refus("Indiquez votre quartier"));
            }
            (Some(l.quartier.trim().to_string()), l.repere.trim().to_string(), l.lat.zip(l.lon))
        }
        ("livraison", None) => return Ok(refus("Adresse de livraison manquante")),
        _ => (None, String::new(), None),
    };
    if type_ == "livraison" {
        if let Some(d) = zones_risque::evaluer(op, quartier.as_deref(), position, op.maintenant)? {
            match d.action.as_str() {
                "bloquer" => return Ok(refus(d.message)),
                "paiement_avance" => exige_avance = true,
                _ => validation_responsable = true,
            }
        }
    }
    // Montant recalculé ici (prix de la zone de la table le cas échéant).
    let mut total_articles = 0;
    for l in &e.lignes {
        let p = match catalogue::produit(op, &l.produit_id) {
            Ok(p) => p,
            Err(_) => return Ok(refus("Un article n'existe plus : rechargez le menu")),
        };
        if !p.actif || !p.disponible {
            return Ok(refus(format!("« {} » n'est plus disponible aujourd'hui", p.nom)));
        }
        if l.quantite <= 0 || l.quantite > 50 {
            return Ok(refus("Quantité invalide"));
        }
        total_articles += l.quantite * crate::promotions::prix_du_moment(op, &p.id, table.as_ref().map(|t| t.2.as_str()), op.maintenant)?.prix;
    }
    // RG-CAN-05 : mode de paiement autorisé, plafond, nouveau client.
    if e.canal == "en_ligne" {
        let nouveau = op.query_row(
            "SELECT COUNT(*) FROM commandes WHERE client_telephone = ?1 AND statut IN ('payee','cloturee')",
            params![telephone],
            |r| r.get::<_, i64>(0),
        )? == 0;
        if c.avance_nouveau_client && nouveau {
            exige_avance = true;
        }
        if c.plafond_paiement_livraison > 0 && total_articles > c.plafond_paiement_livraison {
            exige_avance = true;
        }
        match paiement_mode.as_str() {
            "avance" if !c.paiement_avance => return Ok(refus("Le paiement d'avance n'est pas proposé")),
            "avance" => {
                if e.paiement_reference.as_deref().unwrap_or("").trim().is_empty() || e.paiement_operateur.is_none() {
                    return Ok(refus("Indiquez l'opérateur et la référence du paiement Mobile Money"));
                }
            }
            "a_la_livraison" if exige_avance => return Ok(refus("Paiement Mobile Money d'avance obligatoire pour cette commande")),
            "a_la_livraison" if !c.paiement_a_la_livraison => return Ok(refus("Le paiement à la livraison n'est pas proposé")),
            "a_la_livraison" => {}
            _ => return Ok(refus("Mode de paiement inconnu")),
        }
    }
    let id = op.nouvel_id();
    let numero = op.sequence("commande")?;
    let code_suivi = match &e.code_suivi {
        Some(c) if code_valide(c, 8) => c.clone(),
        _ => code_aleatoire(8),
    };
    let ordre = if paiement_mode == "avance" { "avant" } else { "apres" };
    let frais = match (&type_[..], &quartier) {
        ("livraison", Some(q)) => op.params.quartiers.iter().find(|x| x.nom.eq_ignore_ascii_case(q)).map(|x| x.frais).unwrap_or(0),
        _ => 0,
    };
    let motif = validation_responsable.then(|| "Zone à risque : accord d'un responsable".to_string());
    op.execute(
        "INSERT INTO commandes(id, numero, journee_id, type, ordre_paiement, zone_id, couverts, note, statut,
            livraison_statut, livraison_quartier, livraison_repere, livraison_telephone, livraison_frais, livraison_lat,
            livraison_lon, cree_le, modifie_le, canal, validation, validation_motif, validation_responsable, paiement_mode,
            client_telephone, client_nom_saisi, table_demandee, paiement_reference, paiement_operateur, code_suivi, origine_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, ?7, 'ouverte', ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?15, ?16, 'en_attente', ?17, ?18,
                 ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26)",
        params![
            id,
            numero,
            journee.id,
            type_,
            ordre,
            table.as_ref().map(|t| t.2.clone()),
            e.note.trim(),
            (type_ == "livraison").then_some("nouvelle"),
            quartier,
            (type_ == "livraison").then_some(repere),
            (!telephone.is_empty()).then_some(telephone.clone()),
            frais,
            position.map(|p| p.0),
            position.map(|p| p.1),
            op.maintenant,
            e.canal,
            motif,
            validation_responsable,
            paiement_mode,
            (!telephone.is_empty()).then_some(telephone.clone()),
            e.client_nom.trim(),
            table.as_ref().map(|t| t.0.clone()),
            e.paiement_reference.as_deref().map(str::trim),
            e.paiement_operateur,
            code_suivi,
            e.origine_id,
        ],
    )?;
    // Options invalides, etc. : erreur, toute l'opération est annulée (rien n'est créé).
    commandes::ajouter_lignes_op(op, &id, &e.lignes)?;
    let total = commandes::totaux(op, &id)?.total;
    op.outbox("commande", &id, "recevoir")?;
    op.evenement("commande_entrante", Some(&id));
    Ok(Reponse {
        statut: "en_attente".into(),
        message: "Commande reçue : le restaurant va la confirmer".into(),
        numero: Some(numero),
        code_suivi: Some(code_suivi),
        total,
    })
}

// ───────────── File de validation ─────────────

#[derive(Debug, Serialize)]
pub struct Entrante {
    pub commande: commandes::Commande,
    pub canal: String,
    pub table_demandee: Option<String>,
    pub client_nom: String,
    pub client_telephone: Option<String>,
    pub paiement_mode: Option<String>,
    pub paiement_operateur: Option<String>,
    pub paiement_reference: Option<String>,
    pub validation_responsable: bool,
    pub motif: Option<String>,
    /// Commandes déjà servies à ce numéro (confiance).
    pub commandes_precedentes: i64,
    pub verification_numero: String,
}

pub fn file(conn: &Connection) -> Resultat<Vec<Entrante>> {
    let ids: Vec<String> = conn
        .prepare("SELECT id FROM commandes WHERE validation = 'en_attente' ORDER BY cree_le")?
        .query_map([], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    let verif = crate::parametres::lire(conn)?.canaux.verification_numero;
    let mut v = Vec::new();
    for id in ids {
        let (canal, table, nom, tel, mode, op_mm, reference, resp, motif): (
            String,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            bool,
            Option<String>,
        ) = conn.query_row(
            "SELECT c.canal, t.nom, c.client_nom_saisi, c.client_telephone, c.paiement_mode, c.paiement_operateur,
                    c.paiement_reference, c.validation_responsable, c.validation_motif
             FROM commandes c LEFT JOIN tables_salle t ON t.id = c.table_demandee WHERE c.id = ?1",
            params![id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?, r.get(8)?)),
        )?;
        let precedentes = match &tel {
            Some(t) => conn.query_row(
                "SELECT COUNT(*) FROM commandes WHERE client_telephone = ?1 AND statut IN ('payee','cloturee')",
                params![t],
                |r| r.get(0),
            )?,
            None => 0,
        };
        v.push(Entrante {
            commande: commandes::detail(conn, &id)?,
            canal,
            table_demandee: table,
            client_nom: nom.unwrap_or_default(),
            client_telephone: tel,
            paiement_mode: mode,
            paiement_operateur: op_mm,
            paiement_reference: reference,
            validation_responsable: resp,
            motif,
            commandes_precedentes: precedentes,
            verification_numero: verif.clone(),
        });
    }
    Ok(v)
}

/// Acceptation ou refus d'une commande reçue (RG-CAN-02).
/// Paiement d'avance : il est encaissé à l'acceptation (référence Mobile Money « à vérifier »).
pub fn valider(db: &mut Db, acteur: &Acteur, commande_id: &str, accepter: bool, motif: &str) -> Resultat<()> {
    db.executer(acteur, |op| {
        let autorise_par = op.exiger(perm::COMMANDE_VALIDER_ENTRANTE)?;
        let (validation, resp, table, mode, operateur, reference, statut): (
            Option<String>,
            bool,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            String,
        ) = trouver(
            op.query_row(
                "SELECT validation, validation_responsable, table_demandee, paiement_mode, paiement_operateur, paiement_reference, statut
                 FROM commandes WHERE id = ?1",
                params![commande_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?)),
            ),
            "Commande",
        )?;
        if validation.as_deref() != Some("en_attente") || statut != "ouverte" {
            return Err(Erreur::validation("Cette commande a déjà été traitée"));
        }
        if !accepter {
            if motif.trim().is_empty() {
                return Err(Erreur::regle("RG-CAN-02", "Motif obligatoire pour refuser"));
            }
            op.execute(
                "UPDATE commandes SET validation = 'refusee', validation_motif = ?1, statut = 'annulee', cloturee_le = ?2, modifie_le = ?2
                 WHERE id = ?3",
                params![motif.trim(), op.maintenant, commande_id],
            )?;
            op.audit("commande_entrante.refuser", "commande", Some(commande_id), None, None, Some(motif.trim()), autorise_par.as_deref())?;
            op.evenement("commande", Some(commande_id));
            return Ok(());
        }
        // RG-ZON-02 : zone « validation manuelle » → un responsable.
        let par_zone = if resp { op.exiger(perm::ZONE_OUTREPASSER)? } else { None };
        let mut cible = commande_id.to_string();
        if let Some(t) = &table {
            let ouverte: Option<String> = op
                .query_row("SELECT id FROM commandes WHERE table_id = ?1 AND statut = 'ouverte'", params![t], |r| r.get(0))
                .optional()?;
            match ouverte {
                // La table a déjà une addition : nouvelle tournée sur la même addition (RG-CMD-01).
                Some(existante) => {
                    op.execute("UPDATE lignes_commande SET commande_id = ?1 WHERE commande_id = ?2", params![existante, commande_id])?;
                    op.execute(
                        "UPDATE commandes SET validation = 'acceptee', statut = 'annulee', note = note || ' [ajoutée à l''addition de la table]',
                            cloturee_le = ?1, modifie_le = ?1 WHERE id = ?2",
                        params![op.maintenant, commande_id],
                    )?;
                    cible = existante;
                }
                None => {
                    let zone = crate::salle::zone_de_table(op, t)?;
                    op.execute(
                        "UPDATE commandes SET table_id = ?1, zone_id = ?2, validation = 'acceptee', modifie_le = ?3 WHERE id = ?4",
                        params![t, zone, op.maintenant, commande_id],
                    )?;
                }
            }
        } else {
            op.execute(
                "UPDATE commandes SET validation = 'acceptee', serveur_id = COALESCE(serveur_id, ?1), modifie_le = ?2 WHERE id = ?3",
                params![op.utilisateur(), op.maintenant, commande_id],
            )?;
        }
        if mode.as_deref() == Some("avance") {
            let operateur = operateur.unwrap_or_default();
            let compte: String = op
                .query_row(
                    "SELECT id FROM comptes_tresorerie WHERE nom = ?1 AND type = 'mobile_money' AND actif = 1",
                    params![operateur],
                    |r| r.get(0),
                )
                .optional()?
                .ok_or_else(|| Erreur::validation(format!("Compte Mobile Money « {operateur} » introuvable")))?;
            let reste = commandes::totaux(op, &cible)?.reste;
            let part = crate::caisse::PartSaisie {
                moyen: "mobile_money".into(),
                montant: reste,
                compte_id: Some(compte),
                reference: reference.clone(),
                numero_payeur: None,
                client_id: None,
                par_livreur: false,
            };
            crate::caisse::encaisser_op(op, &crate::caisse::Encaissement { commande_id: cible.clone(), parts: vec![part], especes_recues: None })?;
        } else {
            commandes::envoyer_op(op, &cible)?;
        }
        op.audit(
            "commande_entrante.accepter",
            "commande",
            Some(commande_id),
            None,
            Some(json!({ "paiement": mode })),
            None,
            par_zone.as_deref().or(autorise_par.as_deref()),
        )?;
        op.evenement("commande", Some(&cible));
        op.evenement("table", None);
        Ok(())
    })
}

// ───────────── Suivi public ─────────────

#[derive(Debug, Serialize)]
pub struct Suivi {
    pub numero: i64,
    pub restaurant: String,
    /// recue | refusee | acceptee | en_preparation | prete | en_route | livree | echec | annulee
    pub etape: String,
    pub motif: Option<String>,
    #[serde(rename = "type")]
    pub type_: String,
    pub total: i64,
    pub reste: i64,
    pub paiement_mode: Option<String>,
    pub lignes: Vec<(i64, String)>,
    /// Dernière position du livreur (microdegrés, horodatage), pendant la course seulement.
    pub livreur: Option<(i64, i64, i64)>,
    pub destination: Option<(i64, i64)>,
    pub mis_a_jour: i64,
}

/// Page de suivi du client (sans connexion : le code de suivi fait office de clé).
pub fn suivi(conn: &Connection, code: &str) -> Resultat<Suivi> {
    let id = id_par_code(conn, code.trim())?;
    let c = commandes::detail(conn, &id)?;
    let (validation, motif, mode, lat, lon, modifie): (Option<String>, Option<String>, Option<String>, Option<i64>, Option<i64>, i64) =
        conn.query_row(
            "SELECT validation, validation_motif, paiement_mode, livraison_lat, livraison_lon, modifie_le FROM commandes WHERE id = ?1",
            params![id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)),
        )?;
    // Commande QR ajoutée à une addition existante : on suit l'addition de la table.
    let tous_prets = !c.envois.is_empty() && c.envois.iter().all(|e| e.statut == "pret" || e.statut == "servi");
    let etape = match (validation.as_deref(), c.statut.as_str(), c.livraison_statut.as_deref()) {
        (Some("en_attente"), _, _) => "recue",
        (Some("refusee"), _, _) => "refusee",
        (_, _, Some("livree")) => "livree",
        (_, _, Some("echec")) => "echec",
        (_, _, Some("en_route")) => "en_route",
        (_, "annulee", _) if c.note.contains("[ajoutée") => "acceptee",
        (_, "annulee", _) => "annulee",
        _ if tous_prets => "prete",
        _ if !c.envois.is_empty() => "en_preparation",
        _ => "acceptee",
    };
    let livreur = if etape == "en_route" {
        conn.query_row(
            "SELECT lat, lon, horodatage FROM positions_livreur WHERE commande_id = ?1 ORDER BY horodatage DESC LIMIT 1",
            params![id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?
    } else {
        None
    };
    Ok(Suivi {
        numero: c.numero,
        restaurant: crate::parametres::restaurant(conn)?.nom,
        etape: etape.into(),
        motif: if etape == "refusee" { motif } else { None },
        type_: c.type_.clone(),
        total: c.totaux.total,
        reste: c.totaux.reste,
        paiement_mode: mode,
        lignes: c.lignes.iter().filter(|l| l.quantite > l.quantite_annulee).map(|l| (l.quantite - l.quantite_annulee, l.libelle.clone())).collect(),
        livreur,
        destination: lat.zip(lon),
        mis_a_jour: modifie,
    })
}

/// Position envoyée par le téléphone du livreur (RG-LIV-04) : seulement pendant la course,
/// avec le lien secret du livreur (le code de suivi du client ne permet pas d'envoyer une position).
pub fn ajouter_position(db: &mut Db, code_livreur: &str, lat: i64, lon: i64) -> Resultat<()> {
    if !(-90_000_000..=90_000_000).contains(&lat) || !(-180_000_000..=180_000_000).contains(&lon) {
        return Err(Erreur::validation("Position invalide"));
    }
    db.executer(&Acteur::systeme(), |op| {
        let (id, statut): (String, Option<String>) = trouver(
            op.query_row(
                "SELECT id, livraison_statut FROM commandes WHERE code_livreur = ?1",
                params![code_livreur.trim()],
                |r| Ok((r.get(0)?, r.get(1)?)),
            ),
            "Course",
        )?;
        if !matches!(statut.as_deref(), Some("assignee") | Some("en_route")) {
            return Err(Erreur::regle("RG-LIV-04", "Le suivi n'est actif que pendant la course"));
        }
        op.execute(
            "INSERT INTO positions_livreur(id, commande_id, lat, lon, horodatage) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![op.nouvel_id(), id, lat, lon, op.maintenant],
        )?;
        op.evenement("position", Some(&id));
        Ok(())
    })
}

#[derive(Debug, Serialize, PartialEq)]
pub struct Liens {
    /// Code de la page de suivi du client.
    pub code_suivi: String,
    /// Code de la page du livreur (livraisons seulement).
    pub code_livreur: Option<String>,
}

/// Codes de suivi d'une commande, créés à la demande (pour l'envoyer au client ou ouvrir la page livreur).
pub fn liens(db: &mut Db, acteur: &Acteur, commande_id: &str) -> Resultat<Liens> {
    db.executer(acteur, |op| {
        op.exiger(perm::COMMANDE_CREER)?;
        let (suivi, livreur, type_): (Option<String>, Option<String>, String) = trouver(
            op.query_row(
                "SELECT code_suivi, code_livreur, type FROM commandes WHERE id = ?1",
                params![commande_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            ),
            "Commande",
        )?;
        let suivi = match suivi {
            Some(c) => c,
            None => {
                let c = code_aleatoire(8);
                op.execute("UPDATE commandes SET code_suivi = ?1 WHERE id = ?2", params![c, commande_id])?;
                c
            }
        };
        let livreur = match (livreur, type_.as_str()) {
            (Some(c), _) => Some(c),
            (None, "livraison") => {
                let c = code_aleatoire(12);
                op.execute("UPDATE commandes SET code_livreur = ?1 WHERE id = ?2", params![c, commande_id])?;
                Some(c)
            }
            _ => None,
        };
        Ok(Liens { code_suivi: suivi, code_livreur: livreur })
    })
}

// ───────────── Relais Internet (fiche 0013) ─────────────

#[derive(Debug, Serialize)]
pub struct SuiviRelais {
    pub code_suivi: String,
    pub code_livreur: Option<String>,
    pub suivi: Suivi,
}

/// Suivis publiés sur le relais : commandes récentes ayant un code de suivi.
pub fn suivis_recents(conn: &Connection, depuis_ms: i64) -> Resultat<Vec<SuiviRelais>> {
    let codes: Vec<(String, Option<String>)> = conn
        .prepare("SELECT code_suivi, code_livreur FROM commandes WHERE code_suivi IS NOT NULL AND (cree_le >= ?1 OR modifie_le >= ?1) ORDER BY modifie_le DESC LIMIT 300")?
        .query_map(params![depuis_ms], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<_, _>>()?;
    let mut v = Vec::with_capacity(codes.len());
    for (code, livreur) in codes {
        let suivi = suivi(conn, &code)?;
        v.push(SuiviRelais { code_suivi: code, code_livreur: livreur, suivi });
    }
    Ok(v)
}
