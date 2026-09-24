//! Envoi des codes SMS (RG-CAN-04) : **Orange Mali** (API SMS d'Orange Developer), ou simulation tant que
//! le contrat Orange n'est pas signé. Les identifiants restent sur le relais, seul à envoyer des SMS.
//!
//! [HYPOTHÈSE] API « SMS Mali » d'Orange Developer, à confirmer à la signature du contrat :
//! jeton OAuth2 `POST /oauth/v3/token` (client_credentials, authentification Basic), puis
//! `POST /smsmessaging/v1/outbound/tel:+223XXXXXXXX/requests` avec `outboundSMSMessageRequest`.

use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::Engine;
use serde_json::{json, Value};

#[derive(Clone, Debug)]
pub struct Orange {
    /// `https://api.orange.com` (modifiable pour les tests).
    pub api: String,
    pub client_id: String,
    pub client_secret: String,
    /// Numéro expéditeur attribué par Orange, format +223XXXXXXXX.
    pub expediteur: String,
    /// Nom d'expéditeur validé par Orange (facultatif).
    pub nom_expediteur: Option<String>,
}

#[derive(Clone, Debug)]
pub enum FournisseurSms {
    /// Aucun SMS réel : le code est journalisé et renvoyé à la page (essais, démonstration).
    Simulation,
    Orange(Orange),
}

impl FournisseurSms {
    pub fn nom(&self) -> &'static str {
        match self {
            FournisseurSms::Simulation => "simulation",
            FournisseurSms::Orange(_) => "orange_mali",
        }
    }

    /// Orange si `YOUMA_ORANGE_CLIENT_ID`, `YOUMA_ORANGE_CLIENT_SECRET` et `YOUMA_ORANGE_EXPEDITEUR` sont fournis.
    pub fn depuis_environnement() -> FournisseurSms {
        let v = |n: &str| std::env::var(n).ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
        match (v("YOUMA_ORANGE_CLIENT_ID"), v("YOUMA_ORANGE_CLIENT_SECRET"), v("YOUMA_ORANGE_EXPEDITEUR")) {
            (Some(client_id), Some(client_secret), Some(expediteur)) => FournisseurSms::Orange(Orange {
                api: v("YOUMA_ORANGE_API").unwrap_or_else(|| "https://api.orange.com".into()),
                client_id,
                client_secret,
                expediteur,
                nom_expediteur: v("YOUMA_ORANGE_NOM_EXPEDITEUR"),
            }),
            _ => FournisseurSms::Simulation,
        }
    }
}

pub struct Envoyeur {
    fournisseur: FournisseurSms,
    http: reqwest::Client,
    /// Jeton Orange et son expiration (ms).
    jeton: Mutex<Option<(String, i64)>>,
}

fn maintenant() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

fn encoder(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

impl Envoyeur {
    pub fn new(fournisseur: FournisseurSms) -> Envoyeur {
        let http = reqwest::Client::builder().timeout(Duration::from_secs(15)).build().unwrap_or_default();
        Envoyeur { fournisseur, http, jeton: Mutex::new(None) }
    }

    pub fn nom(&self) -> &'static str {
        self.fournisseur.nom()
    }

    pub fn simulation(&self) -> bool {
        matches!(self.fournisseur, FournisseurSms::Simulation)
    }

    /// Envoie `message` au numéro `+223XXXXXXXX`.
    pub async fn envoyer(&self, numero: &str, message: &str) -> Result<(), String> {
        match &self.fournisseur {
            FournisseurSms::Simulation => {
                tracing::info!("SMS simulé vers {numero} : {message}");
                Ok(())
            }
            FournisseurSms::Orange(o) => {
                match self.envoyer_orange(o, numero, message).await {
                    // Jeton révoqué avant son expiration : on en redemande un, une fois.
                    Err(e) if e.contains("401") => {
                        *self.jeton.lock().unwrap_or_else(|e| e.into_inner()) = None;
                        self.envoyer_orange(o, numero, message).await
                    }
                    r => r,
                }
            }
        }
    }

    async fn jeton_orange(&self, o: &Orange) -> Result<String, String> {
        if let Some((j, expire)) = self.jeton.lock().unwrap_or_else(|e| e.into_inner()).clone() {
            if expire > maintenant() + 60_000 {
                return Ok(j);
            }
        }
        let basic = base64::engine::general_purpose::STANDARD.encode(format!("{}:{}", o.client_id, o.client_secret));
        let r: Value = self
            .http
            .post(format!("{}/oauth/v3/token", o.api.trim_end_matches('/')))
            .header("Authorization", format!("Basic {basic}"))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("Accept", "application/json")
            .body("grant_type=client_credentials")
            .send()
            .await
            .and_then(|r| r.error_for_status())
            .map_err(|e| format!("jeton Orange : {e}"))?
            .json()
            .await
            .map_err(|e| format!("jeton Orange : {e}"))?;
        let j = r["access_token"].as_str().ok_or("jeton Orange absent")?.to_string();
        let expire = maintenant() + r["expires_in"].as_i64().unwrap_or(3_600) * 1_000;
        *self.jeton.lock().unwrap_or_else(|e| e.into_inner()) = Some((j.clone(), expire));
        Ok(j)
    }

    async fn envoyer_orange(&self, o: &Orange, numero: &str, message: &str) -> Result<(), String> {
        let jeton = self.jeton_orange(o).await?;
        let expediteur = format!("tel:{}", o.expediteur);
        let mut requete = json!({
            "outboundSMSMessageRequest": {
                "address": format!("tel:{numero}"),
                "senderAddress": expediteur,
                "outboundSMSTextMessage": { "message": message },
            }
        });
        if let Some(n) = &o.nom_expediteur {
            requete["outboundSMSMessageRequest"]["senderName"] = json!(n);
        }
        let r = self
            .http
            .post(format!("{}/smsmessaging/v1/outbound/{}/requests", o.api.trim_end_matches('/'), encoder(&expediteur)))
            .bearer_auth(jeton)
            .json(&requete)
            .send()
            .await
            .map_err(|e| format!("SMS Orange : {e}"))?;
        if r.status().is_success() {
            Ok(())
        } else {
            Err(format!("SMS Orange : statut {}", r.status().as_u16()))
        }
    }
}
