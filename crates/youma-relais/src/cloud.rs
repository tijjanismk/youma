//! Cloud facultatif (fiche 0018, RG-CLO-01 à 05), multi-restaurants : résumés de journée, sauvegardes chiffrées,
//! résumé SMS au propriétaire, espace propriétaire. Le cloud ne voit jamais le contenu des sauvegardes.

use std::collections::HashMap;

use axum::body::Bytes;
use axum::extract::{DefaultBodyLimit, Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use rand::Rng;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Deserialize;
use serde_json::{json, Value};
use youma_core::cloud::ResumeJournee;
use youma_core::zones_risque::normaliser_telephone;

use crate::{empreinte, erreur, interne, maintenant, Echec, Etat, Rep};

pub const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS cloud_restaurants (
    id TEXT PRIMARY KEY, nom TEXT NOT NULL, empreinte_cle TEXT NOT NULL UNIQUE,
    telephone TEXT NOT NULL DEFAULT '', mdp_hash TEXT NOT NULL DEFAULT '', sms_resume INTEGER NOT NULL DEFAULT 0,
    dernier_contact INTEGER, cree_le INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS cloud_resumes (
    restaurant_id TEXT NOT NULL, date TEXT NOT NULL, contenu TEXT NOT NULL, recu_le INTEGER NOT NULL,
    sms_envoye_le INTEGER, PRIMARY KEY (restaurant_id, date));
CREATE TABLE IF NOT EXISTS cloud_sauvegardes (
    id TEXT PRIMARY KEY, restaurant_id TEXT NOT NULL, nom TEXT NOT NULL, taille INTEGER NOT NULL, recu_le INTEGER NOT NULL);
";

/// Sauvegardes gardées par restaurant (les plus récentes).
pub const SAUVEGARDES_GARDEES: i64 = 14;
const TAILLE_MAX: usize = 512 * 1024 * 1024;

fn hex(o: &[u8]) -> String {
    o.iter().map(|b| format!("{b:02x}")).collect()
}

/// Inscription d'un restaurant par le fournisseur (`youma-relais --ajouter-restaurant NOM`) : renvoie sa clé.
pub fn ajouter_restaurant(conn: &Connection, nom: &str) -> rusqlite::Result<String> {
    let cle: String = (0..32).map(|_| format!("{:x}", rand::thread_rng().gen_range(0..16))).collect();
    conn.execute(
        "INSERT INTO cloud_restaurants(id, nom, empreinte_cle, cree_le) VALUES (?1, ?2, ?3, ?4)",
        params![uuid::Uuid::now_v7().to_string(), nom.trim(), hex(&empreinte(&cle)), maintenant()],
    )?;
    Ok(cle)
}

pub fn routes() -> Router<Etat> {
    Router::new()
        .route("/api/cloud/synchroniser", post(synchroniser))
        .route("/api/cloud/sauvegardes", get(sauvegardes))
        .route("/api/cloud/sauvegardes/{id}", get(telecharger))
        .route("/api/cloud/sauvegardes/envoi/{nom}", put(recevoir_sauvegarde).layer(DefaultBodyLimit::max(TAILLE_MAX)))
        .route("/api/proprietaire/connexion", post(connexion))
        .route("/api/proprietaire/tableau", get(tableau))
}

/// Restaurant authentifié par sa clé (`Authorization: Bearer …`).
fn restaurant(e: &Etat, entetes: &HeaderMap) -> Result<String, Echec> {
    let cle = entetes.get("authorization").and_then(|v| v.to_str().ok()).and_then(|v| v.strip_prefix("Bearer ")).unwrap_or("");
    e.avec(|c| {
        c.query_row("SELECT id FROM cloud_restaurants WHERE empreinte_cle = ?1", [hex(&empreinte(cle))], |r| r.get::<_, String>(0))
            .optional()
    })?
    .ok_or_else(|| erreur(StatusCode::UNAUTHORIZED, "NON_AUTHENTIFIE", "Restaurant inconnu du cloud"))
}

#[derive(Deserialize)]
struct Synchronisation {
    restaurant: String,
    #[serde(default)]
    telephone_proprietaire: String,
    #[serde(default)]
    mdp_hash: String,
    #[serde(default)]
    sms_resume: bool,
    #[serde(default)]
    resumes: Vec<ResumeJournee>,
}

/// RG-CLO-01 / 04 : résumés reçus (dernière version par journée) ; SMS une seule fois à la clôture.
async fn synchroniser(State(e): State<Etat>, entetes: HeaderMap, Json(s): Json<Synchronisation>) -> Rep {
    let id = restaurant(&e, &entetes)?;
    let t = maintenant();
    let telephone = normaliser_telephone(&s.telephone_proprietaire);
    let a_envoyer: Vec<ResumeJournee> = e.avec(|c| {
        c.execute(
            "UPDATE cloud_restaurants SET nom = ?1, telephone = ?2, mdp_hash = ?3, sms_resume = ?4, dernier_contact = ?5 WHERE id = ?6",
            params![s.restaurant.trim(), telephone, s.mdp_hash, s.sms_resume, t, id],
        )?;
        let mut v = Vec::new();
        for r in &s.resumes {
            c.execute(
                "INSERT INTO cloud_resumes(restaurant_id, date, contenu, recu_le) VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(restaurant_id, date) DO UPDATE SET contenu = excluded.contenu, recu_le = excluded.recu_le",
                params![id, r.date, serde_json::to_string(r).unwrap_or_default(), t],
            )?;
            let deja: Option<i64> = c.query_row(
                "SELECT sms_envoye_le FROM cloud_resumes WHERE restaurant_id = ?1 AND date = ?2",
                params![id, r.date],
                |x| x.get(0),
            )?;
            if r.cloturee && s.sms_resume && telephone.len() == 8 && deja.is_none() {
                v.push(r.clone());
            }
        }
        Ok(v)
    })?;
    let mut sms = 0;
    for r in a_envoyer {
        let texte = youma_core::cloud::texte_sms(&s.restaurant, &r);
        match e.sms.envoyer(&format!("+223{telephone}"), &texte).await {
            Ok(()) => {
                e.avec(|c| c.execute("UPDATE cloud_resumes SET sms_envoye_le = ?1 WHERE restaurant_id = ?2 AND date = ?3", params![maintenant(), id, r.date]))?;
                sms += 1;
            }
            // Réessayé à la synchronisation suivante.
            Err(err) => tracing::warn!("Résumé SMS non envoyé : {err}"),
        }
    }
    Ok(Json(json!({ "sms": e.sms.nom(), "sms_envoyes": sms })))
}

fn nom_sur(nom: &str) -> String {
    nom.chars().filter(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.')).take(80).collect()
}

/// RG-CLO-02 : sauvegarde chiffrée reçue ; seules les 14 plus récentes sont gardées.
async fn recevoir_sauvegarde(State(e): State<Etat>, entetes: HeaderMap, Path(nom): Path<String>, corps: Bytes) -> Rep {
    let id_resto = restaurant(&e, &entetes)?;
    if corps.len() < 46 || &corps[..6] != b"YOUMA1" {
        return Err(erreur(StatusCode::UNPROCESSABLE_ENTITY, "VALIDATION", "Seules les sauvegardes chiffrées sont acceptées"));
    }
    let id = uuid::Uuid::now_v7().to_string();
    let dossier = e.dossier.join("sauvegardes").join(&id_resto);
    std::fs::create_dir_all(&dossier).map_err(interne)?;
    std::fs::write(dossier.join(&id), &corps).map_err(interne)?;
    let anciennes: Vec<String> = e.avec(|c| {
        c.execute(
            "INSERT INTO cloud_sauvegardes(id, restaurant_id, nom, taille, recu_le) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, id_resto, nom_sur(&nom), corps.len() as i64, maintenant()],
        )?;
        let v: Vec<String> = c
            .prepare("SELECT id FROM cloud_sauvegardes WHERE restaurant_id = ?1 ORDER BY recu_le DESC, id DESC LIMIT -1 OFFSET ?2")?
            .query_map(params![id_resto, SAUVEGARDES_GARDEES], |r| r.get(0))?
            .collect::<Result<_, _>>()?;
        for a in &v {
            c.execute("DELETE FROM cloud_sauvegardes WHERE id = ?1", [a])?;
        }
        Ok(v)
    })?;
    for a in anciennes {
        let _ = std::fs::remove_file(dossier.join(a));
    }
    Ok(Json(json!({ "id": id })))
}

async fn sauvegardes(State(e): State<Etat>, entetes: HeaderMap) -> Rep {
    let id = restaurant(&e, &entetes)?;
    let v: Vec<Value> = e.avec(|c| {
        c.prepare("SELECT id, nom, taille, recu_le FROM cloud_sauvegardes WHERE restaurant_id = ?1 ORDER BY recu_le DESC")?
            .query_map([&id], |r| {
                Ok(json!({ "id": r.get::<_, String>(0)?, "nom": r.get::<_, String>(1)?, "taille": r.get::<_, i64>(2)?, "recu_le": r.get::<_, i64>(3)? }))
            })?
            .collect()
    })?;
    Ok(Json(json!(v)))
}

async fn telecharger(State(e): State<Etat>, entetes: HeaderMap, Path(id): Path<String>) -> Result<Response, Echec> {
    let resto = restaurant(&e, &entetes)?;
    let existe: bool = e.avec(|c| {
        c.query_row("SELECT COUNT(*) FROM cloud_sauvegardes WHERE id = ?1 AND restaurant_id = ?2", params![id, resto], |r| r.get::<_, i64>(0))
            .map(|n| n > 0)
    })?;
    if !existe {
        return Err(erreur(StatusCode::NOT_FOUND, "NON_TROUVE", "Sauvegarde introuvable"));
    }
    let octets = std::fs::read(e.dossier.join("sauvegardes").join(&resto).join(nom_sur(&id))).map_err(interne)?;
    Ok(([("content-type", "application/octet-stream")], octets).into_response())
}

// ───────────── Espace propriétaire ─────────────

#[derive(Deserialize)]
struct Connexion {
    telephone: String,
    mot_de_passe: String,
}

/// RG-CLO-03 : le propriétaire se connecte avec son numéro et le mot de passe défini sur ses postes ;
/// il voit tous les restaurants dont le poste a envoyé ce numéro et l'empreinte de ce mot de passe.
async fn connexion(State(e): State<Etat>, Json(c): Json<Connexion>) -> Rep {
    let tel = normaliser_telephone(&c.telephone);
    if !e.limiter(&format!("proprio:{tel}"), 10, 15 * 60_000) {
        return Err(erreur(StatusCode::TOO_MANY_REQUESTS, "TROP_DE_DEMANDES", "Trop d'essais : réessayez dans 15 minutes"));
    }
    let candidats: Vec<(String, String)> = e.avec(|x| {
        x.prepare("SELECT id, mdp_hash FROM cloud_restaurants WHERE telephone = ?1 AND mdp_hash <> ''")?
            .query_map([&tel], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect()
    })?;
    let mdp = c.mot_de_passe.clone();
    let ids: Vec<String> = tokio::task::spawn_blocking(move || {
        candidats.into_iter().filter(|(_, h)| youma_core::auth::verifier(&mdp, h)).map(|(id, _)| id).collect()
    })
    .await
    .map_err(interne)?;
    if ids.is_empty() {
        return Err(erreur(StatusCode::UNAUTHORIZED, "NON_AUTHENTIFIE", "Numéro ou mot de passe incorrect"));
    }
    let jeton = youma_core::auth::nouveau_jeton();
    let mut s = e.sessions.lock().unwrap_or_else(|x| x.into_inner());
    s.retain(|_, (_, exp)| *exp > maintenant());
    s.insert(jeton.clone(), (ids.clone(), maintenant() + 12 * 3_600_000));
    Ok(Json(json!({ "jeton": jeton, "restaurants": ids.len() })))
}

/// RG-CLO-05 : consultation agrégée des restaurants du propriétaire (30 dernières journées).
async fn tableau(State(e): State<Etat>, entetes: HeaderMap) -> Rep {
    let jeton = entetes.get("authorization").and_then(|v| v.to_str().ok()).and_then(|v| v.strip_prefix("Bearer ")).unwrap_or("");
    let ids = e
        .sessions
        .lock()
        .unwrap_or_else(|x| x.into_inner())
        .get(jeton)
        .filter(|(_, exp)| *exp > maintenant())
        .map(|(ids, _)| ids.clone())
        .ok_or_else(|| erreur(StatusCode::UNAUTHORIZED, "NON_AUTHENTIFIE", "Connectez-vous"))?;
    let mut restaurants = Vec::new();
    let mut totaux: HashMap<String, (i64, i64)> = HashMap::new();
    for id in ids {
        let (nom, contact): (String, Option<i64>) =
            e.avec(|c| c.query_row("SELECT nom, dernier_contact FROM cloud_restaurants WHERE id = ?1", [&id], |r| Ok((r.get(0)?, r.get(1)?))))?;
        let resumes: Vec<ResumeJournee> = e.avec(|c| {
            c.prepare("SELECT contenu FROM cloud_resumes WHERE restaurant_id = ?1 ORDER BY date DESC LIMIT 30")?
                .query_map([&id], |r| r.get::<_, String>(0))?
                .map(|x| x.map(|s| serde_json::from_str(&s).ok()))
                .filter_map(|x| x.transpose())
                .collect()
        })?;
        for r in &resumes {
            let t = totaux.entry(r.date.clone()).or_default();
            t.0 += r.chiffre_affaires;
            t.1 += r.commandes;
        }
        let derniere_sauvegarde: Option<i64> =
            e.avec(|c| c.query_row("SELECT MAX(recu_le) FROM cloud_sauvegardes WHERE restaurant_id = ?1", [&id], |r| r.get(0)))?;
        restaurants.push(json!({ "id": id, "nom": nom, "dernier_contact": contact, "derniere_sauvegarde": derniere_sauvegarde, "resumes": resumes }));
    }
    let mut totaux: Vec<Value> = totaux.into_iter().map(|(d, (ca, n))| json!({ "date": d, "chiffre_affaires": ca, "commandes": n })).collect();
    totaux.sort_by(|a, b| b["date"].as_str().cmp(&a["date"].as_str()));
    Ok(Json(json!({ "restaurants": restaurants, "totaux": totaux })))
}
