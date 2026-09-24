//! Poste central : API HTTP + WebSocket, service de l'interface, imprimantes, tâches de fond.
//! Même code en mono-poste (écoute locale) et en réseau (écoute sur le réseau local).

pub mod api;
pub mod erreurs;
pub mod imprimantes;
pub mod relais;
pub mod taches;
pub mod ws;

use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use axum::extract::{ConnectInfo, FromRequestParts};
use axum::http::request::Parts;
use tokio::sync::broadcast;
use youma_core::erreur::{Erreur, Resultat};
use youma_core::horloge::{Horloge, HorlogeSysteme};
use youma_core::{auth, Acteur, Db};

use crate::erreurs::ApiErreur;

#[derive(Clone, Debug)]
pub struct Config {
    pub dossier_donnees: PathBuf,
    pub port: u16,
    /// Mode B : écoute sur toutes les interfaces du réseau local.
    pub reseau: bool,
    /// Dossier de l'interface compilée (ui/dist).
    pub dossier_ui: Option<PathBuf>,
    /// Base de démonstration séparée (youma-demo.db), remplie si vide.
    pub demo: bool,
    /// Désactive le contrôle de licence du module réseau (tests, démonstration).
    pub reseau_sans_licence: bool,
}

impl Config {
    pub fn chemin_base(&self) -> PathBuf {
        self.dossier_donnees.join(if self.demo { "youma-demo.db" } else { "youma.db" })
    }
    pub fn dossier_sauvegardes(&self) -> PathBuf {
        self.dossier_donnees.join("sauvegardes")
    }
}

#[derive(Clone)]
pub struct Etat {
    pub db: Arc<Mutex<Db>>,
    pub evenements: broadcast::Sender<String>,
    pub config: Arc<Config>,
    /// État de la synchronisation avec le relais Internet facultatif.
    pub relais: Arc<Mutex<relais::EtatRelais>>,
}

impl Etat {
    pub fn ouvrir(config: Config) -> Resultat<Etat> {
        Self::ouvrir_avec_horloge(config, Arc::new(HorlogeSysteme))
    }

    pub fn ouvrir_avec_horloge(config: Config, horloge: Arc<dyn Horloge>) -> Resultat<Etat> {
        let mut db = Db::ouvrir(&config.chemin_base(), horloge)?;
        if config.demo && auth::nombre_utilisateurs(db.conn())? == 0 {
            youma_core::demo::remplir(&mut db)?;
        }
        let (tx, _) = broadcast::channel(256);
        let emetteur = tx.clone();
        db.definir_ecouteur(Arc::new(move |e| {
            let _ = emetteur.send(serde_json::to_string(&e).unwrap_or_default());
        }));
        // Contrôle rapide au démarrage (cahier §20).
        let r = youma_core::sauvegarde::verifier_integrite(db.conn(), false, db.maintenant())?;
        if !r.ok {
            tracing::error!("Contrôle d'intégrité en échec : {:?}", r.messages);
        }
        Ok(Etat { db: Arc::new(Mutex::new(db)), evenements: tx, config: Arc::new(config), relais: Arc::default() })
    }

    /// Exécute une fonction bloquante sur la base (un seul écrivain, cf. fiche 0003).
    pub async fn avec_db<T: Send + 'static>(&self, f: impl FnOnce(&mut Db) -> Resultat<T> + Send + 'static) -> Result<T, ApiErreur> {
        let db = self.db.clone();
        tokio::task::spawn_blocking(move || {
            let mut g = db.lock().unwrap_or_else(|e| e.into_inner());
            f(&mut g)
        })
        .await
        .map_err(|e| ApiErreur(Erreur::validation(format!("tâche interrompue : {e}"))))?
        .map_err(ApiErreur)
    }
}

/// Utilisateur authentifié (en-tête `Authorization: Bearer …`),
/// avec éventuelle autorisation ponctuelle (`X-Autorisation-Pin`, RG-AUT-03).
pub struct Auth {
    pub acteur: Acteur,
    pub jeton: String,
    pub utilisateur_id: String,
}

fn entete(parts: &Parts, nom: &str) -> Option<String> {
    parts.headers.get(nom).and_then(|v| v.to_str().ok()).map(str::to_owned)
}

/// Mode réseau : un poste distant doit être un appareil appairé, et le module réseau actif.
async fn controler_appareil(parts: &mut Parts, etat: &Etat) -> Result<Option<String>, ApiErreur> {
    let ip = ConnectInfo::<SocketAddr>::from_request_parts(parts, etat).await.ok().map(|c| c.0.ip());
    let distant = ip.is_some_and(|ip| !est_local(ip));
    if !distant {
        return Ok(None);
    }
    let jeton = entete(parts, "x-appareil").unwrap_or_default();
    let sans_licence = etat.config.reseau_sans_licence || etat.config.demo;
    etat.avec_db(move |db| {
        let jour = youma_core::horloge::date_locale(db.maintenant(), 0);
        if !sans_licence && !youma_core::licence::module_actif(db.conn(), "reseau", &jour) {
            return Err(Erreur::Interdit("Module réseau non activé par la licence".into()));
        }
        youma_core::appareils::verifier(db.conn(), &jeton, db.maintenant())?
            .map(Some)
            .ok_or_else(|| Erreur::Interdit("Appareil non autorisé : appairez-le depuis le poste central".into()))
    })
    .await
}

pub fn est_local(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v) => v.is_loopback(),
        IpAddr::V6(v) => v.is_loopback() || v.to_ipv4_mapped().is_some_and(|v| v.is_loopback()),
    }
}

impl FromRequestParts<Etat> for Auth {
    type Rejection = ApiErreur;

    async fn from_request_parts(parts: &mut Parts, etat: &Etat) -> Result<Self, Self::Rejection> {
        let appareil = controler_appareil(parts, etat).await?;
        let jeton = entete(parts, "authorization")
            .and_then(|v| v.strip_prefix("Bearer ").map(str::to_owned))
            .ok_or(ApiErreur(Erreur::NonAuthentifie))?;
        let pin = entete(parts, "x-autorisation-pin");
        let j = jeton.clone();
        let s = etat.avec_db(move |db| auth::verifier_session(db, &j)).await?;
        let acteur = Acteur::utilisateur(&s.utilisateur_id)
            .avec_appareil(appareil.or(s.appareil_id))
            .avec_autorisation(pin)
            .avec_eleve(s.eleve);
        Ok(Auth { acteur, jeton, utilisateur_id: s.utilisateur_id })
    }
}

/// Appareil appelant (routes publiques : connexion, état).
pub struct Poste(pub Option<String>);

impl FromRequestParts<Etat> for Poste {
    type Rejection = ApiErreur;

    async fn from_request_parts(parts: &mut Parts, etat: &Etat) -> Result<Self, Self::Rejection> {
        Ok(Poste(controler_appareil(parts, etat).await?))
    }
}

/// Démarre le serveur (bloquant jusqu'à l'arrêt).
pub async fn demarrer(config: Config) -> Resultat<()> {
    let etat = Etat::ouvrir(config.clone())?;
    let adresse: IpAddr = if config.reseau { [0, 0, 0, 0].into() } else { [127, 0, 0, 1].into() };
    let ecoute = tokio::net::TcpListener::bind(SocketAddr::new(adresse, config.port)).await?;
    tracing::info!("Youma à l'écoute sur http://{}", ecoute.local_addr()?);
    servir(etat, ecoute).await
}

pub async fn servir(etat: Etat, ecoute: tokio::net::TcpListener) -> Resultat<()> {
    taches::lancer(etat.clone());
    relais::lancer(etat.clone());
    let app = api::routeur(etat);
    axum::serve(ecoute, app.into_make_service_with_connect_info::<SocketAddr>()).await?;
    Ok(())
}
