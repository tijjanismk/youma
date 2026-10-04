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
    relais_complet(sms, None).await
}

async fn relais_complet(sms: FournisseurSms, whatsapp: Option<youma_relais::WhatsApp>) -> Relais {
    let dossier = tempfile::tempdir().unwrap();
    let config = Config { dossier_donnees: dossier.path().to_path_buf(), port: 0, cle: CLE.into(), dossier_ui: None, derriere_proxy: false, sms, whatsapp };
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

/// Faux WhatsApp Cloud (Meta) : garde les messages reçus.
async fn faux_whatsapp() -> (youma_relais::WhatsApp, Arc<Mutex<Vec<Value>>>) {
    let recus = Arc::new(Mutex::new(Vec::new()));
    let r = recus.clone();
    let app = axum::Router::new().route(
        "/{numero}/messages",
        post(move |Path(numero): Path<String>, h: HeaderMap, Json(v): Json<Value>| {
            let r = r.clone();
            async move {
                assert_eq!(h["authorization"], "Bearer JETON-META");
                assert_eq!(numero, "1234567890");
                r.lock().unwrap().push(v);
                Json(json!({ "messages": [{ "id": "wamid.x" }] }))
            }
        }),
    );
    let ecoute = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let a: SocketAddr = ecoute.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(ecoute, app).await });
    let w = youma_relais::WhatsApp {
        api: format!("http://{a}"),
        numero_id: "1234567890".into(),
        jeton: "JETON-META".into(),
        modele: "youma_code".into(),
        langue: "fr".into(),
    };
    (w, recus)
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

/// Fiche 0046 : le client choisit SMS ou WhatsApp, vérifie son numéro une fois et commande ensuite sans code.
#[tokio::test]
async fn code_par_whatsapp_puis_numero_reconnu() {
    let (orange, sms) = faux_orange().await;
    let (whatsapp, wa) = faux_whatsapp().await;
    let r = relais_complet(FournisseurSms::Orange(orange), Some(whatsapp)).await;
    let config = json!({ "verification_numero": "sms" });
    r.synchroniser(CLE, json!({ "menu": menu(), "config": config })).await;
    let (_, m) = r.get("/public/menu").await;
    assert_eq!(m["canaux_verification"], json!(["sms", "whatsapp"]));
    assert_eq!(r.post("/public/verification", json!({ "telephone": "76000001", "canal": "pigeon" })).await.0, 422);

    let (code, v) = r.post("/public/verification", json!({ "telephone": "76 00 00 01", "canal": "whatsapp" })).await;
    assert_eq!(code, 200, "{v}");
    assert!(v.get("code").is_none(), "WhatsApp configuré : le code ne part que par WhatsApp");
    assert!(sms.lock().unwrap().is_empty(), "aucun SMS");
    let message = wa.lock().unwrap()[0].clone();
    assert_eq!(message["to"], "22376000001");
    assert_eq!(message["template"]["name"], "youma_code");
    let code_wa = message["template"]["components"][0]["parameters"][0]["text"].as_str().unwrap().to_string();
    assert_eq!(message["template"]["components"][1]["parameters"][0]["text"], code_wa.as_str());

    // Mauvais code : refusé ; bon code : un jeton pour ce téléphone.
    let faux = if code_wa == "0000" { "1111" } else { "0000" };
    assert_eq!(r.post("/public/verification/confirmer", json!({ "telephone": "76000001", "code": faux })).await.0, 422);
    let (code, v) = r.post("/public/verification/confirmer", json!({ "telephone": "+223 76 00 00 01", "code": code_wa })).await;
    assert_eq!(code, 200, "{v}");
    let jeton = v["jeton_client"].as_str().unwrap().to_string();
    assert_eq!(v["telephone"], "76000001");
    // Le code est consommé.
    assert_eq!(r.post("/public/verification/confirmer", json!({ "telephone": "76000001", "code": code_wa })).await.0, 422);

    // Commandes suivantes : le jeton suffit, sans code.
    let avec_jeton = |j: &str, n: i64| {
        let mut c = commande("");
        c["jeton_client"] = json!(j);
        c["lignes"][0]["quantite"] = json!(n);
        c
    };
    let (_, rep) = r.post("/public/commandes", avec_jeton(&jeton, 1)).await;
    assert_eq!(rep["statut"], "en_attente", "{rep}");
    let (_, rep) = r.post("/public/commandes", avec_jeton(&jeton, 2)).await;
    assert_eq!(rep["statut"], "en_attente", "{rep}");
    // Faux jeton, ou jeton d'un autre numéro : refusé.
    assert_eq!(r.post("/public/commandes", avec_jeton("FAUX", 3)).await.1["statut"], "refusee");
    let mut autre = avec_jeton(&jeton, 3);
    autre["telephone"] = json!("76000002");
    assert_eq!(r.post("/public/commandes", autre).await.1["statut"], "refusee");

    // Le poste reçoit des commandes vérifiées, sans le jeton.
    let (_, s) = r.synchroniser(CLE, json!({ "menu": menu(), "config": config })).await;
    let commandes = s["commandes"].as_array().unwrap();
    assert_eq!(commandes.len(), 2);
    assert!(commandes.iter().all(|c| c["telephone_verifie"] == true && c.get("jeton_client").is_none()));
}

#[tokio::test]
async fn whatsapp_propose_seulement_s_il_est_configure() {
    let (orange, _) = faux_orange().await;
    let r = relais_avec(FournisseurSms::Orange(orange)).await;
    r.synchroniser(CLE, json!({ "menu": menu(), "config": { "verification_numero": "sms" } })).await;
    assert_eq!(r.get("/public/menu").await.1["canaux_verification"], json!(["sms"]));
    assert_eq!(r.post("/public/verification", json!({ "telephone": "76000001", "canal": "whatsapp" })).await.0, 422);
    // Relais de démonstration (tout simulé) : les deux, le code s'affiche.
    let r = relais().await;
    r.synchroniser(CLE, json!({ "menu": menu(), "config": { "verification_numero": "sms" } })).await;
    assert_eq!(r.get("/public/menu").await.1["canaux_verification"], json!(["sms", "whatsapp"]));
    let (_, v) = r.post("/public/verification", json!({ "telephone": "76000001", "canal": "whatsapp" })).await;
    assert_eq!(v["simulation"], true);
    assert_eq!(v["code"].as_str().unwrap().len(), 4);
}

/// RG-LIV-05 (fiche 0047) : le livreur se connecte par téléphone + PIN et voit ses courses publiées par le poste.
#[tokio::test]
async fn livreur_connecte_voit_ses_courses() {
    let r = relais().await;
    let hash = youma_core::auth::hacher("6666").unwrap();
    let course = json!({ "code_livreur": "LIVREURCODE1", "numero": 12, "statut": "assignee", "client_nom": "Awa", "telephone": "76000001",
                         "quartier": "Hamdallaye", "repere": "École", "lat": 12_640_000, "lon": -8_000_000, "reste": 2500 });
    let livreurs = json!([{ "employe_id": "e1", "nom": "Ibrahim", "telephone": "76554433", "pin_hash": hash, "courses": [course] }]);
    r.synchroniser(CLE, json!({ "menu": menu(), "livreurs": livreurs })).await;

    let connexion = |tel: &str, pin: &str| json!({ "telephone": tel, "pin": pin });
    assert_eq!(r.post("/public/livreur/connexion", connexion("76554433", "0000")).await.0, 401);
    assert_eq!(r.post("/public/livreur/connexion", connexion("76000000", "6666")).await.0, 401);
    let (code, v) = r.post("/public/livreur/connexion", connexion("+223 76 55 44 33", "6666")).await;
    assert_eq!(code, 200, "{v}");
    assert_eq!(v["nom"], "Ibrahim");
    assert_eq!(v["restaurant"], "Maquis Le Baobab");
    let jeton = v["jeton"].as_str().unwrap().to_string();

    let courses = |j: String| {
        let c = r.client.clone();
        let url = format!("{}/public/livreur/courses", r.url);
        async move {
            let rep = c.get(url).bearer_auth(j).send().await.unwrap();
            (rep.status().as_u16(), rep.json::<Value>().await.unwrap_or(Value::Null))
        }
    };
    assert_eq!(courses("faux".into()).await.0, 401);
    let (code, v) = courses(jeton.clone()).await;
    assert_eq!(code, 200, "{v}");
    assert_eq!(v["courses"][0]["code_livreur"], "LIVREURCODE1");
    assert_eq!(v["courses"][0]["quartier"], "Hamdallaye");

    // Poste ancien (sans « livreurs ») : rien ne change.
    r.synchroniser(CLE, json!({ "menu": menu() })).await;
    assert_eq!(courses(jeton.clone()).await.0, 200);
    // PIN changé sur le poste : la session se ferme.
    let autre = json!([{ "employe_id": "e1", "nom": "Ibrahim", "telephone": "76554433", "pin_hash": youma_core::auth::hacher("7777").unwrap(), "courses": [] }]);
    r.synchroniser(CLE, json!({ "menu": menu(), "livreurs": autre })).await;
    assert_eq!(courses(jeton).await.0, 401);
    // 5 essais par numéro en 15 minutes, réussis compris (2 déjà faits sur ce numéro) : le 6e est refusé, même juste.
    for pin in ["1111", "2222", "3333"] {
        assert_eq!(r.post("/public/livreur/connexion", connexion("76554433", pin)).await.0, 401);
    }
    assert_eq!(r.post("/public/livreur/connexion", connexion("76554433", "7777")).await.0, 429);
}

/// Fiche 0048 : les photos du menu sont servies à part (cache d'un an) et les réponses compressées ; un menu déjà
/// reçu avec ses photos intégrées est converti au redémarrage du relais.
#[tokio::test]
async fn photos_du_menu_servies_a_part_et_compression() {
    const JPEG: &str = "data:image/jpeg;base64,/9j/4AAQSkZJRgABAQAAAQABAAD/2wBDAAgGBgcGBQgHBwcJCQgKDBQNDAsLDBkSEw8U";
    let avec_photo = |photo: &str| {
        let mut m = menu();
        m["produits"] = json!([{ "id": "p1", "categorie_id": "c", "nom": "Brochettes", "description": "", "photo": photo, "prix": 1500, "groupes_options": [] },
                               { "id": "p2", "categorie_id": "c", "nom": "Coca", "description": "", "photo": "", "prix": 500, "groupes_options": [] }]);
        m
    };
    let r = relais().await;
    r.synchroniser(CLE, json!({ "menu": avec_photo(JPEG) })).await;
    let (_, m) = r.get("/public/menu").await;
    let adresse = m["produits"][0]["photo"].as_str().unwrap().to_string();
    assert!(adresse.starts_with("/api/public/photos/"), "{adresse}");
    assert_eq!(m["produits"][1]["photo"], "");
    assert!(!m.to_string().contains("base64"), "plus de photo dans le menu");
    let rep = r.client.get(format!("{}{}", r.url.trim_end_matches("/api"), adresse)).send().await.unwrap();
    assert_eq!(rep.status().as_u16(), 200);
    assert_eq!(rep.headers()["content-type"], "image/jpeg");
    assert!(rep.headers()["cache-control"].to_str().unwrap().contains("immutable"));
    let octets = rep.bytes().await.unwrap();
    assert_eq!(&octets[..3], &[0xFF, 0xD8, 0xFF], "image JPEG d'origine");
    // Réponses compressées pour qui le demande (navigateurs, applications).
    let rep = r.client.get(format!("{}/public/menu", r.url)).header("accept-encoding", "gzip").send().await.unwrap();
    assert_eq!(rep.headers()["content-encoding"], "gzip");
    // Photo retirée du plat : effacée du relais.
    r.synchroniser(CLE, json!({ "menu": avec_photo("") })).await;
    let rep = r.client.get(format!("{}{}", r.url.trim_end_matches("/api"), adresse)).send().await.unwrap();
    assert_eq!(rep.status().as_u16(), 404);

    // Relais mis à jour avec un ancien menu (photos intégrées) : converti à l'ouverture.
    let dossier = tempfile::tempdir().unwrap();
    let config = Config { dossier_donnees: dossier.path().to_path_buf(), port: 0, cle: CLE.into(), dossier_ui: None, derriere_proxy: false, sms: FournisseurSms::Simulation, whatsapp: None };
    drop(Etat::ouvrir(&config).unwrap());
    rusqlite::Connection::open(dossier.path().join("youma-relais.db"))
        .unwrap()
        .execute("INSERT INTO etat(cle, valeur) VALUES ('menu', ?1)", [avec_photo(JPEG).to_string()])
        .unwrap();
    let etat = Etat::ouvrir(&config).unwrap();
    let ecoute = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", ecoute.local_addr().unwrap());
    tokio::spawn(youma_relais::servir(etat, None, ecoute));
    let c = reqwest::Client::new();
    c.post(format!("{url}/api/relais/synchroniser")).bearer_auth(CLE).json(&json!({})).send().await.unwrap();
    let m: Value = c.get(format!("{url}/api/public/menu")).send().await.unwrap().json().await.unwrap();
    let photo = m["produits"][0]["photo"].as_str().unwrap();
    assert_eq!(photo, adresse, "même photo, même adresse");
    assert_eq!(c.get(format!("{url}{photo}")).send().await.unwrap().status().as_u16(), 200);
}

/// Cloud multi-restaurants : résumés, SMS de clôture une seule fois, espace propriétaire, sauvegardes chiffrées.
#[tokio::test]
async fn cloud_resumes_proprietaire_et_sauvegardes() {
    let dossier = tempfile::tempdir().unwrap();
    let cle_a = youma_relais::inscrire_restaurant(dossier.path(), "Maquis A").unwrap();
    let cle_b = youma_relais::inscrire_restaurant(dossier.path(), "Maquis B").unwrap();
    let config = Config { dossier_donnees: dossier.path().to_path_buf(), port: 0, cle: CLE.into(), dossier_ui: None, derriere_proxy: false, sms: FournisseurSms::Simulation, whatsapp: None };
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

#[tokio::test]
async fn menu_renvoye_seulement_s_il_change() {
    let r = relais().await;
    let config = json!({ "verification_numero": "rappel" });
    let (_, s) = r.synchroniser(CLE, json!({ "menu": menu(), "menu_empreinte": "e1", "config": config })).await;
    assert_eq!(s["menu_empreinte"], "e1");
    // Menu déjà détenu : le poste n'envoie que l'empreinte, le relais garde son menu.
    let (code, s) = r.synchroniser(CLE, json!({ "menu": null, "menu_empreinte": "e1", "config": config })).await;
    assert_eq!(code, 200);
    assert_eq!(s["menu_empreinte"], "e1");
    assert_eq!(r.get("/public/menu").await.1["restaurant"], "Maquis Le Baobab");
    let mut m = menu();
    m["restaurant"] = json!("Maquis Le Fromager");
    let (_, s) = r.synchroniser(CLE, json!({ "menu": m, "menu_empreinte": "e2", "config": config })).await;
    assert_eq!(s["menu_empreinte"], "e2");
    assert_eq!(r.get("/public/menu").await.1["restaurant"], "Maquis Le Fromager");
}

/// RG-CAN-07 : page rechargée ou double appui : même numéro et même panier → la commande déjà transmise.
#[tokio::test]
async fn commande_renvoyee_une_seule_fois() {
    let r = relais().await;
    let (code, s) = r.synchroniser(CLE, json!({ "menu": menu(), "config": { "verification_numero": "rappel" } })).await;
    assert_eq!(code, 200, "{s}");
    let (_, a) = r.post("/public/commandes", commande("")).await;
    assert_eq!(a["statut"], "en_attente", "{a}");
    let (_, b) = r.post("/public/commandes", commande("")).await;
    assert_eq!(b["code_suivi"], a["code_suivi"]);
    assert!(b["message"].as_str().unwrap().contains("déjà été reçue"));
    let mut autre = commande("");
    autre["lignes"][0]["quantite"] = json!(2);
    let (_, c) = r.post("/public/commandes", autre).await;
    assert_ne!(c["code_suivi"], a["code_suivi"], "autre panier, autre commande");
    let (_, sync) = r.synchroniser(CLE, json!({ "config": { "verification_numero": "rappel" } })).await;
    assert_eq!(sync["commandes"].as_array().unwrap().len(), 2, "{sync}");
}

/// RG-CAN-08 : 15 codes inconnus en 15 minutes depuis une adresse, puis refus (même pour un vrai code).
#[tokio::test]
async fn codes_inconnus_limites_a_15_essais() {
    let r = relais().await;
    for i in 0..15 {
        assert_eq!(r.get(&format!("/public/suivi/INCONNU{i}")).await.0, 404);
    }
    let (statut, v) = r.get("/public/suivi/INCONNU99").await;
    assert_eq!(statut, 429, "{v}");
    assert_eq!(v["code"], "TROP_D_ESSAIS");
    let (statut, _) = r.post("/public/position/LIVREURX", json!({ "lat": 12_640_000, "lon": -8_000_000 })).await;
    assert_eq!(statut, 429);
}

/// RG-AVI-01 : avis donné sur le relais pour une commande terminée, une fois, transmis au poste à la synchronisation.
#[tokio::test]
async fn avis_du_client_transmis_au_poste() {
    let r = relais().await;
    let suivi = |possible: bool| json!({ "numero": 12, "restaurant": "Chez Mariam", "etape": if possible { "livree" } else { "en_route" },
        "motif": null, "type": "livraison", "total": 3000, "reste": 0, "paiement_mode": null, "lignes": [], "livreur": null,
        "destination": null, "mis_a_jour": 0, "avis": null, "avis_possible": possible });
    let (code, s) = r.synchroniser(CLE, json!({ "menu": menu(), "suivis": [{ "code_suivi": "AVIS0001", "code_livreur": null, "suivi": suivi(false) }] })).await;
    assert_eq!(code, 200, "{s}");
    assert_eq!(r.post("/public/avis/AVIS0001", json!({ "note": 4 })).await.0, 422, "commande pas encore livrée");
    r.synchroniser(CLE, json!({ "suivis": [{ "code_suivi": "AVIS0001", "code_livreur": null, "suivi": suivi(true) }] })).await;
    assert_eq!(r.post("/public/avis/AVIS0001", json!({ "note": 9 })).await.0, 422);
    assert_eq!(r.post("/public/avis/INCONNU1", json!({ "note": 4 })).await.0, 404);
    let (statut, v) = r.post("/public/avis/AVIS0001", json!({ "note": 4, "commentaire": "Rapide, merci" })).await;
    assert_eq!(statut, 200, "{v}");
    assert_eq!(r.post("/public/avis/AVIS0001", json!({ "note": 1 })).await.0, 422, "un seul avis");
    let (_, s) = r.get("/public/suivi/AVIS0001").await;
    assert_eq!((s["avis"].as_i64(), s["avis_possible"].as_bool()), (Some(4), Some(false)));
    let (_, sync) = r.synchroniser(CLE, json!({ "suivis": [{ "code_suivi": "AVIS0001", "code_livreur": null, "suivi": suivi(true) }] })).await;
    assert_eq!(sync["avis"], json!([{ "code_suivi": "AVIS0001", "note": 4, "commentaire": "Rapide, merci" }]));
    let (_, sync) = r.synchroniser(CLE, json!({})).await;
    assert_eq!(sync["avis"], json!([]), "transmis une seule fois");
}
