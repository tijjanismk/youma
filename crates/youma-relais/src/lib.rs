//! Serveur relais Internet **facultatif** (fiche 0013).
//!
//! Le poste central reste la seule source de vérité : c'est lui qui appelle le relais (il est derrière la box,
//! sans adresse publique). Il y publie son menu, ses réglages et l'état des commandes suivies ; il y reprend
//! les commandes en ligne et les positions des livreurs. Le relais ne fait que transmettre :
//! s'il disparaît, le restaurant continue de vendre normalement.
//!
//! Routes publiques (mêmes chemins que le poste central, l'interface est la même) :
//! `GET /api/public/menu`, `POST /api/public/verification`, `POST /api/public/commandes`,
//! `GET /api/public/suivi/{code}`, `POST /api/public/position/{code_livreur}`.
//! Route du poste central : `POST /api/relais/synchroniser` (en-tête `Authorization: Bearer <clé>`).

use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use axum::extract::{ConnectInfo, Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use base64::Engine;
use rand::Rng;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tower_http::services::{ServeDir, ServeFile};
use youma_core::entrantes::{code_aleatoire, empreinte_panier, FENETRE_DOUBLON_MS};
use youma_core::zones_risque::normaliser_telephone;

pub mod cloud;
pub mod sms;
pub use sms::{FournisseurSms, Orange, WhatsApp};

/// Au-delà, le poste central est considéré injoignable : le menu s'affiche « fermé ».
pub const SILENCE_MAX_MS: i64 = 90_000;
/// Une commande transmise sans résultat est renvoyée après ce délai (le poste est idempotent).
pub const RENVOI_MS: i64 = 60_000;

#[derive(Clone, Debug)]
pub struct Config {
    pub dossier_donnees: PathBuf,
    pub port: u16,
    /// Clé partagée avec le poste central (Administration → Commandes à distance).
    pub cle: String,
    pub dossier_ui: Option<PathBuf>,
    /// Derrière un proxy HTTPS (Caddy, nginx) : l'adresse du client est dans `X-Forwarded-For`.
    pub derriere_proxy: bool,
    /// Orange Mali, ou simulation tant que le contrat n'est pas signé.
    pub sms: FournisseurSms,
    /// WhatsApp Cloud, second canal des codes au choix du client (fiche 0046).
    pub whatsapp: Option<WhatsApp>,
}

#[derive(Clone)]
pub struct Etat {
    db: Arc<Mutex<Connection>>,
    empreinte_cle: [u8; 32],
    sms: Arc<sms::Envoyeur>,
    limites: Arc<Mutex<HashMap<String, Vec<i64>>>>,
    derriere_proxy: bool,
    /// Sauvegardes chiffrées reçues (cloud, fiche 0018).
    dossier: PathBuf,
    /// Espace propriétaire : jeton → (restaurants accessibles, expiration).
    sessions: Arc<Mutex<HashMap<String, SessionProprietaire>>>,
    /// Relais partagé (fiche 0049) : base de chaque restaurant inscrit, ouverte à la première demande.
    bases: Arc<Mutex<HashMap<String, Arc<Mutex<Connection>>>>>,
    /// Restaurant servi (`/r/<slug>`) ; aucun pour le restaurant historique du relais (adresses à la racine).
    restaurant: Option<String>,
}

/// En-tête interne posé par l'aiguillage des adresses `/r/<slug>/…` (jamais accepté du client).
const ENTETE_RESTAURANT: &str = "x-youma-restaurant";

fn slug_valide(s: &str) -> bool {
    (2..=40).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// Relais partagé (fiche 0049) : `/r/<slug>/api/public/menu` est servi comme `/api/public/menu` pour ce restaurant ;
/// les pages `/r/<slug>/menu`, `/r/<slug>/suivi/…` reçoivent la même interface.
fn aiguiller(mut req: axum::extract::Request) -> axum::extract::Request {
    req.headers_mut().remove(ENTETE_RESTAURANT);
    let Some(reste) = req.uri().path().strip_prefix("/r/") else { return req };
    let (slug, suite) = match reste.find('/') {
        Some(i) => (reste[..i].to_string(), reste[i..].to_string()),
        None => (reste.to_string(), "/".to_string()),
    };
    if !slug_valide(&slug) {
        return req;
    }
    let cible = match req.uri().query() {
        Some(q) => format!("{suite}?{q}"),
        None => suite,
    };
    if let (Ok(uri), Ok(v)) = (cible.parse(), axum::http::HeaderValue::from_str(&slug)) {
        *req.uri_mut() = uri;
        req.headers_mut().insert(ENTETE_RESTAURANT, v);
    }
    req
}

/// État du restaurant de la requête : celui du relais (racine) ou un restaurant inscrit (`/r/<slug>`).
pub struct Resto(Etat);

impl axum::extract::FromRequestParts<Etat> for Resto {
    type Rejection = Echec;

    async fn from_request_parts(parts: &mut axum::http::request::Parts, e: &Etat) -> Result<Self, Echec> {
        match parts.headers.get(ENTETE_RESTAURANT).and_then(|v| v.to_str().ok()) {
            None => Ok(Resto(e.clone())),
            Some(slug) => e.pour_restaurant(slug).map(Resto),
        }
    }
}

fn base_restaurant(chemin: &std::path::Path) -> rusqlite::Result<Connection> {
    let conn = Connection::open(chemin)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.execute_batch(SCHEMA)?;
    Ok(conn)
}

fn octets_hex(h: &str) -> Option<[u8; 32]> {
    let mut o = [0u8; 32];
    if h.len() != 64 {
        return None;
    }
    for (i, b) in o.iter_mut().enumerate() {
        *b = u8::from_str_radix(h.get(2 * i..2 * i + 2)?, 16).ok()?;
    }
    Some(o)
}

/// Restaurants accessibles et expiration (ms) d'une session de l'espace propriétaire.
type SessionProprietaire = (Vec<String>, i64);

/// Erreur au format de l'API du poste central (`code`, `message`).
pub struct Echec {
    statut: StatusCode,
    code: &'static str,
    message: String,
}

impl IntoResponse for Echec {
    fn into_response(self) -> Response {
        (self.statut, Json(json!({ "code": self.code, "message": self.message }))).into_response()
    }
}

type Rep = Result<Json<Value>, Echec>;

fn maintenant() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

fn empreinte(s: &str) -> [u8; 32] {
    Sha256::digest(s.as_bytes()).into()
}

fn erreur(statut: StatusCode, code: &'static str, message: impl Into<String>) -> Echec {
    Echec { statut, code, message: message.into() }
}

fn interne(e: impl std::fmt::Display) -> Echec {
    tracing::error!("relais : {e}");
    erreur(StatusCode::INTERNAL_SERVER_ERROR, "ERREUR", "Erreur du relais")
}

fn refus(message: &str) -> Json<Value> {
    Json(json!({ "statut": "refusee", "message": message, "numero": null, "code_suivi": null, "total": 0 }))
}

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS etat (cle TEXT PRIMARY KEY, valeur TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS commandes (
    origine_id TEXT PRIMARY KEY, code_suivi TEXT NOT NULL UNIQUE, corps TEXT NOT NULL,
    cree_le INTEGER NOT NULL, transmise_le INTEGER, resultat TEXT);
CREATE TABLE IF NOT EXISTS suivis (code_suivi TEXT PRIMARY KEY, code_livreur TEXT, corps TEXT NOT NULL, maj INTEGER NOT NULL);
CREATE INDEX IF NOT EXISTS idx_suivis_livreur ON suivis(code_livreur);
CREATE TABLE IF NOT EXISTS positions (code_livreur TEXT PRIMARY KEY, lat INTEGER NOT NULL, lon INTEGER NOT NULL,
    ms INTEGER NOT NULL, transmise INTEGER NOT NULL DEFAULT 0);
CREATE TABLE IF NOT EXISTS modifications (code_suivi TEXT PRIMARY KEY, corps TEXT NOT NULL, ms INTEGER NOT NULL,
    transmise INTEGER NOT NULL DEFAULT 0);
CREATE TABLE IF NOT EXISTS avis (code_suivi TEXT PRIMARY KEY, note INTEGER NOT NULL, commentaire TEXT NOT NULL,
    ms INTEGER NOT NULL, transmis INTEGER NOT NULL DEFAULT 0);
CREATE TABLE IF NOT EXISTS verifications (telephone TEXT PRIMARY KEY, empreinte TEXT NOT NULL, expire INTEGER NOT NULL,
    tentatives INTEGER NOT NULL DEFAULT 0);
CREATE TABLE IF NOT EXISTS photos (empreinte TEXT PRIMARY KEY, type TEXT NOT NULL, octets BLOB NOT NULL);
CREATE TABLE IF NOT EXISTS livreurs (telephone TEXT PRIMARY KEY, nom TEXT NOT NULL, pin_hash TEXT NOT NULL,
    courses TEXT NOT NULL, maj INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS sessions_livreurs (empreinte TEXT PRIMARY KEY, telephone TEXT NOT NULL, pin_hash TEXT NOT NULL,
    expire INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS clients_verifies (empreinte TEXT PRIMARY KEY, telephone TEXT NOT NULL, cree_le INTEGER NOT NULL,
    utilise_le INTEGER NOT NULL);
";

impl Etat {
    pub fn ouvrir(config: &Config) -> rusqlite::Result<Etat> {
        std::fs::create_dir_all(&config.dossier_donnees).ok();
        let conn = Connection::open(config.dossier_donnees.join("youma-relais.db"))?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.execute_batch(SCHEMA)?;
        conn.execute_batch(cloud::SCHEMA)?;
        cloud::migrer(&conn)?;
        // Relais mis à jour : un menu déjà reçu avec ses photos intégrées est converti (fiche 0048).
        let ancien: Option<String> = conn.query_row("SELECT valeur FROM etat WHERE cle = 'menu'", [], |r| r.get(0)).optional()?;
        if let Some(mut m) = ancien.filter(|v| v.contains("\"data:")).and_then(|v| serde_json::from_str::<Value>(&v).ok()) {
            let tx = conn.unchecked_transaction()?;
            ranger_photos(&tx, &mut m)?;
            tx.execute("UPDATE etat SET valeur = ?1 WHERE cle = 'menu'", [m.to_string()])?;
            tx.commit()?;
        }
        Ok(Etat {
            db: Arc::new(Mutex::new(conn)),
            empreinte_cle: empreinte(&config.cle),
            sms: Arc::new(sms::Envoyeur::new(config.sms.clone(), config.whatsapp.clone())),
            limites: Arc::new(Mutex::new(HashMap::new())),
            derriere_proxy: config.derriere_proxy,
            dossier: config.dossier_donnees.clone(),
            sessions: Arc::new(Mutex::new(HashMap::new())),
            bases: Arc::new(Mutex::new(HashMap::new())),
            restaurant: None,
        })
    }

    /// Restaurant inscrit sur le relais partagé : sa base (`restaurants/<slug>.db`) et sa clé. Inconnu : 404.
    fn pour_restaurant(&self, slug: &str) -> Result<Etat, Echec> {
        let cle: Option<String> =
            self.avec(|c| c.query_row("SELECT empreinte_cle FROM cloud_restaurants WHERE slug = ?1", [slug], |r| r.get(0)).optional())?;
        let empreinte_cle = cle
            .as_deref()
            .and_then(octets_hex)
            .ok_or_else(|| erreur(StatusCode::NOT_FOUND, "NON_TROUVE", "Restaurant inconnu sur ce relais"))?;
        let db = {
            let mut bases = self.bases.lock().unwrap_or_else(|e| e.into_inner());
            match bases.get(slug) {
                Some(b) => b.clone(),
                None => {
                    let dossier = self.dossier.join("restaurants");
                    std::fs::create_dir_all(&dossier).map_err(interne)?;
                    let b = Arc::new(Mutex::new(base_restaurant(&dossier.join(format!("{slug}.db"))).map_err(interne)?));
                    bases.insert(slug.to_string(), b.clone());
                    b
                }
            }
        };
        Ok(Etat { db, empreinte_cle, restaurant: Some(slug.to_string()), ..self.clone() })
    }

    /// Début des adresses de ce restaurant : vide à la racine, `/r/<slug>` sur le relais partagé.
    fn prefixe(&self) -> String {
        self.restaurant.as_deref().map(|s| format!("/r/{s}")).unwrap_or_default()
    }

    fn avec<T>(&self, f: impl FnOnce(&Connection) -> rusqlite::Result<T>) -> Result<T, Echec> {
        let c = self.db.lock().unwrap_or_else(|e| e.into_inner());
        f(&c).map_err(interne)
    }

    fn lire(&self, cle: &str) -> Result<Option<Value>, Echec> {
        let v: Option<String> = self.avec(|c| c.query_row("SELECT valeur FROM etat WHERE cle = ?1", [cle], |r| r.get(0)).optional())?;
        Ok(v.and_then(|s| serde_json::from_str(&s).ok()))
    }

    /// Anti-abus : au plus `max` appels par `fenetre_ms` pour cette clé (adresse IP, numéro).
    fn limiter(&self, cle: &str, max: usize, fenetre_ms: i64) -> bool {
        let t = maintenant();
        let mut l = self.limites.lock().unwrap_or_else(|e| e.into_inner());
        if l.len() > 10_000 {
            l.retain(|_, v| v.iter().any(|x| t - x < 3_600_000));
        }
        let v = l.entry(cle.to_string()).or_default();
        v.retain(|x| t - x < fenetre_ms);
        if v.len() >= max {
            return false;
        }
        v.push(t);
        true
    }

    /// RG-CAN-08 : 15 codes de suivi ou de livreur inconnus en 15 minutes depuis une adresse : refus suivants.
    fn essais_epuises(&self, adresse: &str) -> bool {
        let t = maintenant();
        let mut l = self.limites.lock().unwrap_or_else(|e| e.into_inner());
        let v = l.entry(format!("essai:{adresse}")).or_default();
        v.retain(|x| t - x < ESSAIS_FENETRE_MS);
        v.len() >= ESSAIS_MAX
    }

    fn compter_essai(&self, adresse: &str) {
        let t = maintenant();
        self.limites.lock().unwrap_or_else(|e| e.into_inner()).entry(format!("essai:{adresse}")).or_default().push(t);
    }

    fn adresse(&self, entetes: &HeaderMap, ip: SocketAddr) -> String {
        if self.derriere_proxy {
            if let Some(v) = entetes.get("x-forwarded-for").and_then(|v| v.to_str().ok()) {
                return v.split(',').next().unwrap_or("").trim().to_string();
            }
        }
        ip.ip().to_string()
    }

    /// Le poste central s'est manifesté récemment.
    fn poste_joignable(&self) -> Result<bool, Echec> {
        let dernier = self.lire("dernier_contact")?.and_then(|v| v.as_i64()).unwrap_or(0);
        Ok(maintenant() - dernier < SILENCE_MAX_MS)
    }
}

pub fn routeur(etat: Etat, ui: Option<PathBuf>) -> Router {
    let mut app = Router::new()
        .route("/api/relais/synchroniser", post(synchroniser))
        .route("/api/etat", get(|State(e): State<Etat>| async move { Json(json!({ "relais": true, "sms": e.sms.nom() })) }))
        .route("/api/public/menu", get(menu))
        .route("/api/public/verification", post(verification))
        .route("/api/public/verification/confirmer", post(confirmer))
        .route("/api/public/commandes", post(commande))
        .route("/api/public/suivi/{code}", get(suivi))
        .route("/api/public/commandes/{code}/modifier", post(modifier))
        .route("/api/public/position/{code}", post(position))
        .route("/api/public/avis/{code}", post(avis))
        .route("/api/public/photos/{empreinte}", get(photo))
        .route("/api/public/livreur/connexion", post(connexion_livreur))
        .route("/api/public/livreur/courses", get(courses_livreur))
        .merge(cloud::routes())
        // Le relais ne sert que les pages du client et du livreur, jamais l'application du personnel.
        .route("/", get(|Resto(e): Resto| async move { Redirect::temporary(&format!("{}/menu", e.prefixe())) }));
    if let Some(ui) = ui {
        let index = ui.join("index.html");
        app = app.fallback_service(ServeDir::new(ui).fallback(ServeFile::new(index)));
    }
    // Réponses compressées (gzip) pour les téléphones en 3G : menu, suivi, courses (fiche 0048).
    app.layer(tower_http::compression::CompressionLayer::new()).with_state(etat)
}

pub async fn servir(etat: Etat, ui: Option<PathBuf>, ecoute: tokio::net::TcpListener) -> std::io::Result<()> {
    // L'aiguillage `/r/<slug>` passe avant le routage (fiche 0049).
    let app = tower::Layer::layer(&tower::util::MapRequestLayer::new(aiguiller), routeur(etat, ui));
    axum::serve(ecoute, axum::ServiceExt::<axum::extract::Request>::into_make_service_with_connect_info::<SocketAddr>(app)).await
}

/// Inscription d'un restaurant au cloud : renvoie la clé à saisir sur son poste central.
pub fn inscrire_restaurant(dossier: &std::path::Path, nom: &str) -> rusqlite::Result<String> {
    std::fs::create_dir_all(dossier).ok();
    let conn = Connection::open(dossier.join("youma-relais.db"))?;
    conn.execute_batch(SCHEMA)?;
    conn.execute_batch(cloud::SCHEMA)?;
    cloud::ajouter_restaurant(&conn, nom)
}

/// Nom court (adresse `/r/<slug>`) du restaurant de cette clé, sur le relais partagé (fiche 0049).
pub fn adresse_restaurant(dossier: &std::path::Path, cle: &str) -> rusqlite::Result<Option<String>> {
    let conn = Connection::open(dossier.join("youma-relais.db"))?;
    conn.query_row("SELECT slug FROM cloud_restaurants WHERE empreinte_cle = ?1", [hex(&empreinte(cle))], |r| r.get(0)).optional()
}

pub async fn demarrer(config: Config) -> std::io::Result<()> {
    let etat = Etat::ouvrir(&config).map_err(std::io::Error::other)?;
    let ecoute = tokio::net::TcpListener::bind(SocketAddr::from(([0, 0, 0, 0], config.port))).await?;
    tracing::info!("Relais Youma à l'écoute sur {}", ecoute.local_addr()?);
    servir(etat, config.dossier_ui.clone(), ecoute).await
}

// ───────────── Poste central ─────────────

#[derive(Deserialize)]
struct Synchronisation {
    /// Absent quand le relais a déjà ce menu (`menu_empreinte` renvoyée) : les photos ne repartent pas toutes les 10 s.
    #[serde(default)]
    menu: Option<Value>,
    #[serde(default)]
    menu_empreinte: Option<String>,
    #[serde(default)]
    config: Value,
    #[serde(default)]
    suivis: Vec<SuiviPublie>,
    #[serde(default)]
    resultats: Vec<Resultat>,
    /// RG-LIV-05 : livreurs ayant l'accès à l'application et leurs courses. Absent (poste ancien) : rien ne change.
    #[serde(default)]
    livreurs: Option<Vec<LivreurPublie>>,
}

#[derive(Deserialize)]
struct LivreurPublie {
    telephone: String,
    nom: String,
    pin_hash: String,
    #[serde(default)]
    courses: Vec<Value>,
}

#[derive(Deserialize)]
struct SuiviPublie {
    code_suivi: String,
    code_livreur: Option<String>,
    suivi: Value,
}

#[derive(Deserialize)]
struct Resultat {
    origine_id: String,
    reponse: Value,
}

async fn synchroniser(Resto(e): Resto, entetes: HeaderMap, Json(s): Json<Synchronisation>) -> Rep {
    let cle = entetes.get("authorization").and_then(|v| v.to_str().ok()).and_then(|v| v.strip_prefix("Bearer ")).unwrap_or("");
    if empreinte(cle) != e.empreinte_cle {
        return Err(erreur(StatusCode::UNAUTHORIZED, "NON_AUTHENTIFIE", "Clé du relais incorrecte"));
    }
    let t = maintenant();
    let (commandes, positions, modifications, avis, menu_empreinte) = e.avec(|c| {
        let tx = c.unchecked_transaction()?;
        let mut valeurs = vec![("config", s.config.clone()), ("dernier_contact", json!(t))];
        if let Some(m) = &s.menu {
            let mut m = m.clone();
            ranger_photos(&tx, &mut m)?;
            valeurs.push(("menu", m));
            valeurs.push(("menu_empreinte", json!(s.menu_empreinte)));
        }
        for (cle, v) in valeurs {
            tx.execute("INSERT OR REPLACE INTO etat(cle, valeur) VALUES (?1, ?2)", params![cle, v.to_string()])?;
        }
        let menu_empreinte: Option<String> = tx
            .query_row("SELECT valeur FROM etat WHERE cle = 'menu_empreinte'", [], |r| r.get::<_, String>(0))
            .optional()?
            .and_then(|v| serde_json::from_str::<Option<String>>(&v).ok().flatten());
        // Les suivis sont un cache : la dernière publication du poste fait foi.
        tx.execute("DELETE FROM suivis", [])?;
        for p in &s.suivis {
            tx.execute(
                "INSERT OR REPLACE INTO suivis(code_suivi, code_livreur, corps, maj) VALUES (?1, ?2, ?3, ?4)",
                params![p.code_suivi, p.code_livreur, p.suivi.to_string(), t],
            )?;
        }
        if let Some(livreurs) = &s.livreurs {
            tx.execute("DELETE FROM livreurs", [])?;
            for l in livreurs {
                tx.execute(
                    "INSERT OR REPLACE INTO livreurs(telephone, nom, pin_hash, courses, maj) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![normaliser_telephone(&l.telephone), l.nom, l.pin_hash, json!(l.courses).to_string(), t],
                )?;
            }
        }
        for r in &s.resultats {
            tx.execute("UPDATE commandes SET resultat = ?1 WHERE origine_id = ?2", params![r.reponse.to_string(), r.origine_id])?;
        }
        let a_transmettre: Vec<(String, String)> = tx
            .prepare("SELECT origine_id, corps FROM commandes WHERE resultat IS NULL AND (transmise_le IS NULL OR transmise_le < ?1) ORDER BY cree_le")?
            .query_map([t - RENVOI_MS], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<Result<_, _>>()?;
        for (id, _) in &a_transmettre {
            tx.execute("UPDATE commandes SET transmise_le = ?1 WHERE origine_id = ?2", params![t, id])?;
        }
        let positions: Vec<Value> = tx
            .prepare("SELECT code_livreur, lat, lon FROM positions WHERE transmise = 0")?
            .query_map([], |r| Ok(json!({ "code_livreur": r.get::<_, String>(0)?, "lat": r.get::<_, i64>(1)?, "lon": r.get::<_, i64>(2)? })))?
            .collect::<Result<_, _>>()?;
        tx.execute("UPDATE positions SET transmise = 1 WHERE transmise = 0", [])?;
        let modifications: Vec<Value> = tx
            .prepare("SELECT code_suivi, corps FROM modifications WHERE transmise = 0")?
            .query_map([], |r| {
                let corps: String = r.get(1)?;
                Ok(json!({ "code_suivi": r.get::<_, String>(0)?, "modification": serde_json::from_str::<Value>(&corps).unwrap_or_default() }))
            })?
            .collect::<Result<_, _>>()?;
        tx.execute("UPDATE modifications SET transmise = 1 WHERE transmise = 0", [])?;
        tx.execute("DELETE FROM modifications WHERE transmise = 1 AND ms < ?1", [t - 86_400_000])?;
        // RG-AVI-01 : avis des clients, transmis au poste qui les enregistre (fiche 0043).
        let avis: Vec<Value> = tx
            .prepare("SELECT code_suivi, note, commentaire FROM avis WHERE transmis = 0")?
            .query_map([], |r| Ok(json!({ "code_suivi": r.get::<_, String>(0)?, "note": r.get::<_, i64>(1)?, "commentaire": r.get::<_, String>(2)? })))?
            .collect::<Result<_, _>>()?;
        tx.execute("UPDATE avis SET transmis = 1 WHERE transmis = 0", [])?;
        tx.execute("DELETE FROM avis WHERE transmis = 1 AND ms < ?1", [t - 8 * 86_400_000])?;
        // Ménage : commandes traitées de plus de 3 jours, codes SMS expirés.
        tx.execute("DELETE FROM commandes WHERE cree_le < ?1 AND resultat IS NOT NULL", [t - 3 * 86_400_000])?;
        tx.execute("DELETE FROM verifications WHERE expire < ?1", [t])?;
        tx.execute("DELETE FROM clients_verifies WHERE utilise_le < ?1", [t - CLIENT_VERIFIE_MS])?;
        tx.execute("DELETE FROM sessions_livreurs WHERE expire < ?1", [t])?;
        tx.commit()?;
        let commandes: Vec<Value> = a_transmettre.into_iter().filter_map(|(_, c)| serde_json::from_str(&c).ok()).collect();
        Ok((commandes, positions, modifications, avis, menu_empreinte))
    })?;
    Ok(Json(json!({
        "commandes": commandes,
        "positions": positions,
        "modifications": modifications,
        "avis": avis,
        "sms": e.sms.nom(),
        "menu_empreinte": menu_empreinte,
    })))
}

// ───────────── Photos du menu (fiche 0048) ─────────────

/// Photo servie un an sans être redemandée : son adresse change avec son contenu.
const CACHE_PHOTO: &str = "public, max-age=31536000, immutable";

/// Sort les photos (`data:image/…;base64,…`) des plats du menu : chacune est gardée une fois, à part, et le menu
/// ne porte plus que son adresse `/api/public/photos/<empreinte>`. 50 000 ouvertures du menu ne retéléchargent
/// plus les photos : le téléphone les garde en cache. Les photos qui ne servent plus sont effacées.
fn ranger_photos(tx: &Connection, menu: &mut Value) -> rusqlite::Result<()> {
    let mut gardees = Vec::new();
    for p in menu["produits"].as_array_mut().into_iter().flatten() {
        let Some(photo) = p["photo"].as_str().map(str::to_owned) else { continue };
        if let Some(h) = photo.strip_prefix("/api/public/photos/") {
            gardees.push(h.to_string());
            continue;
        }
        let Some((type_, donnees)) = photo.strip_prefix("data:").and_then(|r| r.split_once(";base64,")) else { continue };
        let Ok(octets) = base64::engine::general_purpose::STANDARD.decode(donnees.trim()) else {
            p["photo"] = json!("");
            continue;
        };
        let h = hex(&empreinte(&photo))[..32].to_string();
        tx.execute("INSERT OR IGNORE INTO photos(empreinte, type, octets) VALUES (?1, ?2, ?3)", params![h, type_, octets])?;
        p["photo"] = json!(format!("/api/public/photos/{h}"));
        gardees.push(h);
    }
    let liste = json!(gardees).to_string();
    tx.execute("DELETE FROM photos WHERE empreinte NOT IN (SELECT value FROM json_each(?1))", [liste])?;
    Ok(())
}

async fn photo(Resto(e): Resto, Path(h): Path<String>) -> Response {
    let trouvee: Result<Option<(String, Vec<u8>)>, Echec> =
        e.avec(|c| c.query_row("SELECT type, octets FROM photos WHERE empreinte = ?1", [&h], |r| Ok((r.get(0)?, r.get(1)?))).optional());
    match trouvee {
        Ok(Some((type_, octets))) if type_.starts_with("image/") => {
            ([(axum::http::header::CONTENT_TYPE, type_), (axum::http::header::CACHE_CONTROL, CACHE_PHOTO.to_string())], octets).into_response()
        }
        Ok(_) => erreur(StatusCode::NOT_FOUND, "NON_TROUVE", "Photo introuvable").into_response(),
        Err(e) => e.into_response(),
    }
}

// ───────────── Client ─────────────

async fn menu(Resto(e): Resto, Query(q): Query<HashMap<String, String>>) -> Rep {
    if q.contains_key("table") {
        return Err(erreur(StatusCode::FORBIDDEN, "INTERDIT", "Le QR des tables fonctionne sur le Wi-Fi du restaurant"));
    }
    let Some(mut m) = e.lire("menu")? else {
        return Err(erreur(StatusCode::SERVICE_UNAVAILABLE, "INDISPONIBLE", "Le restaurant n'est pas encore connecté"));
    };
    if m["en_ligne"] != json!(true) {
        return Err(erreur(StatusCode::FORBIDDEN, "INTERDIT", "Commande en ligne non activée dans ce restaurant"));
    }
    let ouvert = m["ouvert"] == json!(true) && e.poste_joignable()?;
    m["ouvert"] = json!(ouvert);
    m["canaux_verification"] = json!(e.sms.canaux());
    Ok(Json(m))
}

#[derive(Deserialize)]
struct DemandeCode {
    telephone: String,
    /// `sms` (par défaut) ou `whatsapp`, au choix du client (fiche 0046).
    canal: Option<String>,
}

/// Numéro vérifié une fois : reconnu pendant 180 jours après sa dernière commande (fiche 0046).
pub const CLIENT_VERIFIE_MS: i64 = 180 * 86_400_000;

/// RG-CAN-04 : code à 4 chiffres envoyé par SMS ou WhatsApp, valable 10 minutes, 5 essais.
async fn verification(Resto(e): Resto, ConnectInfo(ip): ConnectInfo<SocketAddr>, entetes: HeaderMap, Json(d): Json<DemandeCode>) -> Rep {
    let config = e.lire("config")?.unwrap_or_default();
    if config["verification_numero"] != "sms" {
        return Err(erreur(StatusCode::FORBIDDEN, "INTERDIT", "Vérification du numéro non activée"));
    }
    let canal = sms::Canal::depuis(d.canal.as_deref())
        .filter(|c| e.sms.canaux().contains(&c.nom()))
        .ok_or_else(|| erreur(StatusCode::UNPROCESSABLE_ENTITY, "VALIDATION", "Moyen d'envoi du code non proposé"))?;
    let tel = normaliser_telephone(&d.telephone);
    if tel.len() != 8 {
        return Err(erreur(StatusCode::UNPROCESSABLE_ENTITY, "VALIDATION", "Numéro malien à 8 chiffres"));
    }
    let adresse = e.adresse(&entetes, ip);
    if !e.limiter(&format!("sms-ip:{adresse}"), 10, 3_600_000) || !e.limiter(&format!("sms-tel:{tel}"), 3, 3_600_000) {
        return Err(erreur(StatusCode::TOO_MANY_REQUESTS, "TROP_DE_DEMANDES", "Trop de codes demandés : réessayez plus tard"));
    }
    let code = format!("{:04}", rand::thread_rng().gen_range(0..10_000));
    let restaurant = e.lire("menu")?.and_then(|m| m["restaurant"].as_str().map(str::to_owned)).unwrap_or_default();
    if let Err(err) = e.sms.envoyer_code(canal, &format!("+223{tel}"), &restaurant, &code).await {
        tracing::warn!("code non envoyé : {err}");
        let message = if canal == sms::Canal::WhatsApp {
            "Message WhatsApp non envoyé : essayez par SMS ou appelez le restaurant"
        } else {
            "SMS non envoyé : réessayez ou appelez le restaurant"
        };
        return Err(erreur(StatusCode::BAD_GATEWAY, "SMS", message));
    }
    let h = hex(&empreinte(&format!("{tel}:{code}")));
    e.avec(|c| {
        c.execute(
            "INSERT OR REPLACE INTO verifications(telephone, empreinte, expire, tentatives) VALUES (?1, ?2, ?3, 0)",
            params![tel, h, maintenant() + 600_000],
        )
    })?;
    // Simulation (pas encore de contrat Orange, WhatsApp non configuré) : le code s'affiche sur la page du client.
    if e.sms.simule(canal) {
        return Ok(Json(json!({ "envoye": true, "simulation": true, "code": code })));
    }
    Ok(Json(json!({ "envoye": true })))
}

fn hex(o: &[u8]) -> String {
    o.iter().map(|b| format!("{b:02x}")).collect()
}

fn verifier_code(e: &Etat, tel: &str, code: &str) -> Result<bool, Echec> {
    e.avec(|c| {
        let ligne: Option<(String, i64, i64)> = c
            .query_row("SELECT empreinte, expire, tentatives FROM verifications WHERE telephone = ?1", [tel], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .optional()?;
        let Some((h, expire, tentatives)) = ligne else { return Ok(false) };
        if expire < maintenant() || tentatives >= 5 {
            return Ok(false);
        }
        if h == hex(&empreinte(&format!("{tel}:{}", code.trim()))) {
            c.execute("DELETE FROM verifications WHERE telephone = ?1", [tel])?;
            Ok(true)
        } else {
            c.execute("UPDATE verifications SET tentatives = tentatives + 1 WHERE telephone = ?1", [tel])?;
            Ok(false)
        }
    })
}

#[derive(Deserialize)]
struct Confirmation {
    telephone: String,
    code: String,
}

/// Fiche 0046 : le bon code donne un jeton au téléphone du client ; ses commandes suivantes n'ont plus besoin de
/// code. Seule l'empreinte du jeton est gardée.
async fn confirmer(Resto(e): Resto, Json(d): Json<Confirmation>) -> Rep {
    let tel = normaliser_telephone(&d.telephone);
    if !verifier_code(&e, &tel, &d.code)? {
        return Err(erreur(StatusCode::UNPROCESSABLE_ENTITY, "VALIDATION", "Code incorrect ou expiré"));
    }
    let jeton = code_aleatoire(32);
    let t = maintenant();
    e.avec(|c| {
        c.execute(
            "INSERT INTO clients_verifies(empreinte, telephone, cree_le, utilise_le) VALUES (?1, ?2, ?3, ?3)",
            params![hex(&empreinte(&jeton)), tel, t],
        )
    })?;
    Ok(Json(json!({ "telephone": tel, "jeton_client": jeton })))
}

/// Jeton valable pour ce numéro : sa date d'usage est prolongée.
fn client_verifie(e: &Etat, tel: &str, jeton: &str) -> Result<bool, Echec> {
    if jeton.is_empty() || tel.is_empty() {
        return Ok(false);
    }
    let t = maintenant();
    e.avec(|c| {
        let n = c.execute(
            "UPDATE clients_verifies SET utilise_le = ?3 WHERE empreinte = ?1 AND telephone = ?2 AND utilise_le >= ?4",
            params![hex(&empreinte(jeton)), tel, t, t - CLIENT_VERIFIE_MS],
        )?;
        Ok(n == 1)
    })
}

/// Commande en ligne : gardée jusqu'à ce que le poste central la reprenne. Le client reçoit tout de suite
/// son code de suivi ; le poste décide (prix, zones à risque, liste noire, file de validation).
async fn commande(Resto(e): Resto, ConnectInfo(ip): ConnectInfo<SocketAddr>, entetes: HeaderMap, Json(mut c): Json<Value>) -> Rep {
    let adresse = e.adresse(&entetes, ip);
    if !e.limiter(&format!("cmd:{adresse}"), 10, 600_000) {
        return Err(erreur(StatusCode::TOO_MANY_REQUESTS, "TROP_DE_DEMANDES", "Trop de commandes : réessayez dans quelques minutes"));
    }
    if c["canal"] != "en_ligne" {
        return Ok(refus("Le QR des tables fonctionne sur le Wi-Fi du restaurant"));
    }
    let lignes = c["lignes"].as_array().map(Vec::len).unwrap_or(0);
    if lignes == 0 || lignes > 50 || c.to_string().len() > 20_000 {
        return Ok(refus("Commande invalide"));
    }
    if !e.poste_joignable()? {
        return Ok(refus("Le restaurant est injoignable pour le moment : appelez-le directement"));
    }
    let config = e.lire("config")?.unwrap_or_default();
    let tel = normaliser_telephone(c["telephone"].as_str().unwrap_or(""));
    let verifie = if config["verification_numero"] == "sms" {
        let jeton = c["jeton_client"].as_str().unwrap_or("").to_string();
        let code = c["code_verification"].as_str().unwrap_or("").to_string();
        if !client_verifie(&e, &tel, &jeton)? && !verifier_code(&e, &tel, &code)? {
            return Ok(refus(if jeton.is_empty() { "Code incorrect ou expiré" } else { "Numéro à vérifier de nouveau" }));
        }
        true
    } else {
        false
    };
    // RG-CAN-07 : même numéro et même panier depuis moins de 5 minutes (page rechargée, double appui) : on renvoie
    // la commande déjà transmise au lieu d'en créer une seconde.
    let empreinte = |corps: &Value| {
        empreinte_panier(corps["lignes"].as_array().into_iter().flatten().map(|l| (l["produit_id"].as_str().unwrap_or(""), l["quantite"].as_i64().unwrap_or(1))))
    };
    let voulu = empreinte(&c);
    let recentes: Vec<(String, String)> = e.avec(|db| {
        db.prepare("SELECT code_suivi, corps FROM commandes WHERE cree_le >= ?1 ORDER BY cree_le DESC")?
            .query_map([maintenant() - FENETRE_DOUBLON_MS], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect()
    })?;
    for (code, corps) in recentes {
        let corps: Value = serde_json::from_str(&corps).unwrap_or_default();
        if !tel.is_empty() && normaliser_telephone(corps["telephone"].as_str().unwrap_or("")) == tel && empreinte(&corps) == voulu {
            return Ok(Json(json!({
                "statut": "en_attente",
                "message": "Votre commande a déjà été reçue : inutile de la renvoyer.",
                "numero": null,
                "code_suivi": code,
                "total": 0,
            })));
        }
    }
    let origine = uuid::Uuid::now_v7().to_string();
    let code_suivi = code_aleatoire(8);
    let o = c.as_object_mut().ok_or_else(|| erreur(StatusCode::UNPROCESSABLE_ENTITY, "VALIDATION", "Commande invalide"))?;
    o.remove("code_verification");
    o.remove("jeton_client");
    o.insert("origine_id".into(), json!(origine));
    o.insert("code_suivi".into(), json!(code_suivi));
    o.insert("telephone_verifie".into(), json!(verifie));
    e.avec(|db| {
        db.execute(
            "INSERT INTO commandes(origine_id, code_suivi, corps, cree_le) VALUES (?1, ?2, ?3, ?4)",
            params![origine, code_suivi, c.to_string(), maintenant()],
        )
    })?;
    Ok(Json(json!({
        "statut": "en_attente",
        "message": "Commande transmise : le restaurant va la confirmer",
        "numero": null,
        "code_suivi": code_suivi,
        "total": 0,
    })))
}

// ───────────── Livreur (RG-LIV-05, fiche 0047) ─────────────

/// Session de l'application Youma Livreur : 30 jours.
pub const SESSION_LIVREUR_MS: i64 = 30 * 86_400_000;

#[derive(Deserialize)]
struct ConnexionLivreur {
    telephone: String,
    pin: String,
}

/// RG-LIV-05 : téléphone + PIN donné par le restaurant. 5 essais en 15 minutes par numéro, 20 par adresse.
async fn connexion_livreur(Resto(e): Resto, ConnectInfo(ip): ConnectInfo<SocketAddr>, entetes: HeaderMap, Json(c): Json<ConnexionLivreur>) -> Rep {
    let tel = normaliser_telephone(&c.telephone);
    let adresse = e.adresse(&entetes, ip);
    if !e.limiter(&format!("livreur-ip:{adresse}"), 20, 15 * 60_000) || !e.limiter(&format!("livreur-tel:{tel}"), 5, 15 * 60_000) {
        return Err(erreur(StatusCode::TOO_MANY_REQUESTS, "TROP_DE_DEMANDES", "Trop d'essais : réessayez dans 15 minutes"));
    }
    let livreur: Option<(String, String)> =
        e.avec(|x| x.query_row("SELECT nom, pin_hash FROM livreurs WHERE telephone = ?1", [&tel], |r| Ok((r.get(0)?, r.get(1)?))).optional())?;
    let pin = c.pin.trim().to_string();
    let ok = match &livreur {
        Some((_, h)) => {
            let h = h.clone();
            tokio::task::spawn_blocking(move || youma_core::auth::verifier(&pin, &h)).await.map_err(interne)?
        }
        None => false,
    };
    let Some((nom, pin_hash)) = livreur.filter(|_| ok) else {
        return Err(erreur(StatusCode::UNAUTHORIZED, "NON_AUTHENTIFIE", "Téléphone ou PIN incorrect"));
    };
    let jeton = youma_core::auth::nouveau_jeton();
    e.avec(|x| {
        x.execute(
            "INSERT INTO sessions_livreurs(empreinte, telephone, pin_hash, expire) VALUES (?1, ?2, ?3, ?4)",
            params![hex(&empreinte(&jeton)), tel, pin_hash, maintenant() + SESSION_LIVREUR_MS],
        )
    })?;
    let restaurant = e.lire("menu")?.and_then(|m| m["restaurant"].as_str().map(str::to_owned)).unwrap_or_default();
    Ok(Json(json!({ "jeton": jeton, "nom": nom, "restaurant": restaurant })))
}

/// Courses en cours du livreur connecté. Un PIN changé ou un accès retiré sur le poste ferme ses sessions.
async fn courses_livreur(Resto(e): Resto, entetes: HeaderMap) -> Rep {
    let jeton = entetes.get("authorization").and_then(|v| v.to_str().ok()).and_then(|v| v.strip_prefix("Bearer ")).unwrap_or("");
    let h = hex(&empreinte(jeton));
    let trouve: Option<(String, String)> = e.avec(|x| {
        x.query_row(
            "SELECT l.nom, l.courses FROM sessions_livreurs s JOIN livreurs l ON l.telephone = s.telephone AND l.pin_hash = s.pin_hash
             WHERE s.empreinte = ?1 AND s.expire > ?2",
            params![h, maintenant()],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()
    })?;
    let Some((nom, courses)) = trouve.filter(|_| !jeton.is_empty()) else {
        e.avec(|x| x.execute("DELETE FROM sessions_livreurs WHERE empreinte = ?1", [&h]))?;
        return Err(erreur(StatusCode::UNAUTHORIZED, "NON_AUTHENTIFIE", "Session expirée : reconnectez-vous"));
    };
    let courses: Value = serde_json::from_str(&courses).unwrap_or_else(|_| json!([]));
    Ok(Json(json!({ "nom": nom, "courses": courses })))
}

const ESSAIS_MAX: usize = 15;
const ESSAIS_FENETRE_MS: i64 = 15 * 60_000;

/// RG-CAN-08 : refus après 15 codes inconnus en 15 minutes depuis la même adresse (énumération des codes).
fn essais_publics(e: &Etat, adresse: &str) -> Result<(), Echec> {
    if e.essais_epuises(adresse) {
        return Err(erreur(StatusCode::TOO_MANY_REQUESTS, "TROP_D_ESSAIS", "Trop de codes inconnus essayés : réessayez dans 15 minutes"));
    }
    Ok(())
}

fn compter_si_inconnu(e: &Etat, adresse: &str, r: &Rep) {
    if matches!(r, Err(echec) if echec.statut == StatusCode::NOT_FOUND) {
        e.compter_essai(adresse);
    }
}

async fn suivi(Resto(e): Resto, ConnectInfo(ip): ConnectInfo<SocketAddr>, entetes: HeaderMap, Path(code): Path<String>) -> Rep {
    let adresse = e.adresse(&entetes, ip);
    essais_publics(&e, &adresse)?;
    let r = suivi_publie(&e, code);
    compter_si_inconnu(&e, &adresse, &r);
    r
}

fn suivi_publie(e: &Etat, code: String) -> Rep {
    let code = code.trim().to_uppercase();
    let publie: Option<(String, Option<String>)> =
        e.avec(|c| c.query_row("SELECT corps, code_livreur FROM suivis WHERE code_suivi = ?1", [&code], |r| Ok((r.get(0)?, r.get(1)?))).optional())?;
    if let Some((corps, livreur)) = publie {
        let mut s: Value = serde_json::from_str(&corps).map_err(interne)?;
        // Avis reçu ici mais pas encore transmis au poste : la page de suivi le montre déjà.
        let note: Option<i64> = e.avec(|c| c.query_row("SELECT note FROM avis WHERE code_suivi = ?1", [&code], |r| r.get(0)).optional())?;
        if let Some(n) = note {
            s["avis"] = json!(n);
            s["avis_possible"] = json!(false);
        }
        // Position la plus fraîche : celle reçue ici, sans attendre le passage par le poste.
        if s["etape"] == "en_route" {
            if let Some(l) = livreur {
                let p: Option<(i64, i64, i64)> =
                    e.avec(|c| c.query_row("SELECT lat, lon, ms FROM positions WHERE code_livreur = ?1", [&l], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).optional())?;
                if let Some((lat, lon, ms)) = p {
                    if ms > s["livreur"][2].as_i64().unwrap_or(0) {
                        s["livreur"] = json!([lat, lon, ms]);
                    }
                }
            }
        }
        return Ok(Json(s));
    }
    let attente: Option<(Option<String>, i64)> =
        e.avec(|c| c.query_row("SELECT resultat, cree_le FROM commandes WHERE code_suivi = ?1", [&code], |r| Ok((r.get(0)?, r.get(1)?))).optional())?;
    let Some((resultat, cree_le)) = attente else {
        return Err(erreur(StatusCode::NOT_FOUND, "NON_TROUVE", "Commande introuvable"));
    };
    let resultat: Value = resultat.and_then(|r| serde_json::from_str(&r).ok()).unwrap_or_default();
    let refusee = resultat["statut"] == "refusee";
    let restaurant = e.lire("menu")?.and_then(|m| m["restaurant"].as_str().map(str::to_owned)).unwrap_or_default();
    Ok(Json(json!({
        "numero": resultat["numero"].as_i64().unwrap_or(0),
        "restaurant": restaurant,
        "etape": if refusee { "refusee" } else { "recue" },
        "motif": if refusee { resultat["message"].clone() } else { Value::Null },
        "type": "livraison",
        "total": resultat["total"].as_i64().unwrap_or(0),
        "reste": resultat["total"].as_i64().unwrap_or(0),
        "paiement_mode": null,
        "lignes": [],
        "livreur": null,
        "destination": null,
        "mis_a_jour": cree_le,
    })))
}

/// RG-CAN-06 : modification par le client, transmise au poste qui la vérifie et l'applique (ou non) ; le client voit
/// le résultat dans son suivi. Le relais n'accepte que si le dernier suivi publié permet encore de modifier.
async fn modifier(Resto(e): Resto, Path(code): Path<String>, Json(m): Json<Value>) -> Rep {
    let code = code.trim().to_uppercase();
    let lignes = m["lignes"].as_array().map(|l| l.len()).unwrap_or(0);
    if lignes == 0 || lignes > 50 || m["note"].as_str().unwrap_or("").len() > 500 {
        return Ok(refus("Commande vide ou trop longue"));
    }
    let restantes: Option<i64> = e.avec(|c| {
        c.query_row("SELECT corps FROM suivis WHERE code_suivi = ?1", [&code], |r| r.get::<_, String>(0))
            .optional()
            .map(|o| o.and_then(|s| serde_json::from_str::<Value>(&s).ok()).map(|v| v["modifications_restantes"].as_i64().unwrap_or(0)))
    })?;
    match restantes {
        None => Ok(refus("Commande pas encore reçue par le restaurant : réessayez dans quelques secondes")),
        Some(0) => Ok(refus("Le restaurant a déjà pris votre commande, ou vous l'avez déjà modifiée deux fois : appelez le restaurant")),
        Some(_) => {
            e.avec(|c| {
                c.execute(
                    "INSERT OR REPLACE INTO modifications(code_suivi, corps, ms, transmise) VALUES (?1, ?2, ?3, 0)",
                    params![code, m.to_string(), maintenant()],
                )
            })?;
            Ok(Json(json!({ "statut": "en_attente", "message": "Modification envoyée au restaurant", "numero": null, "code_suivi": code, "total": 0 })))
        }
    }
}

#[derive(Deserialize)]
struct NouvelAvis {
    note: i64,
    #[serde(default)]
    commentaire: String,
}

/// RG-AVI-01 : avis du client sur une commande terminée (suivi publié par le poste), une seule fois ; le poste
/// revérifie à la réception. Codes inconnus comptés comme les autres essais (RG-CAN-08).
async fn avis(Resto(e): Resto, ConnectInfo(ip): ConnectInfo<SocketAddr>, entetes: HeaderMap, Path(code): Path<String>, Json(a): Json<NouvelAvis>) -> Rep {
    let adresse = e.adresse(&entetes, ip);
    essais_publics(&e, &adresse)?;
    let r = enregistrer_avis(&e, &code, a);
    compter_si_inconnu(&e, &adresse, &r);
    r
}

fn enregistrer_avis(e: &Etat, code: &str, a: NouvelAvis) -> Rep {
    let code = code.trim().to_uppercase();
    if !(1..=5).contains(&a.note) || a.commentaire.chars().count() > 500 {
        return Err(erreur(StatusCode::UNPROCESSABLE_ENTITY, "VALIDATION", "Note de 1 à 5, commentaire de 500 caractères au plus"));
    }
    let suivi: Option<Value> = e.avec(|c| {
        c.query_row("SELECT corps FROM suivis WHERE code_suivi = ?1", [&code], |r| r.get::<_, String>(0))
            .optional()
            .map(|o| o.and_then(|s| serde_json::from_str::<Value>(&s).ok()))
    })?;
    let Some(s) = suivi else { return Err(erreur(StatusCode::NOT_FOUND, "NON_TROUVE", "Commande introuvable")) };
    if !s["avis_possible"].as_bool().unwrap_or(false) {
        return Err(erreur(StatusCode::UNPROCESSABLE_ENTITY, "REGLE_METIER", "Vous pourrez donner votre avis une fois la commande servie ou livrée"));
    }
    let n = e.avec(|c| {
        c.execute(
            "INSERT OR IGNORE INTO avis(code_suivi, note, commentaire, ms) VALUES (?1, ?2, ?3, ?4)",
            params![code, a.note, a.commentaire.trim(), maintenant()],
        )
    })?;
    if n == 0 {
        return Err(erreur(StatusCode::UNPROCESSABLE_ENTITY, "REGLE_METIER", "Votre avis a déjà été reçu : merci"));
    }
    Ok(Json(Value::Null))
}

#[derive(Deserialize)]
struct Position {
    lat: i64,
    lon: i64,
}

/// RG-LIV-04 : position du livreur (lien secret), pendant la course seulement. Le poste revérifie.
async fn position(Resto(e): Resto, ConnectInfo(ip): ConnectInfo<SocketAddr>, entetes: HeaderMap, Path(code): Path<String>, Json(p): Json<Position>) -> Rep {
    let adresse = e.adresse(&entetes, ip);
    essais_publics(&e, &adresse)?;
    let r = enregistrer_position(&e, &code, p);
    compter_si_inconnu(&e, &adresse, &r);
    r
}

fn enregistrer_position(e: &Etat, code: &str, p: Position) -> Rep {
    if !(-90_000_000..=90_000_000).contains(&p.lat) || !(-180_000_000..=180_000_000).contains(&p.lon) {
        return Err(erreur(StatusCode::UNPROCESSABLE_ENTITY, "VALIDATION", "Position invalide"));
    }
    let etape: Option<String> = e.avec(|c| {
        c.query_row("SELECT corps FROM suivis WHERE code_livreur = ?1", [code.trim()], |r| r.get::<_, String>(0))
            .optional()
            .map(|o| o.and_then(|s| serde_json::from_str::<Value>(&s).ok()).and_then(|v| v["etape"].as_str().map(str::to_owned)))
    })?;
    match etape.as_deref() {
        None => Err(erreur(StatusCode::NOT_FOUND, "NON_TROUVE", "Course introuvable")),
        Some("livree" | "echec" | "annulee" | "refusee") => {
            Err(erreur(StatusCode::UNPROCESSABLE_ENTITY, "REGLE_METIER", "Le suivi n'est actif que pendant la course"))
        }
        Some(_) => {
            e.avec(|c| {
                c.execute(
                    "INSERT OR REPLACE INTO positions(code_livreur, lat, lon, ms, transmise) VALUES (?1, ?2, ?3, ?4, 0)",
                    params![code.trim(), p.lat, p.lon, maintenant()],
                )
            })?;
            Ok(Json(Value::Null))
        }
    }
}
