//! Routes HTTP. Chaque route délègue au cœur (`youma-core`) : aucune règle métier ici,
//! seulement l'authentification, les droits de lecture et la sérialisation.

use std::collections::HashMap;
use std::path::PathBuf;

use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use tower_http::services::{ServeDir, ServeFile};
use youma_core::erreur::{Erreur, Resultat};
use youma_core::permissions as perm;
use youma_core::{
    achats, appareils, auth, caisse, catalogue, clients, commandes, demo, employes, entrantes, horloge, impression, journee,
    licence, livraison, paie, parametres, rapports, salle, sauvegarde, stock, zones_risque, Db,
};

use crate::erreurs::{ApiErreur, Rep};
use crate::{Auth, Etat, Poste};

type Q = Query<HashMap<String, String>>;

fn q<'a>(p: &'a HashMap<String, String>, cle: &str) -> Option<&'a str> {
    p.get(cle).map(String::as_str).filter(|s| !s.is_empty())
}

/// Droit de lecture (les écritures sont contrôlées dans le cœur).
/// RG-AUT-06 : l'administration se lit aussi avec une session confirmée par mot de passe.
fn peut(db: &Db, uid: &str, eleve: bool, p: &str) -> Resultat<()> {
    let (perms, _) = auth::permissions_utilisateur(db.conn(), uid)?;
    if !perms.contains(p) {
        return Err(Erreur::Interdit(p.into()));
    }
    if perm::ADMINISTRATION.contains(&p) && !eleve {
        return Err(Erreur::MotDePasseRequis(p.into()));
    }
    Ok(())
}

fn aujourdhui(db: &Db) -> String {
    let p = parametres::lire(db.conn()).unwrap_or_default();
    horloge::date_exploitation(db.maintenant(), p.fuseau_minutes, p.heure_bascule)
}

macro_rules! ecrire {
    ($etat:expr, $auth:ident, |$db:ident| $corps:expr) => {{
        let a = $auth.acteur.clone();
        let r = $etat.avec_db(move |$db| { let $auth = a; let _ = &$auth; $corps }).await?;
        Ok(Json(r))
    }};
}

macro_rules! lire {
    ($etat:expr, $auth:ident, $perm:expr, |$db:ident| $corps:expr) => {{
        let uid = $auth.utilisateur_id.clone();
        let eleve = $auth.acteur.eleve;
        let r = $etat
            .avec_db(move |$db| {
                if let Some(p) = $perm {
                    peut($db, &uid, eleve, p)?;
                }
                $corps
            })
            .await?;
        Ok(Json(r))
    }};
}

const AUCUNE: Option<&str> = None;

pub fn routeur(etat: Etat) -> Router {
    let api = Router::new()
        // Système et connexion
        .route("/etat", get(etat_general))
        .route("/installation", post(installation))
        .route("/connexion/utilisateurs", get(utilisateurs_connexion))
        .route("/connexion", post(connexion))
        .route("/deconnexion", post(deconnexion))
        .route("/session", get(session))
        .route("/session/elever", post(session_elever))
        .route("/moi/mot-de-passe", post(mon_mot_de_passe))
        .route("/sortie/controle", post(sortie_controle))
        .route("/horloge/accepter", post(horloge_accepter))
        .route("/ws", get(crate::ws::ws))
        // Journée
        .route("/journees", get(journees))
        .route("/journee/ouvrir", post(journee_ouvrir))
        .route("/journee/cloturer", post(journee_cloturer))
        // Catalogue
        .route("/catalogue", get(catalogue_tout))
        .route("/categories", post(categorie_enregistrer))
        .route("/produits", post(produit_enregistrer))
        .route("/produits/import", post(produits_import))
        .route("/produits/{id}/disponibilite", post(produit_disponibilite))
        .route("/produits/{id}/historique", get(produit_historique))
        .route("/postes", post(poste_enregistrer))
        // Salle
        .route("/salle", get(salle_plan))
        .route("/zones", post(zone_enregistrer))
        .route("/tables", post(table_enregistrer))
        .route("/tables/serie", post(tables_serie))
        .route("/tables/{id}/marquer", post(table_marquer))
        // Commandes
        .route("/commandes", get(commandes_lister).post(commande_ouvrir))
        .route("/commandes/{id}", get(commande_detail))
        .route("/commandes/{id}/lignes", post(commande_lignes))
        .route("/commandes/{id}/envoyer", post(commande_envoyer))
        .route("/commandes/{id}/remise", post(commande_remise))
        .route("/commandes/{id}/transferer", post(commande_transferer))
        .route("/commandes/{id}/fusionner", post(commande_fusionner))
        .route("/commandes/{id}/abandonner", post(commande_abandonner))
        .route("/commandes/{id}/imputer", post(commande_imputer))
        .route("/commandes/{id}/client", post(commande_client))
        .route("/commandes/{id}/ticket", get(commande_ticket))
        .route("/commandes/{id}/imprimer", post(commande_imprimer))
        .route("/commandes/{id}/diviser", get(commande_diviser))
        .route("/commandes/{id}/montant-lignes", post(commande_montant_lignes))
        .route("/commandes/{id}/paiements", get(commande_paiements))
        .route("/lignes/{id}", put(ligne_modifier))
        .route("/lignes/{id}/annuler", post(ligne_annuler))
        .route("/lignes/{id}/offrir", post(ligne_offrir))
        // Cuisine et impression
        .route("/cuisine", get(cuisine))
        .route("/envois/{id}/statut", post(envoi_statut))
        .route("/impressions", get(impressions))
        .route("/impressions/{id}/reimprimer", post(impression_reimprimer))
        // Caisse
        .route("/caisse", get(caisse_etat))
        .route("/caisse/ouvrir", post(caisse_ouvrir))
        .route("/caisse/{id}/cloturer", post(caisse_cloturer))
        .route("/caisse/{id}/z", get(caisse_z))
        .route("/caisse/encaisser", post(caisse_encaisser))
        .route("/caisse/mouvement", post(caisse_mouvement))
        .route("/caisse/transfert", post(caisse_transfert))
        .route("/caisse/mouvements", get(caisse_mouvements))
        .route("/paiements/{id}/annuler", post(paiement_annuler))
        .route("/comptes", get(comptes).post(compte_enregistrer))
        .route("/depenses", get(depenses).post(depense_creer))
        .route("/depenses/{id}/annuler", post(depense_annuler))
        .route("/depenses/categories", get(depenses_categories).post(depense_categorie_creer))
        .route("/mobile-money", get(mobile_money))
        .route("/mobile-money/{id}/verifier", post(mobile_money_verifier))
        // Stock et achats
        .route("/stock", get(stock_niveaux))
        .route("/stock/articles", post(article_enregistrer))
        .route("/stock/articles/{id}/mouvements", get(article_mouvements))
        .route("/stock/mouvements", post(stock_mouvement))
        .route("/inventaires", get(inventaires).post(inventaire_creer))
        .route("/inventaires/{id}", get(inventaire_detail))
        .route("/inventaires/{id}/comptage", post(inventaire_comptage))
        .route("/inventaires/{id}/valider", post(inventaire_valider))
        .route("/inventaires/{id}/abandonner", post(inventaire_abandonner))
        .route("/fournisseurs", get(fournisseurs).post(fournisseur_enregistrer))
        .route("/fournisseurs/reglement", post(fournisseur_reglement))
        .route("/achats", get(achats_lister).post(achat_receptionner))
        // Clients
        .route("/clients", get(clients_rechercher).post(client_enregistrer))
        .route("/clients/{id}", get(client_detail))
        .route("/clients/reglement", post(client_reglement))
        // Employés et paie
        .route("/employes", get(employes_lister).post(employe_enregistrer))
        .route("/employes/references", get(employes_references))
        .route("/employes/{id}", get(employe_detail))
        .route("/employes/avance", post(employe_avance))
        .route("/employes/evenement", post(employe_evenement))
        .route("/presences", get(presences).post(presences_pointer))
        .route("/paie/apercu", get(paie_apercu))
        .route("/paie/cloturer", post(paie_cloturer))
        .route("/paie/payer", post(paie_payer))
        .route("/paie/bulletins", get(paie_bulletins))
        .route("/paie/bulletins/{id}", get(paie_bulletin))
        // Livraison
        .route("/livraisons", get(livraisons))
        .route("/livraisons/{id}/assigner", post(livraison_assigner))
        .route("/livraisons/{id}/statut", post(livraison_statut))
        .route("/livreurs", get(livreurs))
        .route("/livreurs/{id}/remise", post(livreur_remise))
        // Rapports
        .route("/tableau-de-bord", get(tableau_de_bord))
        .route("/rapports/periode", get(rapport_periode))
        .route("/rapports/stock", get(rapport_stock))
        .route("/rapports/dettes", get(rapport_dettes))
        .route("/audit", get(audit))
        // Administration
        .route("/parametres", get(parametres_lire).put(parametres_modifier))
        .route("/restaurant", get(restaurant_lire).put(restaurant_modifier))
        .route("/roles", get(roles).put(role_modifier))
        .route("/utilisateurs", get(utilisateurs).post(utilisateur_creer))
        .route("/utilisateurs/{id}", put(utilisateur_modifier))
        .route("/utilisateurs/{id}/mot-de-passe", post(utilisateur_mot_de_passe))
        .route("/appareils", get(appareils_lister))
        .route("/appareils/code", post(appareil_code))
        .route("/appareils/appairer", post(appareil_appairer))
        .route("/appareils/{id}/revoquer", post(appareil_revoquer))
        .route("/reseau", get(reseau))
        .route("/diagnostic", get(diagnostic))
        .route("/sauvegardes", get(sauvegardes).post(sauvegarde_creer))
        .route("/sauvegardes/exporter", post(sauvegarde_exporter))
        .route("/sauvegardes/restaurer", post(sauvegarde_restaurer))
        .route("/integrite", post(integrite))
        .route("/licence", get(licence_etat).post(licence_installer))
        // Commandes reçues (QR, en ligne), zones à risque, liste noire (fiche 0013)
        .route("/entrantes", get(entrantes_file))
        .route("/entrantes/{id}/valider", post(entrante_valider))
        .route("/zones-risque", get(zones_risque_lister).post(zone_risque_enregistrer))
        .route("/numeros-bloques", get(numeros_bloques).post(numero_bloquer))
        .route("/numeros-bloques/debloquer", post(numero_debloquer))
        .route("/tables/codes-qr", get(codes_qr).post(codes_qr_generer))
        .route("/commandes/{id}/liens", post(commande_liens))
        .route("/relais/etat", get(relais_etat))
        // Routes publiques : sans connexion ni appairage, seulement si le canal est activé.
        .route("/public/menu", get(public_menu))
        .route("/public/commandes", post(public_commande))
        .route("/public/suivi/{code}", get(public_suivi))
        .route("/public/position/{code}", post(public_position));

    let mut app = Router::new().nest("/api", api);
    if let Some(ui) = &etat.config.dossier_ui {
        let index = ui.join("index.html");
        app = app.fallback_service(ServeDir::new(ui).fallback(ServeFile::new(index)));
    }
    app.with_state(etat)
}

// ───────────── Système ─────────────

async fn etat_general(State(e): State<Etat>, poste: Result<Poste, ApiErreur>) -> Rep<Value> {
    let config = e.config.clone();
    if let Err(ApiErreur(err)) = poste {
        // Appareil du réseau non appairé : aucune donnée, seulement l'invitation à scanner le QR.
        return Ok(Json(json!({ "installe": true, "appairage_requis": true, "message": err.to_string() })));
    }
    let v = e
        .avec_db(move |db| {
            let r = parametres::restaurant(db.conn())?;
            Ok(json!({
                "installe": auth::nombre_utilisateurs(db.conn())? > 0,
                "demo": demo::est_demo(db.conn()),
                "restaurant": r.nom,
                "horloge": db.etat_horloge()?,
                "journee": journee::ouverte(db.conn())?,
                "version": env!("CARGO_PKG_VERSION"),
                "reseau": config.reseau,
                "parametres": parametres::lire(db.conn())?,
            }))
        })
        .await?;
    Ok(Json(v))
}

#[derive(Deserialize)]
struct Installation {
    nom: String,
    pin: String,
    mot_de_passe: String,
    restaurant: String,
}

async fn installation(State(e): State<Etat>, Json(i): Json<Installation>) -> Rep<Value> {
    let id = e.avec_db(move |db| auth::installer_proprietaire(db, &i.nom, &i.pin, &i.mot_de_passe, &i.restaurant)).await?;
    Ok(Json(json!({ "id": id })))
}

async fn utilisateurs_connexion(State(e): State<Etat>, _p: Poste) -> Rep<Value> {
    let v = e.avec_db(|db| auth::lister_utilisateurs(db.conn(), true)).await?;
    Ok(Json(json!(v.into_iter().map(|u| json!({ "id": u.id, "nom": u.nom, "role": u.role_nom })).collect::<Vec<_>>())))
}

#[derive(Deserialize)]
struct Connexion {
    utilisateur_id: String,
    pin: String,
}

async fn connexion(State(e): State<Etat>, Poste(appareil): Poste, Json(c): Json<Connexion>) -> Rep<auth::Session> {
    let s = e.avec_db(move |db| auth::connexion_pin(db, &c.utilisateur_id, &c.pin, appareil.as_deref())).await?;
    Ok(Json(s))
}

async fn deconnexion(State(e): State<Etat>, a: Auth) -> Rep<Value> {
    let j = a.jeton.clone();
    e.avec_db(move |db| auth::deconnexion(db, &j)).await?;
    Ok(Json(json!({})))
}

async fn session(State(e): State<Etat>, a: Auth) -> Rep<auth::Session> {
    let j = a.jeton.clone();
    Ok(Json(e.avec_db(move |db| auth::session_courante(db, &j)).await?))
}

#[derive(Deserialize)]
struct MotDePasse {
    mot_de_passe: String,
}

/// RG-AUT-06 : confirme la session pour accéder à l'administration.
async fn session_elever(State(e): State<Etat>, a: Auth, Json(m): Json<MotDePasse>) -> Rep<auth::Session> {
    let j = a.jeton.clone();
    Ok(Json(e.avec_db(move |db| auth::elever_session(db, &j, &m.mot_de_passe)).await?))
}

#[derive(Deserialize)]
struct ChangementMotDePasse {
    #[serde(default)]
    ancien: Option<String>,
    nouveau: String,
}

async fn mon_mot_de_passe(State(e): State<Etat>, a: Auth, Json(c): Json<ChangementMotDePasse>) -> Rep<()> {
    let uid = a.utilisateur_id.clone();
    ecrire!(e, a, |db| auth::definir_mot_de_passe(db, &a, &uid, c.ancien.as_deref(), &c.nouveau))
}

async fn utilisateur_mot_de_passe(State(e): State<Etat>, a: Auth, Path(id): Path<String>, Json(c): Json<ChangementMotDePasse>) -> Rep<()> {
    ecrire!(e, a, |db| auth::definir_mot_de_passe(db, &a, &id, c.ancien.as_deref(), &c.nouveau))
}

#[derive(Deserialize)]
struct Controle {
    numero: i64,
    code: String,
}

/// Contrôle du bon de sortie (RG-SOR-01 à 03).
async fn sortie_controle(State(e): State<Etat>, a: Auth, Json(c): Json<Controle>) -> Rep<youma_core::sortie::ResultatControle> {
    ecrire!(e, a, |db| youma_core::sortie::controler(db, &a, c.numero, &c.code))
}

async fn horloge_accepter(State(e): State<Etat>, a: Auth) -> Rep<()> {
    ecrire!(e, a, |db| auth::accepter_heure(db, &a))
}

// ───────────── Journée ─────────────

async fn journees(State(e): State<Etat>, a: Auth) -> Rep<Vec<journee::Journee>> {
    lire!(e, a, AUCUNE, |db| journee::lister(db.conn(), 60))
}

async fn journee_ouvrir(State(e): State<Etat>, a: Auth) -> Rep<journee::Journee> {
    ecrire!(e, a, |db| journee::ouvrir(db, &a))
}

async fn journee_cloturer(State(e): State<Etat>, a: Auth) -> Rep<journee::Journee> {
    let dossier = e.config.dossier_sauvegardes();
    ecrire!(e, a, |db| {
        let j = journee::cloturer(db, &a)?;
        // Sauvegarde automatique à la clôture (cahier §20).
        if db.chemin().is_some() {
            if let Err(err) = sauvegarde::sauvegarder(db, &dossier, "cloture").and_then(|_| sauvegarde::rotation(&dossier)) {
                tracing::error!("Sauvegarde de clôture en échec : {err}");
            }
        }
        Ok(j)
    })
}

// ───────────── Catalogue ─────────────

async fn catalogue_tout(State(e): State<Etat>, a: Auth) -> Rep<Value> {
    lire!(e, a, AUCUNE, |db| Ok(json!({
        "categories": catalogue::lister_categories(db.conn())?,
        "produits": catalogue::lister_produits(db.conn(), false)?,
        "postes": catalogue::lister_postes(db.conn())?,
    })))
}

async fn categorie_enregistrer(State(e): State<Etat>, a: Auth, Json(c): Json<catalogue::Categorie>) -> Rep<String> {
    ecrire!(e, a, |db| catalogue::enregistrer_categorie(db, &a, &c))
}

async fn produit_enregistrer(State(e): State<Etat>, a: Auth, Json(p): Json<catalogue::Produit>) -> Rep<String> {
    ecrire!(e, a, |db| catalogue::enregistrer_produit(db, &a, &p))
}

async fn produits_import(State(e): State<Etat>, a: Auth, corps: String) -> Rep<usize> {
    ecrire!(e, a, |db| catalogue::importer_csv(db, &a, &corps))
}

#[derive(Deserialize)]
struct Disponibilite {
    disponible: bool,
}

async fn produit_disponibilite(State(e): State<Etat>, a: Auth, Path(id): Path<String>, Json(d): Json<Disponibilite>) -> Rep<()> {
    ecrire!(e, a, |db| catalogue::definir_disponibilite(db, &a, &id, d.disponible))
}

async fn produit_historique(State(e): State<Etat>, a: Auth, Path(id): Path<String>) -> Rep<Vec<catalogue::LigneHistoriquePrix>> {
    lire!(e, a, Some(perm::CATALOGUE_GERER), |db| catalogue::historique_prix(db.conn(), &id))
}

async fn poste_enregistrer(State(e): State<Etat>, a: Auth, Json(p): Json<catalogue::Poste>) -> Rep<String> {
    ecrire!(e, a, |db| catalogue::enregistrer_poste(db, &a, &p))
}

// ───────────── Salle ─────────────

async fn salle_plan(State(e): State<Etat>, a: Auth) -> Rep<Value> {
    lire!(e, a, AUCUNE, |db| Ok(json!({ "zones": salle::lister_zones(db.conn())?, "tables": salle::plan(db.conn())? })))
}

async fn zone_enregistrer(State(e): State<Etat>, a: Auth, Json(z): Json<salle::Zone>) -> Rep<String> {
    ecrire!(e, a, |db| salle::enregistrer_zone(db, &a, &z))
}

async fn table_enregistrer(State(e): State<Etat>, a: Auth, Json(t): Json<salle::Table>) -> Rep<String> {
    ecrire!(e, a, |db| salle::enregistrer_table(db, &a, &t))
}

#[derive(Deserialize)]
struct Serie {
    zone_id: String,
    #[serde(default)]
    prefixe: String,
    debut: i64,
    nombre: i64,
}

async fn tables_serie(State(e): State<Etat>, a: Auth, Json(s): Json<Serie>) -> Rep<usize> {
    ecrire!(e, a, |db| salle::creer_tables_serie(db, &a, &s.zone_id, &s.prefixe, s.debut, s.nombre))
}

#[derive(Deserialize)]
struct Marque {
    reservee: Option<bool>,
    a_nettoyer: Option<bool>,
}

async fn table_marquer(State(e): State<Etat>, a: Auth, Path(id): Path<String>, Json(m): Json<Marque>) -> Rep<()> {
    ecrire!(e, a, |db| salle::marquer_table(db, &a, &id, m.reservee, m.a_nettoyer))
}

// ───────────── Commandes ─────────────

async fn commandes_lister(State(e): State<Etat>, a: Auth, Query(p): Q) -> Rep<Vec<commandes::CommandeResume>> {
    let statut = q(&p, "statut").map(str::to_owned);
    let journee_id = q(&p, "journee").map(str::to_owned);
    lire!(e, a, AUCUNE, |db| {
        let j = match journee_id {
            Some(j) => j,
            None => match journee::ouverte(db.conn())? {
                Some(j) => j.id,
                None => return Ok(vec![]),
            },
        };
        commandes::lister(db.conn(), &j, statut.as_deref())
    })
}

async fn commande_ouvrir(State(e): State<Etat>, a: Auth, Json(n): Json<commandes::NouvelleCommande>) -> Rep<String> {
    ecrire!(e, a, |db| commandes::ouvrir(db, &a, &n))
}

async fn commande_detail(State(e): State<Etat>, a: Auth, Path(id): Path<String>) -> Rep<commandes::Commande> {
    lire!(e, a, AUCUNE, |db| commandes::detail(db.conn(), &id))
}

async fn commande_lignes(State(e): State<Etat>, a: Auth, Path(id): Path<String>, Json(l): Json<Vec<commandes::LigneSaisie>>) -> Rep<Vec<String>> {
    ecrire!(e, a, |db| commandes::ajouter_lignes(db, &a, &id, &l))
}

async fn commande_envoyer(State(e): State<Etat>, a: Auth, Path(id): Path<String>) -> Rep<Vec<String>> {
    ecrire!(e, a, |db| commandes::envoyer(db, &a, &id))
}

#[derive(Deserialize)]
struct RemiseSaisie {
    ligne_id: Option<String>,
    montant: Option<i64>,
    pourcentage: Option<i64>,
    motif: String,
}

async fn commande_remise(State(e): State<Etat>, a: Auth, Path(id): Path<String>, Json(r): Json<RemiseSaisie>) -> Rep<()> {
    ecrire!(e, a, |db| commandes::remise(db, &a, &id, r.ligne_id.as_deref(), r.montant, r.pourcentage, &r.motif))
}

#[derive(Deserialize)]
struct Cible {
    cible: String,
}

async fn commande_transferer(State(e): State<Etat>, a: Auth, Path(id): Path<String>, Json(c): Json<Cible>) -> Rep<()> {
    ecrire!(e, a, |db| commandes::transferer(db, &a, &id, &c.cible))
}

async fn commande_fusionner(State(e): State<Etat>, a: Auth, Path(id): Path<String>, Json(c): Json<Cible>) -> Rep<()> {
    ecrire!(e, a, |db| commandes::fusionner(db, &a, &id, &c.cible))
}

async fn commande_abandonner(State(e): State<Etat>, a: Auth, Path(id): Path<String>) -> Rep<()> {
    ecrire!(e, a, |db| commandes::abandonner(db, &a, &id))
}

async fn commande_imputer(State(e): State<Etat>, a: Auth, Path(id): Path<String>) -> Rep<i64> {
    ecrire!(e, a, |db| employes::imputer_commande(db, &a, &id))
}

#[derive(Deserialize)]
struct ClientCommande {
    client_id: Option<String>,
}

async fn commande_client(State(e): State<Etat>, a: Auth, Path(id): Path<String>, Json(c): Json<ClientCommande>) -> Rep<()> {
    ecrire!(e, a, |db| commandes::definir_client(db, &a, &id, c.client_id.as_deref()))
}

async fn commande_ticket(State(e): State<Etat>, a: Auth, Path(id): Path<String>) -> Rep<String> {
    lire!(e, a, AUCUNE, |db| impression::ticket_client(db.conn(), &id).map(|t| impression::texte_brut(&t)))
}

async fn commande_imprimer(State(e): State<Etat>, a: Auth, Path(id): Path<String>) -> Rep<Option<String>> {
    ecrire!(e, a, |db| impression::imprimer_ticket_client(db, &a, &id))
}

async fn commande_diviser(State(e): State<Etat>, a: Auth, Path(id): Path<String>, Query(p): Q) -> Rep<Vec<i64>> {
    let parts: i64 = q(&p, "parts").and_then(|s| s.parse().ok()).unwrap_or(2);
    lire!(e, a, AUCUNE, |db| {
        let t = commandes::totaux(db.conn(), &id)?;
        Ok(commandes::diviser_parts_egales(t.reste, parts, parametres::lire(db.conn())?.arrondi))
    })
}

async fn commande_montant_lignes(State(e): State<Etat>, a: Auth, Path(id): Path<String>, Json(l): Json<Vec<String>>) -> Rep<i64> {
    lire!(e, a, AUCUNE, |db| commandes::montant_lignes(db.conn(), &id, &l))
}

async fn commande_paiements(State(e): State<Etat>, a: Auth, Path(id): Path<String>) -> Rep<Vec<caisse::PaiementLu>> {
    lire!(e, a, AUCUNE, |db| caisse::paiements_commande(db.conn(), &id))
}

#[derive(Deserialize)]
struct ModifLigne {
    quantite: i64,
    commentaire: Option<String>,
}

async fn ligne_modifier(State(e): State<Etat>, a: Auth, Path(id): Path<String>, Json(m): Json<ModifLigne>) -> Rep<()> {
    ecrire!(e, a, |db| commandes::modifier_brouillon(db, &a, &id, m.quantite, m.commentaire.as_deref()))
}

#[derive(Deserialize)]
struct Annulation {
    quantite: i64,
    #[serde(default)]
    motif: String,
    #[serde(default)]
    perte: bool,
}

async fn ligne_annuler(State(e): State<Etat>, a: Auth, Path(id): Path<String>, Json(x): Json<Annulation>) -> Rep<()> {
    ecrire!(e, a, |db| commandes::annuler_ligne(db, &a, &id, x.quantite, &x.motif, x.perte))
}

#[derive(Deserialize)]
struct Motif {
    #[serde(default)]
    motif: String,
}

async fn ligne_offrir(State(e): State<Etat>, a: Auth, Path(id): Path<String>, Json(m): Json<Motif>) -> Rep<()> {
    ecrire!(e, a, |db| commandes::offrir(db, &a, &id, &m.motif))
}

// ───────────── Cuisine, impression ─────────────

async fn cuisine(State(e): State<Etat>, a: Auth, Query(p): Q) -> Rep<Vec<commandes::EnvoiCuisine>> {
    let poste = q(&p, "poste").map(str::to_owned);
    lire!(e, a, Some(perm::CUISINE_VOIR), |db| commandes::envois_actifs(db.conn(), poste.as_deref()))
}

#[derive(Deserialize)]
struct StatutEnvoi {
    statut: String,
    #[serde(default)]
    message: String,
}

async fn envoi_statut(State(e): State<Etat>, a: Auth, Path(id): Path<String>, Json(s): Json<StatutEnvoi>) -> Rep<()> {
    ecrire!(e, a, |db| commandes::statut_envoi(db, &a, &id, &s.statut, &s.message))
}

async fn impressions(State(e): State<Etat>, a: Auth) -> Rep<Vec<impression::Job>> {
    lire!(e, a, Some(perm::CUISINE_VOIR), |db| impression::jobs(db.conn(), &[]))
}

#[derive(Deserialize)]
struct Reimpression {
    destination: Option<String>,
}

async fn impression_reimprimer(State(e): State<Etat>, a: Auth, Path(id): Path<String>, Json(r): Json<Reimpression>) -> Rep<String> {
    ecrire!(e, a, |db| impression::reimprimer(db, &a, &id, r.destination.as_deref()))
}

// ───────────── Caisse ─────────────

async fn caisse_etat(State(e): State<Etat>, a: Auth) -> Rep<Value> {
    let uid = a.utilisateur_id.clone();
    lire!(e, a, AUCUNE, |db| Ok(json!({
        "session": caisse::session_utilisateur(db.conn(), &uid)?,
        "sessions_ouvertes": caisse::sessions_ouvertes(db.conn())?,
        "comptes": caisse::lister_comptes(db.conn())?,
        "coupures": parametres::lire(db.conn())?.coupures,
    })))
}

async fn caisse_ouvrir(State(e): State<Etat>, a: Auth, Json(o): Json<caisse::OuvertureSession>) -> Rep<String> {
    ecrire!(e, a, |db| caisse::ouvrir_session(db, &a, &o))
}

async fn caisse_cloturer(State(e): State<Etat>, a: Auth, Path(id): Path<String>, Json(c): Json<caisse::ClotureSession>) -> Rep<Value> {
    ecrire!(e, a, |db| {
        let s = caisse::cloturer_session(db, &a, &id, &c)?;
        let z = rapports::rapport_z(db.conn(), &id)?;
        Ok(json!({ "session": s, "z": impression::texte_brut(&z) }))
    })
}

async fn caisse_z(State(e): State<Etat>, a: Auth, Path(id): Path<String>) -> Rep<String> {
    lire!(e, a, AUCUNE, |db| rapports::rapport_z(db.conn(), &id).map(|z| impression::texte_brut(&z)))
}

async fn caisse_encaisser(State(e): State<Etat>, a: Auth, Json(x): Json<caisse::Encaissement>) -> Rep<caisse::ResultatEncaissement> {
    ecrire!(e, a, |db| caisse::encaisser(db, &a, &x))
}

async fn caisse_mouvement(State(e): State<Etat>, a: Auth, Json(m): Json<caisse::MouvementCaisse>) -> Rep<String> {
    ecrire!(e, a, |db| caisse::mouvement_caisse(db, &a, &m))
}

async fn caisse_transfert(State(e): State<Etat>, a: Auth, Json(t): Json<caisse::Transfert>) -> Rep<()> {
    ecrire!(e, a, |db| caisse::transferer(db, &a, &t))
}

async fn caisse_mouvements(State(e): State<Etat>, a: Auth, Query(p): Q) -> Rep<Vec<caisse::MouvementLu>> {
    let session = q(&p, "session").map(str::to_owned);
    let compte = q(&p, "compte").map(str::to_owned);
    lire!(e, a, AUCUNE, |db| caisse::mouvements(db.conn(), session.as_deref(), compte.as_deref(), 300))
}

async fn paiement_annuler(State(e): State<Etat>, a: Auth, Path(id): Path<String>, Json(m): Json<Motif>) -> Rep<String> {
    ecrire!(e, a, |db| caisse::annuler_paiement(db, &a, &id, &m.motif))
}

async fn comptes(State(e): State<Etat>, a: Auth) -> Rep<Vec<caisse::Compte>> {
    lire!(e, a, AUCUNE, |db| caisse::lister_comptes(db.conn()))
}

async fn compte_enregistrer(State(e): State<Etat>, a: Auth, Json(c): Json<caisse::Compte>) -> Rep<String> {
    ecrire!(e, a, |db| caisse::enregistrer_compte(db, &a, &c))
}

async fn depenses(State(e): State<Etat>, a: Auth, Query(p): Q) -> Rep<Vec<caisse::DepenseLue>> {
    let jid = q(&p, "journee").map(str::to_owned);
    lire!(e, a, Some(perm::DEPENSE_CREER), |db| {
        let j = match jid {
            Some(j) => j,
            None => match journee::ouverte(db.conn())? {
                Some(j) => j.id,
                None => return Ok(vec![]),
            },
        };
        caisse::depenses_journee(db.conn(), &j)
    })
}

async fn depense_creer(State(e): State<Etat>, a: Auth, Json(d): Json<caisse::NouvelleDepense>) -> Rep<String> {
    ecrire!(e, a, |db| caisse::depenser(db, &a, &d))
}

async fn depense_annuler(State(e): State<Etat>, a: Auth, Path(id): Path<String>, Json(m): Json<Motif>) -> Rep<()> {
    ecrire!(e, a, |db| caisse::annuler_depense(db, &a, &id, &m.motif))
}

async fn depenses_categories(State(e): State<Etat>, a: Auth) -> Rep<Value> {
    lire!(e, a, AUCUNE, |db| Ok(json!(caisse::categories_depense(db.conn())?
        .into_iter()
        .map(|(id, nom)| json!({ "id": id, "nom": nom }))
        .collect::<Vec<_>>())))
}

#[derive(Deserialize)]
struct Nom {
    nom: String,
}

async fn depense_categorie_creer(State(e): State<Etat>, a: Auth, Json(n): Json<Nom>) -> Rep<String> {
    ecrire!(e, a, |db| caisse::ajouter_categorie_depense(db, &a, &n.nom))
}

async fn mobile_money(State(e): State<Etat>, a: Auth, Query(p): Q) -> Rep<Vec<caisse::PartMobileMoney>> {
    let statut = q(&p, "statut").map(str::to_owned);
    lire!(e, a, Some(perm::CAISSE_ENCAISSER), |db| caisse::parts_mobile_money(db.conn(), statut.as_deref()))
}

#[derive(Deserialize)]
struct Verification {
    statut: String,
    #[serde(default)]
    note: String,
}

async fn mobile_money_verifier(State(e): State<Etat>, a: Auth, Path(id): Path<String>, Json(v): Json<Verification>) -> Rep<()> {
    ecrire!(e, a, |db| caisse::verifier_mobile_money(db, &a, &id, &v.statut, &v.note))
}

// ───────────── Stock et achats ─────────────

async fn stock_niveaux(State(e): State<Etat>, a: Auth) -> Rep<Vec<stock::NiveauStock>> {
    lire!(e, a, Some(perm::STOCK_VOIR), |db| stock::niveaux(db.conn()))
}

async fn article_enregistrer(State(e): State<Etat>, a: Auth, Json(x): Json<stock::Article>) -> Rep<String> {
    ecrire!(e, a, |db| stock::enregistrer_article(db, &a, &x))
}

async fn article_mouvements(State(e): State<Etat>, a: Auth, Path(id): Path<String>) -> Rep<Value> {
    lire!(e, a, Some(perm::STOCK_VOIR), |db| Ok(json!({
        "mouvements": stock::mouvements_article(db.conn(), &id, 200)?,
        "prix_achat": achats::historique_prix_achat(db.conn(), &id)?,
    })))
}

async fn stock_mouvement(State(e): State<Etat>, a: Auth, Json(m): Json<stock::MouvementManuel>) -> Rep<String> {
    ecrire!(e, a, |db| stock::mouvement_manuel(db, &a, &m))
}

async fn inventaires(State(e): State<Etat>, a: Auth) -> Rep<Value> {
    lire!(e, a, Some(perm::STOCK_INVENTAIRE), |db| Ok(json!(stock::lister_inventaires(db.conn())?
        .into_iter()
        .map(|(id, libelle, statut, cree_le)| json!({ "id": id, "libelle": libelle, "statut": statut, "cree_le": cree_le }))
        .collect::<Vec<_>>())))
}

#[derive(Deserialize)]
struct Libelle {
    #[serde(default)]
    libelle: String,
}

async fn inventaire_creer(State(e): State<Etat>, a: Auth, Json(l): Json<Libelle>) -> Rep<String> {
    ecrire!(e, a, |db| stock::creer_inventaire(db, &a, &l.libelle))
}

async fn inventaire_detail(State(e): State<Etat>, a: Auth, Path(id): Path<String>) -> Rep<stock::Inventaire> {
    lire!(e, a, Some(perm::STOCK_INVENTAIRE), |db| stock::inventaire(db.conn(), &id))
}

#[derive(Deserialize)]
struct Comptage {
    article_id: String,
    compte: i64,
}

async fn inventaire_comptage(State(e): State<Etat>, a: Auth, Path(id): Path<String>, Json(c): Json<Comptage>) -> Rep<()> {
    ecrire!(e, a, |db| stock::saisir_comptage(db, &a, &id, &c.article_id, c.compte))
}

async fn inventaire_valider(State(e): State<Etat>, a: Auth, Path(id): Path<String>) -> Rep<()> {
    ecrire!(e, a, |db| stock::valider_inventaire(db, &a, &id))
}

async fn inventaire_abandonner(State(e): State<Etat>, a: Auth, Path(id): Path<String>) -> Rep<()> {
    ecrire!(e, a, |db| stock::abandonner_inventaire(db, &a, &id))
}

async fn fournisseurs(State(e): State<Etat>, a: Auth) -> Rep<Vec<achats::Fournisseur>> {
    lire!(e, a, Some(perm::ACHAT_GERER), |db| achats::lister_fournisseurs(db.conn()))
}

async fn fournisseur_enregistrer(State(e): State<Etat>, a: Auth, Json(f): Json<achats::Fournisseur>) -> Rep<String> {
    ecrire!(e, a, |db| achats::enregistrer_fournisseur(db, &a, &f))
}

async fn fournisseur_reglement(State(e): State<Etat>, a: Auth, Json(r): Json<achats::ReglementFournisseur>) -> Rep<String> {
    ecrire!(e, a, |db| achats::regler(db, &a, &r))
}

async fn achats_lister(State(e): State<Etat>, a: Auth) -> Rep<Vec<achats::AchatLu>> {
    lire!(e, a, Some(perm::ACHAT_GERER), |db| achats::lister_achats(db.conn(), 100))
}

async fn achat_receptionner(State(e): State<Etat>, a: Auth, Json(x): Json<achats::NouvelAchat>) -> Rep<String> {
    ecrire!(e, a, |db| achats::receptionner(db, &a, &x))
}

// ───────────── Clients ─────────────

async fn clients_rechercher(State(e): State<Etat>, a: Auth, Query(p): Q) -> Rep<Vec<clients::Client>> {
    let texte = q(&p, "q").unwrap_or("").to_owned();
    lire!(e, a, AUCUNE, |db| clients::rechercher(db.conn(), &texte))
}

async fn client_enregistrer(State(e): State<Etat>, a: Auth, Json(c): Json<clients::Client>) -> Rep<String> {
    ecrire!(e, a, |db| clients::enregistrer(db, &a, &c))
}

async fn client_detail(State(e): State<Etat>, a: Auth, Path(id): Path<String>) -> Rep<Value> {
    lire!(e, a, AUCUNE, |db| Ok(json!({ "client": clients::client(db.conn(), &id)?, "releve": clients::releve(db.conn(), &id)? })))
}

async fn client_reglement(State(e): State<Etat>, a: Auth, Json(r): Json<clients::Reglement>) -> Rep<String> {
    ecrire!(e, a, |db| clients::regler(db, &a, &r))
}

// ───────────── Employés et paie ─────────────

async fn employes_lister(State(e): State<Etat>, a: Auth, Query(p): Q) -> Rep<Vec<employes::Employe>> {
    let partis = q(&p, "partis") == Some("1");
    lire!(e, a, Some(perm::EMPLOYE_VOIR), |db| employes::lister(db.conn(), partis))
}

async fn employes_references(_a: Auth) -> Rep<Value> {
    Ok(Json(json!({
        "contrats": employes::TYPES_CONTRAT,
        "remunerations": employes::TYPES_REMUNERATION,
        "fonctions": employes::FONCTIONS,
        "presences": employes::STATUTS_PRESENCE,
    })))
}

async fn employe_enregistrer(State(e): State<Etat>, a: Auth, Json(x): Json<employes::Employe>) -> Rep<String> {
    ecrire!(e, a, |db| employes::enregistrer(db, &a, &x))
}

async fn employe_detail(State(e): State<Etat>, a: Auth, Path(id): Path<String>) -> Rep<Value> {
    lire!(e, a, Some(perm::EMPLOYE_VOIR), |db| Ok(json!({
        "employe": employes::employe(db.conn(), &id)?,
        "releve": employes::releve(db.conn(), &id)?,
        "bulletins": paie::bulletins(db.conn(), Some(&id))?,
    })))
}

async fn employe_avance(State(e): State<Etat>, a: Auth, Json(x): Json<employes::Avance>) -> Rep<String> {
    ecrire!(e, a, |db| employes::avance(db, &a, &x))
}

async fn employe_evenement(State(e): State<Etat>, a: Auth, Json(x): Json<employes::Evenement>) -> Rep<String> {
    ecrire!(e, a, |db| employes::evenement(db, &a, &x))
}

async fn presences(State(e): State<Etat>, a: Auth, Query(p): Q) -> Rep<Vec<employes::Presence>> {
    let debut = q(&p, "debut").unwrap_or("0000-00-00").to_owned();
    let fin = q(&p, "fin").unwrap_or("9999-99-99").to_owned();
    lire!(e, a, Some(perm::EMPLOYE_PRESENCE), |db| employes::presences(db.conn(), None, &debut, &fin))
}

async fn presences_pointer(State(e): State<Etat>, a: Auth, Json(s): Json<Vec<employes::SaisiePresence>>) -> Rep<()> {
    ecrire!(e, a, |db| employes::pointer(db, &a, &s))
}

async fn paie_apercu(State(e): State<Etat>, a: Auth, Query(p): Q) -> Rep<paie::Bulletin> {
    let (id, debut, fin) = (
        q(&p, "employe").unwrap_or("").to_owned(),
        q(&p, "debut").unwrap_or("").to_owned(),
        q(&p, "fin").unwrap_or("").to_owned(),
    );
    lire!(e, a, Some(perm::PAIE_GERER), |db| paie::apercu(db.conn(), &id, &debut, &fin))
}

#[derive(Deserialize)]
struct Periode {
    employe_id: String,
    debut: String,
    fin: String,
}

async fn paie_cloturer(State(e): State<Etat>, a: Auth, Json(p): Json<Periode>) -> Rep<paie::Bulletin> {
    ecrire!(e, a, |db| paie::cloturer(db, &a, &p.employe_id, &p.debut, &p.fin))
}

async fn paie_payer(State(e): State<Etat>, a: Auth, Json(p): Json<paie::PaiementSalaire>) -> Rep<String> {
    ecrire!(e, a, |db| paie::payer(db, &a, &p))
}

async fn paie_bulletins(State(e): State<Etat>, a: Auth, Query(p): Q) -> Rep<Vec<paie::Bulletin>> {
    let employe = q(&p, "employe").map(str::to_owned);
    lire!(e, a, Some(perm::PAIE_GERER), |db| paie::bulletins(db.conn(), employe.as_deref()))
}

async fn paie_bulletin(State(e): State<Etat>, a: Auth, Path(id): Path<String>) -> Rep<paie::Bulletin> {
    lire!(e, a, Some(perm::PAIE_GERER), |db| paie::bulletin(db.conn(), &id))
}

// ───────────── Livraison ─────────────

async fn livraisons(State(e): State<Etat>, a: Auth) -> Rep<Vec<commandes::CommandeResume>> {
    lire!(e, a, Some(perm::LIVRAISON_GERER), |db| {
        let Some(j) = journee::ouverte(db.conn())? else { return Ok(vec![]) };
        Ok(commandes::lister(db.conn(), &j.id, None)?.into_iter().filter(|c| c.type_ == "livraison").collect())
    })
}

#[derive(Deserialize)]
struct Livreur {
    livreur_id: String,
}

async fn livraison_assigner(State(e): State<Etat>, a: Auth, Path(id): Path<String>, Json(l): Json<Livreur>) -> Rep<()> {
    ecrire!(e, a, |db| livraison::assigner(db, &a, &id, &l.livreur_id))
}

#[derive(Deserialize)]
struct StatutLivraison {
    statut: String,
    #[serde(default)]
    motif: String,
}

async fn livraison_statut(State(e): State<Etat>, a: Auth, Path(id): Path<String>, Json(s): Json<StatutLivraison>) -> Rep<()> {
    ecrire!(e, a, |db| livraison::changer_statut(db, &a, &id, &s.statut, &s.motif))
}

async fn livreurs(State(e): State<Etat>, a: Auth) -> Rep<Vec<livraison::SoldeLivreur>> {
    lire!(e, a, Some(perm::LIVRAISON_GERER), |db| livraison::soldes_livreurs(db.conn()))
}

#[derive(Deserialize)]
struct Remise {
    remis: i64,
    #[serde(default)]
    motif: String,
}

async fn livreur_remise(State(e): State<Etat>, a: Auth, Path(id): Path<String>, Json(r): Json<Remise>) -> Rep<livraison::ResultatRemise> {
    ecrire!(e, a, |db| livraison::remise_livreur(db, &a, &id, r.remis, &r.motif))
}

// ───────────── Rapports ─────────────

async fn tableau_de_bord(State(e): State<Etat>, a: Auth) -> Rep<rapports::TableauDeBord> {
    lire!(e, a, Some(perm::RAPPORT_VOIR), |db| rapports::tableau_de_bord(db.conn()))
}

async fn rapport_periode(State(e): State<Etat>, a: Auth, Query(p): Q) -> Result<Response, ApiErreur> {
    let uid = a.utilisateur_id.clone();
    let eleve = a.acteur.eleve;
    let (debut, fin) = (q(&p, "debut").map(str::to_owned), q(&p, "fin").map(str::to_owned));
    let csv = q(&p, "format") == Some("csv");
    let r = e
        .avec_db(move |db| {
            peut(db, &uid, eleve, perm::RAPPORT_VOIR)?;
            let jour = aujourdhui(db);
            rapports::rapport_periode(db.conn(), debut.as_deref().unwrap_or(&jour), fin.as_deref().unwrap_or(&jour))
        })
        .await?;
    Ok(if csv { csv_reponse(&r) } else { Json(r).into_response() })
}

fn csv_reponse(r: &rapports::Rapport) -> Response {
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8".to_string()),
            (header::CONTENT_DISPOSITION, format!("attachment; filename=\"youma-{}-{}.csv\"", r.debut, r.fin)),
        ],
        rapports::en_csv(r),
    )
        .into_response()
}

async fn rapport_stock(State(e): State<Etat>, a: Auth, Query(p): Q) -> Result<Response, ApiErreur> {
    let uid = a.utilisateur_id.clone();
    let eleve = a.acteur.eleve;
    let csv = q(&p, "format") == Some("csv");
    let r = e.avec_db(move |db| { peut(db, &uid, eleve, perm::STOCK_VOIR)?; rapports::rapport_stock(db.conn()) }).await?;
    Ok(if csv { csv_reponse(&r) } else { Json(r).into_response() })
}

async fn rapport_dettes(State(e): State<Etat>, a: Auth, Query(p): Q) -> Result<Response, ApiErreur> {
    let uid = a.utilisateur_id.clone();
    let eleve = a.acteur.eleve;
    let csv = q(&p, "format") == Some("csv");
    let r = e
        .avec_db(move |db| { peut(db, &uid, eleve, perm::RAPPORT_VOIR)?; rapports::rapport_dettes(db.conn(), db.maintenant()) })
        .await?;
    Ok(if csv { csv_reponse(&r) } else { Json(r).into_response() })
}

async fn audit(State(e): State<Etat>, a: Auth, Query(p): Q) -> Rep<Vec<parametres::LigneAudit>> {
    let action = q(&p, "action").map(str::to_owned);
    lire!(e, a, Some(perm::AUDIT_VOIR), |db| parametres::journal_audit(db.conn(), action.as_deref(), 500))
}

// ───────────── Administration ─────────────

async fn parametres_lire(State(e): State<Etat>, a: Auth) -> Rep<parametres::Parametres> {
    lire!(e, a, AUCUNE, |db| parametres::lire(db.conn()))
}

async fn parametres_modifier(State(e): State<Etat>, a: Auth, Json(p): Json<parametres::Parametres>) -> Rep<()> {
    ecrire!(e, a, |db| parametres::modifier(db, &a, &p))
}

async fn restaurant_lire(State(e): State<Etat>, a: Auth) -> Rep<parametres::Restaurant> {
    lire!(e, a, AUCUNE, |db| parametres::restaurant(db.conn()))
}

async fn restaurant_modifier(State(e): State<Etat>, a: Auth, Json(r): Json<parametres::Restaurant>) -> Rep<()> {
    ecrire!(e, a, |db| parametres::modifier_restaurant(db, &a, &r))
}

async fn roles(State(e): State<Etat>, a: Auth) -> Rep<Value> {
    lire!(e, a, Some(perm::UTILISATEUR_GERER), |db| Ok(json!({ "roles": auth::lister_roles(db.conn())?, "permissions": perm::TOUTES })))
}

async fn role_modifier(State(e): State<Etat>, a: Auth, Json(r): Json<auth::ModifRole>) -> Rep<()> {
    ecrire!(e, a, |db| auth::modifier_role(db, &a, &r))
}

async fn utilisateurs(State(e): State<Etat>, a: Auth) -> Rep<Vec<auth::UtilisateurResume>> {
    lire!(e, a, Some(perm::UTILISATEUR_GERER), |db| auth::lister_utilisateurs(db.conn(), false))
}

async fn utilisateur_creer(State(e): State<Etat>, a: Auth, Json(u): Json<auth::NouvelUtilisateur>) -> Rep<String> {
    ecrire!(e, a, |db| auth::creer_utilisateur(db, &a, &u))
}

async fn utilisateur_modifier(State(e): State<Etat>, a: Auth, Path(id): Path<String>, Json(mut m): Json<auth::ModifUtilisateur>) -> Rep<()> {
    m.id = id;
    ecrire!(e, a, |db| auth::modifier_utilisateur(db, &a, &m))
}

async fn appareils_lister(State(e): State<Etat>, a: Auth) -> Rep<Vec<appareils::Appareil>> {
    lire!(e, a, Some(perm::APPAREIL_GERER), |db| appareils::lister(db.conn()))
}

async fn appareil_code(State(e): State<Etat>, a: Auth) -> Rep<appareils::CodeAppairage> {
    ecrire!(e, a, |db| appareils::generer_code(db, &a))
}

#[derive(Deserialize)]
struct Appairage {
    code: String,
    #[serde(default)]
    nom: String,
    #[serde(default = "navigateur")]
    #[serde(rename = "type")]
    type_: String,
}

fn navigateur() -> String {
    "navigateur".into()
}

/// Public : c'est justement l'étape qui rend l'appareil autorisé.
async fn appareil_appairer(State(e): State<Etat>, Json(x): Json<Appairage>) -> Rep<Value> {
    let (id, jeton) = e.avec_db(move |db| appareils::appairer(db, &x.code, &x.nom, &x.type_)).await?;
    Ok(Json(json!({ "id": id, "jeton": jeton })))
}

async fn appareil_revoquer(State(e): State<Etat>, a: Auth, Path(id): Path<String>) -> Rep<()> {
    ecrire!(e, a, |db| appareils::revoquer(db, &a, &id))
}

/// Adresses à afficher (et à mettre en QR) pour connecter les téléphones.
async fn reseau(State(e): State<Etat>, _a: Auth) -> Rep<Value> {
    let port = e.config.port;
    let ip = adresse_locale();
    Ok(Json(json!({
        "actif": e.config.reseau,
        "port": port,
        "adresses": ip.map(|ip| vec![format!("http://{ip}:{port}/")]).unwrap_or_default(),
    })))
}

/// Adresse IP locale (sans envoyer de paquet : `connect` UDP ne fait que choisir la route).
fn adresse_locale() -> Option<std::net::IpAddr> {
    let s = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    s.connect("192.168.1.1:80").or_else(|_| s.connect("10.0.0.1:80")).ok()?;
    s.local_addr().ok().map(|a| a.ip()).filter(|ip| !ip.is_unspecified())
}

async fn diagnostic(State(e): State<Etat>, a: Auth) -> Rep<Value> {
    let config = e.config.clone();
    let connectes = e.evenements.receiver_count();
    lire!(e, a, Some(perm::SAUVEGARDE_GERER), |db| {
        let chemin = db.chemin().map(|c| c.to_path_buf()).unwrap_or_default();
        let taille = std::fs::metadata(&chemin).map(|m| m.len()).unwrap_or(0);
        Ok(json!({
            "version": env!("CARGO_PKG_VERSION"),
            "schema": youma_core::db::version_schema(),
            "base": chemin,
            "taille_base": taille,
            "sauvegardes": sauvegarde::etat(db.conn(), &config.dossier_sauvegardes(), db.maintenant())?,
            "postes_connectes": connectes,
            "appareils": appareils::lister(db.conn())?,
            "horloge": db.etat_horloge()?,
            "mode": if config.reseau { "réseau local" } else { "mono-poste" },
            "demo": demo::est_demo(db.conn()),
        }))
    })
}

async fn sauvegardes(State(e): State<Etat>, a: Auth) -> Rep<Vec<sauvegarde::Sauvegarde>> {
    let dossier = e.config.dossier_sauvegardes();
    lire!(e, a, Some(perm::SAUVEGARDE_GERER), |db| sauvegarde::lister(&dossier))
}

async fn sauvegarde_creer(State(e): State<Etat>, a: Auth) -> Rep<sauvegarde::Sauvegarde> {
    let dossier = e.config.dossier_sauvegardes();
    let uid = a.utilisateur_id.clone();
    let eleve = a.acteur.eleve;
    let r = e.avec_db(move |db| { peut(db, &uid, eleve, perm::SAUVEGARDE_GERER)?; sauvegarde::sauvegarder(db, &dossier, "manuelle") }).await?;
    Ok(Json(r))
}

#[derive(Deserialize)]
struct Dossier {
    chemin: String,
}

async fn sauvegarde_exporter(State(e): State<Etat>, a: Auth, Json(d): Json<Dossier>) -> Rep<sauvegarde::Sauvegarde> {
    ecrire!(e, a, |db| sauvegarde::exporter(db, &a, &PathBuf::from(&d.chemin)))
}

async fn sauvegarde_restaurer(State(e): State<Etat>, a: Auth, Json(d): Json<Dossier>) -> Rep<()> {
    let dossier = e.config.dossier_sauvegardes();
    ecrire!(e, a, |db| sauvegarde::restaurer(db, &a, &PathBuf::from(&d.chemin), &dossier))
}

async fn integrite(State(e): State<Etat>, a: Auth) -> Rep<sauvegarde::RapportIntegrite> {
    lire!(e, a, Some(perm::SAUVEGARDE_GERER), |db| sauvegarde::verifier_integrite(db.conn(), true, db.maintenant()))
}

async fn licence_etat(State(e): State<Etat>, a: Auth) -> Rep<licence::EtatLicence> {
    lire!(e, a, AUCUNE, |db| licence::etat(db.conn(), &aujourdhui(db)))
}

async fn licence_installer(State(e): State<Etat>, a: Auth, corps: Bytes) -> Rep<licence::Licence> {
    let texte = String::from_utf8_lossy(&corps).to_string();
    ecrire!(e, a, |db| licence::installer(db, &a, &texte))
}

// ───────────── Commandes reçues, zones à risque (fiche 0013) ─────────────

async fn entrantes_file(State(e): State<Etat>, a: Auth) -> Rep<Vec<entrantes::Entrante>> {
    lire!(e, a, Some(perm::COMMANDE_VALIDER_ENTRANTE), |db| entrantes::file(db.conn()))
}

#[derive(Deserialize)]
struct Validation {
    accepter: bool,
    #[serde(default)]
    motif: String,
}

async fn entrante_valider(State(e): State<Etat>, a: Auth, Path(id): Path<String>, Json(v): Json<Validation>) -> Rep<()> {
    ecrire!(e, a, |db| entrantes::valider(db, &a, &id, v.accepter, &v.motif))
}

async fn zones_risque_lister(State(e): State<Etat>, a: Auth) -> Rep<Vec<zones_risque::ZoneRisque>> {
    lire!(e, a, Some(perm::ZONE_OUTREPASSER), |db| zones_risque::lister(db.conn()))
}

async fn zone_risque_enregistrer(State(e): State<Etat>, a: Auth, Json(z): Json<zones_risque::ZoneRisque>) -> Rep<String> {
    ecrire!(e, a, |db| zones_risque::enregistrer(db, &a, &z))
}

async fn numeros_bloques(State(e): State<Etat>, a: Auth) -> Rep<Value> {
    lire!(e, a, Some(perm::ZONE_OUTREPASSER), |db| {
        Ok(json!(zones_risque::numeros_bloques(db.conn())?
            .into_iter()
            .map(|(t, m, c)| json!({ "telephone": t, "motif": m, "cree_le": c }))
            .collect::<Vec<_>>()))
    })
}

#[derive(Deserialize)]
struct NumeroBloque {
    telephone: String,
    #[serde(default)]
    motif: String,
}

async fn numero_bloquer(State(e): State<Etat>, a: Auth, Json(n): Json<NumeroBloque>) -> Rep<()> {
    ecrire!(e, a, |db| zones_risque::bloquer_numero(db, &a, &n.telephone, &n.motif))
}

async fn numero_debloquer(State(e): State<Etat>, a: Auth, Json(n): Json<NumeroBloque>) -> Rep<()> {
    ecrire!(e, a, |db| zones_risque::debloquer_numero(db, &a, &n.telephone))
}

async fn codes_qr(State(e): State<Etat>, a: Auth) -> Rep<Value> {
    lire!(e, a, Some(perm::SALLE_GERER), |db| {
        Ok(json!(entrantes::codes_qr(db.conn())?
            .into_iter()
            .map(|(id, nom, code)| json!({ "table_id": id, "nom": nom, "code": code }))
            .collect::<Vec<_>>()))
    })
}

#[derive(Deserialize)]
struct GenerationQr {
    #[serde(default)]
    regenerer: bool,
}

async fn codes_qr_generer(State(e): State<Etat>, a: Auth, Json(g): Json<GenerationQr>) -> Rep<usize> {
    ecrire!(e, a, |db| entrantes::generer_codes_qr(db, &a, g.regenerer))
}

async fn commande_liens(State(e): State<Etat>, a: Auth, Path(id): Path<String>) -> Rep<entrantes::Liens> {
    ecrire!(e, a, |db| entrantes::liens(db, &a, &id))
}

async fn relais_etat(State(e): State<Etat>, a: Auth) -> Rep<crate::relais::EtatRelais> {
    let _ = &a;
    Ok(Json(e.relais.lock().map(|r| r.clone()).unwrap_or_default()))
}

async fn public_menu(State(e): State<Etat>, Query(p): Q) -> Rep<entrantes::MenuPublic> {
    let table = q(&p, "table").map(str::to_owned);
    let m = e
        .avec_db(move |db| {
            let m = entrantes::menu_public(db.conn(), table.as_deref())?;
            let actif = if table.is_some() { m.qr_table } else { m.en_ligne };
            if !actif {
                return Err(Erreur::Interdit("Commande à distance non activée dans ce restaurant".into()));
            }
            Ok(m)
        })
        .await?;
    Ok(Json(m))
}

async fn public_commande(State(e): State<Etat>, Json(c): Json<entrantes::CommandeEntrante>) -> Rep<entrantes::Reponse> {
    // Le relais Internet seul peut affirmer qu'un numéro a été vérifié par SMS.
    let mut c = c;
    c.telephone_verifie = false;
    c.origine_id = None;
    c.code_suivi = None;
    Ok(Json(e.avec_db(move |db| entrantes::recevoir(db, &c)).await?))
}

async fn public_suivi(State(e): State<Etat>, Path(code): Path<String>) -> Rep<entrantes::Suivi> {
    Ok(Json(e.avec_db(move |db| entrantes::suivi(db.conn(), &code)).await?))
}

#[derive(Deserialize)]
struct Position {
    lat: i64,
    lon: i64,
}

async fn public_position(State(e): State<Etat>, Path(code): Path<String>, Json(p): Json<Position>) -> Rep<()> {
    Ok(Json(e.avec_db(move |db| entrantes::ajouter_position(db, &code, p.lat, p.lon)).await?))
}
