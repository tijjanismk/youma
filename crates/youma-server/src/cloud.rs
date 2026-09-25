//! Synchronisation avec le cloud facultatif (fiche 0018) : résumés de journée et sauvegardes chiffrées.
//! Le poste reste la source de vérité ; sans connexion, rien ne s'arrête et l'envoi reprend plus tard.

use std::time::Duration;

use serde::Serialize;
use serde_json::Value;
use youma_core::{cloud, parametres, sauvegarde};

use crate::Etat;

#[derive(Debug, Clone, Serialize)]
pub struct EtatCloud {
    #[serde(skip)]
    pub intervalle_ms: u64,
    pub actif: bool,
    pub dernier_succes: Option<i64>,
    pub derniere_erreur: Option<String>,
    pub derniere_sauvegarde: Option<String>,
    /// Fournisseur SMS du serveur Internet : orange_mali ou simulation.
    pub sms: Option<String>,
}

impl Default for EtatCloud {
    fn default() -> Self {
        EtatCloud { intervalle_ms: 5 * 60_000, actif: false, dernier_succes: None, derniere_erreur: None, derniere_sauvegarde: None, sms: None }
    }
}

pub fn lancer(etat: Etat) {
    tokio::spawn(async move {
        loop {
            let intervalle = etat.cloud.lock().map(|c| c.intervalle_ms).unwrap_or(300_000);
            tokio::time::sleep(Duration::from_millis(intervalle)).await;
            let _ = synchroniser(&etat).await;
        }
    });
}

fn client() -> reqwest::Client {
    reqwest::Client::builder().timeout(Duration::from_secs(600)).build().unwrap_or_default()
}

/// Adresse et clé du cloud, si configuré et permis par la licence (module « cloud »).
pub(crate) async fn configuration(etat: &Etat) -> Result<Option<(String, String)>, String> {
    let sans_licence = etat.config.demo || etat.config.reseau_sans_licence;
    etat.avec_db(move |db| {
        let c = parametres::lire(db.conn())?.cloud;
        if c.url.trim().is_empty() || c.cle.trim().is_empty() {
            return Ok(None);
        }
        let jour = youma_core::horloge::date_locale(db.maintenant(), 0);
        if !sans_licence && !youma_core::licence::module_actif(db.conn(), "cloud", &jour) {
            return Err(youma_core::Erreur::Interdit("Module cloud non activé par la licence".into()));
        }
        Ok(Some((format!("{}/api", c.url.trim().trim_end_matches('/')), c.cle)))
    })
    .await
    .map_err(|e| e.0.to_string())
}

fn noter(etat: &Etat, f: impl FnOnce(&mut EtatCloud)) {
    if let Ok(mut c) = etat.cloud.lock() {
        f(&mut c);
    }
}

/// RG-CLO-01/02 : envoie les résumés, puis la dernière sauvegarde (chiffrée) si elle n'a pas encore été envoyée.
pub async fn synchroniser(etat: &Etat) -> Result<(), String> {
    let r = synchroniser_interne(etat).await;
    match &r {
        Ok(true) => noter(etat, |c| {
            c.actif = true;
            c.dernier_succes = Some(chrono::Utc::now().timestamp_millis());
            c.derniere_erreur = None;
        }),
        Ok(false) => noter(etat, |c| c.actif = false),
        Err(e) => {
            tracing::warn!("Cloud : {e}");
            noter(etat, |c| {
                c.actif = true;
                c.derniere_erreur = Some(e.clone());
            })
        }
    }
    r.map(|_| ())
}

async fn synchroniser_interne(etat: &Etat) -> Result<bool, String> {
    let Some((url, cle)) = configuration(etat).await? else { return Ok(false) };
    let charge = etat.avec_db(|db| cloud::charge_synchronisation(db.conn(), db.maintenant())).await.map_err(|e| e.0.to_string())?;
    let http = client();
    let reponse: Value = http
        .post(format!("{url}/cloud/synchroniser"))
        .bearer_auth(&cle)
        .json(&charge)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    noter(etat, |c| c.sms = reponse["sms"].as_str().map(str::to_owned));

    // Dernière sauvegarde locale, chiffrée avec la phrase du restaurant (le cloud ne peut pas la lire).
    let dossier = etat.config.dossier_sauvegardes();
    let (phrase, deja) = etat
        .avec_db(|db| Ok((parametres::lire(db.conn())?.cloud.phrase_chiffrement, cloud::derniere_sauvegarde_envoyee(db.conn())?)))
        .await
        .map_err(|e| e.0.to_string())?;
    if phrase.is_empty() {
        return Ok(true);
    }
    let Some(derniere) = sauvegarde::lister(&dossier).map_err(|e| e.to_string())?.into_iter().find(|s| !s.motif.starts_with("cloud")) else {
        return Ok(true);
    };
    let nom = std::path::Path::new(&derniere.chemin).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    if deja.as_deref() == Some(nom.as_str()) {
        return Ok(true);
    }
    let chemin = derniere.chemin.clone();
    let chiffre = tokio::task::spawn_blocking(move || -> Result<Vec<u8>, String> {
        let octets = std::fs::read(&chemin).map_err(|e| e.to_string())?;
        cloud::chiffrer(&octets, &phrase).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())??;
    http.put(format!("{url}/cloud/sauvegardes/envoi/{nom}"))
        .bearer_auth(&cle)
        .body(chiffre)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| e.to_string())?;
    let n = nom.clone();
    etat.avec_db(move |db| cloud::marquer_sauvegarde_envoyee(db.conn(), &n)).await.map_err(|e| e.0.to_string())?;
    noter(etat, |c| c.derniere_sauvegarde = Some(nom));
    Ok(true)
}

/// Liste des sauvegardes gardées dans le cloud pour ce restaurant.
pub async fn sauvegardes_distantes(etat: &Etat) -> Result<Value, String> {
    let (url, cle) = configuration(etat).await?.ok_or("Cloud non configuré")?;
    client()
        .get(format!("{url}/cloud/sauvegardes"))
        .bearer_auth(cle)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
}

/// Récupère une sauvegarde distante, la déchiffre et la range avec les sauvegardes locales (à restaurer ensuite).
pub async fn recuperer(etat: &Etat, id: &str) -> Result<String, String> {
    let (url, cle) = configuration(etat).await?.ok_or("Cloud non configuré")?;
    let octets = client()
        .get(format!("{url}/cloud/sauvegardes/{id}"))
        .bearer_auth(cle)
        .send()
        .await
        .and_then(|r| r.error_for_status())
        .map_err(|e| e.to_string())?
        .bytes()
        .await
        .map_err(|e| e.to_string())?;
    let (phrase, ms) = etat
        .avec_db(|db| Ok((parametres::lire(db.conn())?.cloud.phrase_chiffrement, db.maintenant())))
        .await
        .map_err(|e| e.0.to_string())?;
    let dossier = etat.config.dossier_sauvegardes();
    tokio::task::spawn_blocking(move || -> Result<String, String> {
        let clair = cloud::dechiffrer(&octets, &phrase).map_err(|e| e.to_string())?;
        std::fs::create_dir_all(&dossier).map_err(|e| e.to_string())?;
        let d = chrono::DateTime::<chrono::Utc>::from_timestamp_millis(ms).unwrap_or_default();
        let chemin = dossier.join(format!("youma-{}-cloud.db", d.format("%Y%m%d-%H%M%S")));
        std::fs::write(&chemin, clair).map_err(|e| e.to_string())?;
        Ok(chemin.to_string_lossy().to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
