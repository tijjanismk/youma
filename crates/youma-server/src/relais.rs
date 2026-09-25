//! Synchronisation avec le serveur relais Internet facultatif (fiche 0013).
//! Le poste central appelle le relais (il n'a pas d'adresse publique) : il publie son menu et ses suivis,
//! reprend les commandes en ligne et les positions des livreurs. Sans relais configuré, rien ne se passe.

use std::time::Duration;

use serde::Serialize;
use serde_json::{json, Value};
use youma_core::entrantes::{self, CommandeEntrante};
use youma_core::parametres;

use crate::Etat;

#[derive(Debug, Clone, Serialize)]
pub struct EtatRelais {
    /// Intervalle entre deux synchronisations (réduit dans les tests).
    #[serde(skip)]
    pub intervalle_ms: u64,
    pub actif: bool,
    pub dernier_succes: Option<i64>,
    pub derniere_erreur: Option<String>,
    pub commandes_recues: u64,
    /// Fournisseur SMS du relais : orange_mali ou simulation.
    pub sms: Option<String>,
}

impl Default for EtatRelais {
    fn default() -> Self {
        EtatRelais { intervalle_ms: 10_000, actif: false, dernier_succes: None, derniere_erreur: None, commandes_recues: 0, sms: None }
    }
}

pub fn lancer(etat: Etat) {
    tokio::spawn(async move {
        let client = reqwest::Client::builder().timeout(Duration::from_secs(20)).build().unwrap_or_default();
        let mut resultats: Vec<Value> = Vec::new();
        loop {
            let intervalle = etat.relais.lock().map(|r| r.intervalle_ms).unwrap_or(10_000);
            tokio::time::sleep(Duration::from_millis(intervalle)).await;
            // Des commandes viennent d'arriver : on renvoie aussitôt les résultats au relais.
            while let Ok(n) = synchroniser(&etat, &client, &mut resultats).await {
                if n == 0 {
                    break;
                }
            }
        }
    });
}

/// Un aller-retour avec le relais ; renvoie le nombre de commandes reçues.
async fn synchroniser(etat: &Etat, client: &reqwest::Client, resultats: &mut Vec<Value>) -> Result<usize, ()> {
    let envoyes = resultats.clone();
    let nb_envoyes = envoyes.len();
    let preparation = etat
        .avec_db(move |db| {
            let p = parametres::lire(db.conn())?;
            let c = p.canaux;
            if c.relais_url.trim().is_empty() || !c.en_ligne {
                return Ok(None);
            }
            let corps = json!({
                "menu": entrantes::menu_public(db.conn(), None, db.maintenant())?,
                "config": { "verification_numero": c.verification_numero },
                "suivis": entrantes::suivis_recents(db.conn(), db.maintenant() - 24 * 3_600_000)?,
                "resultats": envoyes,
            });
            Ok(Some((format!("{}/api/relais/synchroniser", c.relais_url.trim().trim_end_matches('/')), c.relais_cle, corps)))
        })
        .await;
    let Ok(Some((url, cle, corps))) = preparation else {
        if let Ok(mut r) = etat.relais.lock() {
            r.actif = false;
        }
        return Err(());
    };
    let reponse = match client.post(&url).bearer_auth(&cle).json(&corps).send().await.and_then(|r| r.error_for_status()) {
        Ok(r) => r.json::<Value>().await,
        Err(e) => Err(e),
    };
    let reponse = match reponse {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("Relais injoignable : {e}");
            if let Ok(mut r) = etat.relais.lock() {
                r.actif = true;
                r.derniere_erreur = Some(e.to_string());
            }
            return Err(());
        }
    };
    // Les résultats envoyés sont acquittés.
    resultats.drain(..nb_envoyes.min(resultats.len()));

    let commandes = reponse["commandes"].as_array().cloned().unwrap_or_default();
    let n = commandes.len();
    for v in commandes {
        let origine = v["origine_id"].as_str().unwrap_or("").to_string();
        let reponse = match serde_json::from_value::<CommandeEntrante>(v) {
            Ok(c) => match etat.avec_db(move |db| entrantes::recevoir(db, &c)).await {
                Ok(r) => json!(r),
                Err(e) => json!({ "statut": "refusee", "message": e.0.to_string(), "numero": null, "code_suivi": null, "total": 0 }),
            },
            Err(_) => json!({ "statut": "refusee", "message": "Commande illisible", "numero": null, "code_suivi": null, "total": 0 }),
        };
        if !origine.is_empty() {
            resultats.push(json!({ "origine_id": origine, "reponse": reponse }));
        }
    }
    for p in reponse["positions"].as_array().cloned().unwrap_or_default() {
        let (Some(code), Some(lat), Some(lon)) = (p["code_livreur"].as_str().map(str::to_owned), p["lat"].as_i64(), p["lon"].as_i64()) else {
            continue;
        };
        // Hors course, la position est simplement ignorée (RG-LIV-04).
        let _ = etat.avec_db(move |db| entrantes::ajouter_position(db, &code, lat, lon)).await;
    }
    if let Ok(mut r) = etat.relais.lock() {
        r.actif = true;
        r.dernier_succes = Some(chrono::Utc::now().timestamp_millis());
        r.derniere_erreur = None;
        r.commandes_recues += n as u64;
        r.sms = reponse["sms"].as_str().map(str::to_owned);
    }
    Ok(n)
}
