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
use rand::Rng;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tower_http::services::{ServeDir, ServeFile};
use youma_core::entrantes::code_aleatoire;
use youma_core::zones_risque::normaliser_telephone;

pub mod cloud;
pub mod sms;
pub use sms::{FournisseurSms, Orange};

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
CREATE TABLE IF NOT EXISTS verifications (telephone TEXT PRIMARY KEY, empreinte TEXT NOT NULL, expire INTEGER NOT NULL,
    tentatives INTEGER NOT NULL DEFAULT 0);
";

impl Etat {
    pub fn ouvrir(config: &Config) -> rusqlite::Result<Etat> {
        std::fs::create_dir_all(&config.dossier_donnees).ok();
        let conn = Connection::open(config.dossier_donnees.join("youma-relais.db"))?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.execute_batch(SCHEMA)?;
        conn.execute_batch(cloud::SCHEMA)?;
        Ok(Etat {
            db: Arc::new(Mutex::new(conn)),
            empreinte_cle: empreinte(&config.cle),
            sms: Arc::new(sms::Envoyeur::new(config.sms.clone())),
            limites: Arc::new(Mutex::new(HashMap::new())),
            derriere_proxy: config.derriere_proxy,
            dossier: config.dossier_donnees.clone(),
            sessions: Arc::new(Mutex::new(HashMap::new())),
        })
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
        .route("/api/public/commandes", post(commande))
        .route("/api/public/suivi/{code}", get(suivi))
        .route("/api/public/position/{code}", post(position))
        .merge(cloud::routes())
        // Le relais ne sert que les pages du client et du livreur, jamais l'application du personnel.
        .route("/", get(|| async { Redirect::temporary("/menu") }));
    if let Some(ui) = ui {
        let index = ui.join("index.html");
        app = app.fallback_service(ServeDir::new(ui).fallback(ServeFile::new(index)));
    }
    app.with_state(etat)
}

pub async fn servir(etat: Etat, ui: Option<PathBuf>, ecoute: tokio::net::TcpListener) -> std::io::Result<()> {
    axum::serve(ecoute, routeur(etat, ui).into_make_service_with_connect_info::<SocketAddr>()).await
}

/// Inscription d'un restaurant au cloud : renvoie la clé à saisir sur son poste central.
pub fn inscrire_restaurant(dossier: &std::path::Path, nom: &str) -> rusqlite::Result<String> {
    std::fs::create_dir_all(dossier).ok();
    let conn = Connection::open(dossier.join("youma-relais.db"))?;
    conn.execute_batch(SCHEMA)?;
    conn.execute_batch(cloud::SCHEMA)?;
    cloud::ajouter_restaurant(&conn, nom)
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
    menu: Value,
    #[serde(default)]
    config: Value,
    #[serde(default)]
    suivis: Vec<SuiviPublie>,
    #[serde(default)]
    resultats: Vec<Resultat>,
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

async fn synchroniser(State(e): State<Etat>, entetes: HeaderMap, Json(s): Json<Synchronisation>) -> Rep {
    let cle = entetes.get("authorization").and_then(|v| v.to_str().ok()).and_then(|v| v.strip_prefix("Bearer ")).unwrap_or("");
    if empreinte(cle) != e.empreinte_cle {
        return Err(erreur(StatusCode::UNAUTHORIZED, "NON_AUTHENTIFIE", "Clé du relais incorrecte"));
    }
    let t = maintenant();
    let (commandes, positions) = e.avec(|c| {
        let tx = c.unchecked_transaction()?;
        for (cle, v) in [("menu", &s.menu), ("config", &s.config), ("dernier_contact", &json!(t))] {
            tx.execute("INSERT OR REPLACE INTO etat(cle, valeur) VALUES (?1, ?2)", params![cle, v.to_string()])?;
        }
        // Les suivis sont un cache : la dernière publication du poste fait foi.
        tx.execute("DELETE FROM suivis", [])?;
        for p in &s.suivis {
            tx.execute(
                "INSERT OR REPLACE INTO suivis(code_suivi, code_livreur, corps, maj) VALUES (?1, ?2, ?3, ?4)",
                params![p.code_suivi, p.code_livreur, p.suivi.to_string(), t],
            )?;
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
        // Ménage : commandes traitées de plus de 3 jours, codes SMS expirés.
        tx.execute("DELETE FROM commandes WHERE cree_le < ?1 AND resultat IS NOT NULL", [t - 3 * 86_400_000])?;
        tx.execute("DELETE FROM verifications WHERE expire < ?1", [t])?;
        tx.commit()?;
        let commandes: Vec<Value> = a_transmettre.into_iter().filter_map(|(_, c)| serde_json::from_str(&c).ok()).collect();
        Ok((commandes, positions))
    })?;
    Ok(Json(json!({ "commandes": commandes, "positions": positions, "sms": e.sms.nom() })))
}

// ───────────── Client ─────────────

async fn menu(State(e): State<Etat>, Query(q): Query<HashMap<String, String>>) -> Rep {
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
    Ok(Json(m))
}

#[derive(Deserialize)]
struct DemandeCode {
    telephone: String,
}

/// RG-CAN-04 : code à 4 chiffres envoyé par SMS, valable 10 minutes, 5 essais.
async fn verification(State(e): State<Etat>, ConnectInfo(ip): ConnectInfo<SocketAddr>, entetes: HeaderMap, Json(d): Json<DemandeCode>) -> Rep {
    let config = e.lire("config")?.unwrap_or_default();
    if config["verification_numero"] != "sms" {
        return Err(erreur(StatusCode::FORBIDDEN, "INTERDIT", "Vérification par SMS non activée"));
    }
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
    let message = format!("{restaurant} : votre code de commande est {code}");
    if let Err(err) = e.sms.envoyer(&format!("+223{tel}"), &message).await {
        tracing::warn!("SMS non envoyé : {err}");
        return Err(erreur(StatusCode::BAD_GATEWAY, "SMS", "SMS non envoyé : réessayez ou appelez le restaurant"));
    }
    let h = hex(&empreinte(&format!("{tel}:{code}")));
    e.avec(|c| {
        c.execute(
            "INSERT OR REPLACE INTO verifications(telephone, empreinte, expire, tentatives) VALUES (?1, ?2, ?3, 0)",
            params![tel, h, maintenant() + 600_000],
        )
    })?;
    // Simulation (pas encore de contrat Orange) : le code s'affiche sur la page du client.
    if e.sms.simulation() {
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

/// Commande en ligne : gardée jusqu'à ce que le poste central la reprenne. Le client reçoit tout de suite
/// son code de suivi ; le poste décide (prix, zones à risque, liste noire, file de validation).
async fn commande(State(e): State<Etat>, ConnectInfo(ip): ConnectInfo<SocketAddr>, entetes: HeaderMap, Json(mut c): Json<Value>) -> Rep {
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
        let code = c["code_verification"].as_str().unwrap_or("").to_string();
        if !verifier_code(&e, &tel, &code)? {
            return Ok(refus("Code SMS incorrect ou expiré"));
        }
        true
    } else {
        false
    };
    let origine = uuid::Uuid::now_v7().to_string();
    let code_suivi = code_aleatoire(8);
    let o = c.as_object_mut().ok_or_else(|| erreur(StatusCode::UNPROCESSABLE_ENTITY, "VALIDATION", "Commande invalide"))?;
    o.remove("code_verification");
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

async fn suivi(State(e): State<Etat>, Path(code): Path<String>) -> Rep {
    let code = code.trim().to_uppercase();
    let publie: Option<(String, Option<String>)> =
        e.avec(|c| c.query_row("SELECT corps, code_livreur FROM suivis WHERE code_suivi = ?1", [&code], |r| Ok((r.get(0)?, r.get(1)?))).optional())?;
    if let Some((corps, livreur)) = publie {
        let mut s: Value = serde_json::from_str(&corps).map_err(interne)?;
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

#[derive(Deserialize)]
struct Position {
    lat: i64,
    lon: i64,
}

/// RG-LIV-04 : position du livreur (lien secret), pendant la course seulement. Le poste revérifie.
async fn position(State(e): State<Etat>, Path(code): Path<String>, Json(p): Json<Position>) -> Rep {
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
