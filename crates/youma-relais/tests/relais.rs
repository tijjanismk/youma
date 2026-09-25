//! Relais Internet : le poste central (simulé ici) synchronise ; le client commande, reçoit un code SMS, suit sa commande.

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use axum::extract::Path;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::post;
use axum::Json;
use serde_json::{json, Value};
use youma_relais::{Config, Etat, FournisseurSms};

const CLE: &str = "cle-de-test-du-relais-0123";

struct Relais {
    url: String,
    client: reqwest::Client,
    _dossier: tempfile::TempDir,
}

async fn relais() -> Relais {
    relais_avec(FournisseurSms::Simulation).await
}

async fn relais_avec(sms: FournisseurSms) -> Relais {
    let dossier = tempfile::tempdir().unwrap();
    let config = Config { dossier_donnees: dossier.path().to_path_buf(), port: 0, cle: CLE.into(), dossier_ui: None, derriere_proxy: false, sms };
    let etat = Etat::ouvrir(&config).unwrap();
    let ecoute = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let adresse = ecoute.local_addr().unwrap();
    tokio::spawn(youma_relais::servir(etat, None, ecoute));
    Relais { url: format!("http://{adresse}/api"), client: reqwest::Client::new(), _dossier: dossier }
}

/// Faux serveur Orange (API SMS) : jeton OAuth2 puis envoi ; garde les messages reçus.
async fn faux_orange() -> (youma_relais::Orange, Arc<Mutex<Vec<Value>>>) {
    let recus = Arc::new(Mutex::new(Vec::new()));
    let r = recus.clone();
    let app = axum::Router::new()
        .route(
            "/oauth/v3/token",
            post(|h: HeaderMap, corps: String| async move {
                // Basic base64("id:secret")
                assert_eq!(h["authorization"], "Basic aWQ6c2VjcmV0");
                assert_eq!(corps, "grant_type=client_credentials");
                Json(json!({ "token_type": "Bearer", "access_token": "JETON", "expires_in": 3600 }))
            }),
        )
        .route(
            "/smsmessaging/v1/outbound/{expediteur}/requests",
            post(move |Path(expediteur): Path<String>, h: HeaderMap, Json(v): Json<Value>| {
                let r = r.clone();
                async move {
                    assert_eq!(h["authorization"], "Bearer JETON");
                    assert_eq!(expediteur, "tel:+22370000000");
                    r.lock().unwrap().push(v);
                    (StatusCode::CREATED, Json(json!({ "outboundSMSMessageRequest": {} })))
                }
            }),
        );
    let ecoute = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let a: SocketAddr = ecoute.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(ecoute, app).await });
    let orange = youma_relais::Orange {
        api: format!("http://{a}"),
        client_id: "id".into(),
        client_secret: "secret".into(),
        expediteur: "+22370000000".into(),
        nom_expediteur: Some("Baobab".into()),
    };
    (orange, recus)
}

impl Relais {
    async fn post(&self, chemin: &str, corps: Value) -> (u16, Value) {
        let r = self.client.post(format!("{}{chemin}", self.url)).json(&corps).send().await.unwrap();
        (r.status().as_u16(), r.json().await.unwrap_or(Value::Null))
    }
    async fn get(&self, chemin: &str) -> (u16, Value) {
        let r = self.client.get(format!("{}{chemin}", self.url)).send().await.unwrap();
        (r.status().as_u16(), r.json().await.unwrap_or(Value::Null))
    }
    async fn synchroniser(&self, cle: &str, corps: Value) -> (u16, Value) {
        let r = self.client.post(format!("{}/relais/synchroniser", self.url)).bearer_auth(cle).json(&corps).send().await.unwrap();
        (r.status().as_u16(), r.json().await.unwrap_or(Value::Null))
    }
}

fn menu() -> Value {
    json!({ "restaurant": "Maquis Le Baobab", "telephone": "70000000", "ouvert": true, "table": null, "qr_table": true, "en_ligne": true,
            "paiement_avance": true, "paiement_a_la_livraison": true, "verification_numero": "sms", "operateurs": ["Orange Money"],
            "quartiers": [], "categories": [], "produits": [] })
}

fn commande(code: &str) -> Value {
    json!({ "canal": "en_ligne", "type": "livraison", "telephone": "+223 76 00 00 01", "code_verification": code,
            "livraison": { "quartier": "Hamdallaye", "repere": "École", "telephone": "76000001" },
            "paiement_mode": "a_la_livraison", "lignes": [{ "produit_id": "p", "quantite": 1 }] })
}

#[tokio::test]
async fn parcours_relais_sms_suivi_et_position() {
    let (orange, sms) = faux_orange().await;
    let r = relais_avec(FournisseurSms::Orange(orange)).await;
    // Avant la première synchronisation : rien à montrer.
    assert_eq!(r.get("/public/menu").await.0, 503);
    // Mauvaise clé : refusé.
    assert_eq!(r.synchroniser("mauvaise", json!({ "menu": menu() })).await.0, 401);
    let config = json!({ "verification_numero": "sms" });
    let (code, s) = r.synchroniser(CLE, json!({ "menu": menu(), "config": config })).await;
    assert_eq!(code, 200, "{s}");
    let (_, m) = r.get("/public/menu").await;
    assert_eq!(m["ouvert"], true);
    assert_eq!(r.get("/public/menu?table=ABCDEF").await.0, 403, "le QR des tables reste local");

    // Code SMS : faux code refusé, bon code accepté.
    let (code, v) = r.post("/public/verification", json!({ "telephone": "76 00 00 01" })).await;
    assert_eq!(code, 200, "{v}");
    assert!(v.get("code").is_none(), "avec Orange, le code ne part que par SMS");
    let envoi = sms.lock().unwrap()[0]["outboundSMSMessageRequest"].clone();
    assert_eq!(envoi["address"], "tel:+22376000001");
    assert_eq!(envoi["senderAddress"], "tel:+22370000000");
    assert_eq!(envoi["senderName"], "Baobab");
    let texte = envoi["outboundSMSTextMessage"]["message"].as_str().unwrap().to_string();
    let code_sms = texte.rsplit(' ').next().unwrap().to_string();
    assert_eq!(code_sms.len(), 4);
    let faux = if code_sms == "0000" { "1111" } else { "0000" };
    let (_, rep) = r.post("/public/commandes", commande(faux)).await;
    assert_eq!(rep["statut"], "refusee");
    let (_, rep) = r.post("/public/commandes", commande(&code_sms)).await;
    assert_eq!(rep["statut"], "en_attente", "{rep}");
    let code_suivi = rep["code_suivi"].as_str().unwrap().to_string();
    let (_, s) = r.get(&format!("/public/suivi/{code_suivi}")).await;
    assert_eq!(s["etape"], "recue");
    // Le code ne sert qu'une fois.
    assert_eq!(r.post("/public/commandes", commande(&code_sms)).await.1["statut"], "refusee");

    // Le poste reprend la commande, attestée vérifiée, sans le code SMS.
    let (_, s) = r.synchroniser(CLE, json!({ "menu": menu(), "config": config })).await;
    let c = &s["commandes"][0];
    assert_eq!(c["telephone_verifie"], true);
    assert_eq!(c["code_suivi"], code_suivi.as_str());
    assert!(c.get("code_verification").is_none());
    let origine = c["origine_id"].as_str().unwrap().to_string();
    // Transmise une fois : pas renvoyée tout de suite.
    let (_, s) = r.synchroniser(CLE, json!({ "menu": menu(), "config": config })).await;
    assert_eq!(s["commandes"].as_array().unwrap().len(), 0);

    // Le poste publie le suivi (commande acceptée, livreur en route) avec le lien du livreur.
    let suivi = json!({ "numero": 12, "restaurant": "Maquis Le Baobab", "etape": "en_route", "motif": null, "type": "livraison",
                        "total": 2500, "reste": 2500, "paiement_mode": "a_la_livraison", "lignes": [[1, "Brochettes"]],
                        "livreur": null, "destination": null, "mis_a_jour": 0 });
    r.synchroniser(CLE, json!({ "menu": menu(), "config": config,
        "resultats": [{ "origine_id": origine, "reponse": { "statut": "en_attente", "numero": 12, "total": 2500 } }],
        "suivis": [{ "code_suivi": code_suivi, "code_livreur": "LIVREURCODE1", "suivi": suivi }] })).await;
    assert_eq!(r.post("/public/position/INCONNU", json!({ "lat": 12_640_000, "lon": -8_000_000 })).await.0, 404);
    assert_eq!(r.post("/public/position/LIVREURCODE1", json!({ "lat": 12_640_000, "lon": -8_000_000 })).await.0, 200);
    let (_, s) = r.get(&format!("/public/suivi/{}", code_suivi.to_lowercase())).await;
    assert_eq!(s["numero"], 12);
    assert_eq!(s["livreur"][0], 12_640_000, "position la plus fraîche, vue sur le relais");
    // Le poste reçoit la position à la synchronisation suivante, une seule fois.
    let (_, s) = r.synchroniser(CLE, json!({ "menu": menu(), "config": config, "suivis": [{ "code_suivi": code_suivi, "code_livreur": "LIVREURCODE1", "suivi": suivi }] })).await;
    assert_eq!(s["positions"][0]["code_livreur"], "LIVREURCODE1");
    let (_, s) = r.synchroniser(CLE, json!({ "menu": menu(), "config": config, "suivis": [{ "code_suivi": code_suivi, "code_livreur": "LIVREURCODE1", "suivi": suivi }] })).await;
    assert_eq!(s["positions"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn refus_du_poste_et_anti_abus() {
    let r = relais().await;
    let mut m = menu();
    m["verification_numero"] = json!("rappel");
    r.synchroniser(CLE, json!({ "menu": m, "config": { "verification_numero": "rappel" } })).await;
    // Vérification par rappel : pas de code SMS.
    assert_eq!(r.post("/public/verification", json!({ "telephone": "76000001" })).await.0, 403);
    let (_, rep) = r.post("/public/commandes", commande("")).await;
    assert_eq!(rep["statut"], "en_attente");
    let code = rep["code_suivi"].as_str().unwrap().to_string();
    let (_, s) = r.synchroniser(CLE, json!({ "menu": m })).await;
    assert_eq!(s["commandes"][0]["telephone_verifie"], false);
    let origine = s["commandes"][0]["origine_id"].clone();
    // Zone à risque côté poste : refus transmis au client.
    r.synchroniser(CLE, json!({ "menu": m, "resultats": [{ "origine_id": origine, "reponse": { "statut": "refusee", "message": "Livraison indisponible à Kalaban la nuit" } }] })).await;
    let (_, s) = r.get(&format!("/public/suivi/{code}")).await;
    assert_eq!(s["etape"], "refusee");
    assert_eq!(s["motif"], "Livraison indisponible à Kalaban la nuit");
    // Canal QR : jamais par le relais.
    let mut qr = commande("");
    qr["canal"] = json!("qr_table");
    assert_eq!(r.post("/public/commandes", qr).await.1["statut"], "refusee");
    // Au plus 10 commandes par 10 minutes depuis la même adresse.
    let mut refus = 0;
    for _ in 0..12 {
        if r.post("/public/commandes", commande("")).await.0 == 429 {
            refus += 1;
        }
    }
    assert!(refus >= 2, "{refus}");
    // Menu en ligne désactivé : 403.
    m["en_ligne"] = json!(false);
    r.synchroniser(CLE, json!({ "menu": m })).await;
    assert_eq!(r.get("/public/menu").await.0, 403);
}

#[tokio::test]
async fn sms_simule_tant_qu_orange_n_est_pas_configure() {
    let r = relais().await;
    let config = json!({ "verification_numero": "sms" });
    r.synchroniser(CLE, json!({ "menu": menu(), "config": config })).await;
    let (_, etat) = r.get("/etat").await;
    assert_eq!(etat["sms"], "simulation");
    let (code, v) = r.post("/public/verification", json!({ "telephone": "76000001" })).await;
    assert_eq!(code, 200, "{v}");
    assert_eq!(v["simulation"], true);
    let code_sms = v["code"].as_str().unwrap().to_string();
    let (_, rep) = r.post("/public/commandes", commande(&code_sms)).await;
    assert_eq!(rep["statut"], "en_attente", "{rep}");
    let (_, s) = r.synchroniser(CLE, json!({ "menu": menu(), "config": config })).await;
    assert_eq!(s["sms"], "simulation");
    assert_eq!(s["commandes"][0]["telephone_verifie"], true);
}

/// Cloud multi-restaurants : résumés, SMS de clôture une seule fois, espace propriétaire, sauvegardes chiffrées.
#[tokio::test]
async fn cloud_resumes_proprietaire_et_sauvegardes() {
    let dossier = tempfile::tempdir().unwrap();
    let cle_a = youma_relais::inscrire_restaurant(dossier.path(), "Maquis A").unwrap();
    let cle_b = youma_relais::inscrire_restaurant(dossier.path(), "Maquis B").unwrap();
    let config = Config { dossier_donnees: dossier.path().to_path_buf(), port: 0, cle: CLE.into(), dossier_ui: None, derriere_proxy: false, sms: FournisseurSms::Simulation };
    let etat = Etat::ouvrir(&config).unwrap();
    let ecoute = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/api", ecoute.local_addr().unwrap());
    tokio::spawn(youma_relais::servir(etat, None, ecoute));
    let c = reqwest::Client::new();
    let hash = youma_core::auth::hacher("mot-de-passe-proprio").unwrap();
    let resume = |date: &str, ca: i64, cloturee: bool| {
        json!({ "date": date, "cloturee": cloturee, "chiffre_affaires": ca, "commandes": 3, "depenses": 0, "encaissements": [["especes", ca]],
                "mobile_money_a_verifier": [0, 0], "annulations": [0, 0], "ecarts_caisse": 0, "mis_a_jour": 0 })
    };
    let sync = |cle: String, corps: Value| {
        let c = c.clone();
        let url = url.clone();
        async move {
            let r = c.post(format!("{url}/cloud/synchroniser")).bearer_auth(cle).json(&corps).send().await.unwrap();
            (r.status().as_u16(), r.json::<Value>().await.unwrap_or(Value::Null))
        }
    };
    let corps = |nom: &str, resumes: Vec<Value>| json!({ "restaurant": nom, "telephone_proprietaire": "+223 76 00 00 01", "mdp_hash": hash, "sms_resume": true, "resumes": resumes });
    assert_eq!(sync("inconnue".into(), corps("X", vec![])).await.0, 401);
    // Journée en cours : pas de SMS ; clôturée : un SMS, une seule fois.
    assert_eq!(sync(cle_a.clone(), corps("Maquis A", vec![resume("2026-03-14", 5_000, false)])).await.1["sms_envoyes"], 0);
    assert_eq!(sync(cle_a.clone(), corps("Maquis A", vec![resume("2026-03-14", 8_000, true)])).await.1["sms_envoyes"], 1);
    assert_eq!(sync(cle_a.clone(), corps("Maquis A", vec![resume("2026-03-14", 8_000, true)])).await.1["sms_envoyes"], 0);
    sync(cle_b.clone(), corps("Maquis B", vec![resume("2026-03-14", 2_000, false)])).await;

    // Espace propriétaire : les deux maquis, et le total de la journée.
    let r = c.post(format!("{url}/proprietaire/connexion")).json(&json!({ "telephone": "76000001", "mot_de_passe": "faux" })).send().await.unwrap();
    assert_eq!(r.status().as_u16(), 401);
    let r: Value = c
        .post(format!("{url}/proprietaire/connexion"))
        .json(&json!({ "telephone": "76 00 00 01", "mot_de_passe": "mot-de-passe-proprio" }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(r["restaurants"], 2);
    let jeton = r["jeton"].as_str().unwrap().to_string();
    let t: Value = c.get(format!("{url}/proprietaire/tableau")).bearer_auth(&jeton).send().await.unwrap().json().await.unwrap();
    assert_eq!(t["restaurants"].as_array().unwrap().len(), 2);
    assert_eq!(t["totaux"][0], json!({ "date": "2026-03-14", "chiffre_affaires": 10_000, "commandes": 6 }));
    assert_eq!(c.get(format!("{url}/proprietaire/tableau")).bearer_auth("faux").send().await.unwrap().status().as_u16(), 401);

    // Sauvegardes : chiffrées seulement, 14 gardées, invisibles des autres restaurants.
    let envoyer = |cle: String, nom: String, octets: Vec<u8>| {
        let c = c.clone();
        let url = url.clone();
        async move { c.put(format!("{url}/cloud/sauvegardes/envoi/{nom}")).bearer_auth(cle).body(octets).send().await.unwrap().status().as_u16() }
    };
    assert_eq!(envoyer(cle_a.clone(), "clair.db".into(), b"SQLite format 3 non chiffre........................".to_vec()).await, 422);
    let chiffre = youma_core::cloud::chiffrer(b"base du maquis A", "phrase du maquis A !").unwrap();
    for i in 0..16 {
        assert_eq!(envoyer(cle_a.clone(), format!("youma-{i:02}.db"), chiffre.clone()).await, 200);
    }
    let liste: Value = c.get(format!("{url}/cloud/sauvegardes")).bearer_auth(&cle_a).send().await.unwrap().json().await.unwrap();
    assert_eq!(liste.as_array().unwrap().len(), 14);
    assert_eq!(liste[0]["nom"], "youma-15.db");
    let id = liste[0]["id"].as_str().unwrap();
    let octets = c.get(format!("{url}/cloud/sauvegardes/{id}")).bearer_auth(&cle_a).send().await.unwrap().bytes().await.unwrap();
    assert_eq!(youma_core::cloud::dechiffrer(&octets, "phrase du maquis A !").unwrap(), b"base du maquis A");
    assert_eq!(c.get(format!("{url}/cloud/sauvegardes/{id}")).bearer_auth(&cle_b).send().await.unwrap().status().as_u16(), 404);
}
