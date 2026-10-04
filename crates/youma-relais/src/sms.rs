//! Envoi des codes de vérification (RG-CAN-04) : par SMS **Orange Mali** (API SMS d'Orange Developer), ou
//! simulation tant que le contrat Orange n'est pas signé ; par **WhatsApp** (API WhatsApp Cloud de Meta) si le
//! relais est configuré, au choix du client (fiche 0046). Les identifiants restent sur le relais, seul à envoyer.
//!
//! [HYPOTHÈSE] API « SMS Mali » d'Orange Developer, à confirmer à la signature du contrat :
//! jeton OAuth2 `POST /oauth/v3/token` (client_credentials, authentification Basic), puis
//! `POST /smsmessaging/v1/outbound/tel:+223XXXXXXXX/requests` avec `outboundSMSMessageRequest`.
//!
//! [HYPOTHÈSE] WhatsApp Cloud : `POST https://graph.facebook.com/v21.0/{numero_id}/messages` (jeton permanent d'un
//! utilisateur système), modèle d'**authentification** validé par Meta (`youma_code` par défaut) avec le code en
//! paramètre du corps et du bouton « Copier le code ».

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

/// WhatsApp Cloud (Meta). Facultatif : sans lui, le client n'a que le SMS.
#[derive(Clone, Debug)]
pub struct WhatsApp {
    /// `https://graph.facebook.com/v21.0` (modifiable pour les tests).
    pub api: String,
    /// Identifiant du numéro WhatsApp Business (`phone_number_id`).
    pub numero_id: String,
    pub jeton: String,
    /// Nom du modèle d'authentification validé par Meta.
    pub modele: String,
    /// Langue du modèle (`fr`).
    pub langue: String,
}

impl WhatsApp {
    /// Si `YOUMA_WHATSAPP_JETON` et `YOUMA_WHATSAPP_NUMERO_ID` sont fournis.
    pub fn depuis_environnement() -> Option<WhatsApp> {
        let v = |n: &str| std::env::var(n).ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
        Some(WhatsApp {
            api: v("YOUMA_WHATSAPP_API").unwrap_or_else(|| "https://graph.facebook.com/v21.0".into()),
            numero_id: v("YOUMA_WHATSAPP_NUMERO_ID")?,
            jeton: v("YOUMA_WHATSAPP_JETON")?,
            modele: v("YOUMA_WHATSAPP_MODELE").unwrap_or_else(|| "youma_code".into()),
            langue: v("YOUMA_WHATSAPP_LANGUE").unwrap_or_else(|| "fr".into()),
        })
    }
}

/// Canal choisi par le client pour recevoir son code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Canal {
    Sms,
    WhatsApp,
}

impl Canal {
    pub fn depuis(s: Option<&str>) -> Option<Canal> {
        match s.unwrap_or("sms") {
            "sms" => Some(Canal::Sms),
            "whatsapp" => Some(Canal::WhatsApp),
            _ => None,
        }
    }

    pub fn nom(self) -> &'static str {
        match self {
            Canal::Sms => "sms",
            Canal::WhatsApp => "whatsapp",
        }
    }
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
    whatsapp: Option<WhatsApp>,
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
    pub fn new(fournisseur: FournisseurSms, whatsapp: Option<WhatsApp>) -> Envoyeur {
        let http = reqwest::Client::builder().timeout(Duration::from_secs(15)).build().unwrap_or_default();
        Envoyeur { fournisseur, whatsapp, http, jeton: Mutex::new(None) }
    }

    /// Canaux proposés au client. WhatsApp seulement s'il est configuré, sauf sur un relais tout en simulation
    /// (démonstration) : un code simulé s'affiche sur la page, ce qui n'est acceptable que sans vrai envoi.
    pub fn canaux(&self) -> Vec<&'static str> {
        if self.whatsapp.is_some() || self.simulation() {
            vec!["sms", "whatsapp"]
        } else {
            vec!["sms"]
        }
    }

    /// Le code de ce canal est simulé : renvoyé à la page au lieu d'être envoyé.
    pub fn simule(&self, canal: Canal) -> bool {
        match canal {
            Canal::Sms => self.simulation(),
            Canal::WhatsApp => self.whatsapp.is_none(),
        }
    }

    /// Envoie le code de vérification par le canal choisi (`numero` au format `+223XXXXXXXX`).
    pub async fn envoyer_code(&self, canal: Canal, numero: &str, restaurant: &str, code: &str) -> Result<(), String> {
        match (canal, &self.whatsapp) {
            (Canal::WhatsApp, Some(w)) => self.envoyer_whatsapp(w, numero, code).await,
            (Canal::WhatsApp, None) => {
                tracing::info!("WhatsApp simulé vers {numero} : code {code}");
                Ok(())
            }
            (Canal::Sms, _) => self.envoyer(numero, &format!("{restaurant} : votre code de commande est {code}")).await,
        }
    }

    async fn envoyer_whatsapp(&self, w: &WhatsApp, numero: &str, code: &str) -> Result<(), String> {
        let requete = json!({
            "messaging_product": "whatsapp",
            "to": numero.trim_start_matches('+'),
            "type": "template",
            "template": {
                "name": w.modele,
                "language": { "code": w.langue },
                "components": [
                    { "type": "body", "parameters": [{ "type": "text", "text": code }] },
                    { "type": "button", "sub_type": "url", "index": "0", "parameters": [{ "type": "text", "text": code }] },
                ],
            },
        });
        let r = self
            .http
            .post(format!("{}/{}/messages", w.api.trim_end_matches('/'), w.numero_id))
            .bearer_auth(&w.jeton)
            .json(&requete)
            .send()
            .await
            .map_err(|e| format!("WhatsApp : {e}"))?;
        if r.status().is_success() {
            Ok(())
        } else {
            Err(format!("WhatsApp : statut {}", r.status().as_u16()))
        }
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
