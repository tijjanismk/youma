//! Tests de l'API HTTP/WebSocket sur un vrai serveur (port aléatoire, base de démonstration).

use std::net::SocketAddr;

use futures_util::StreamExt;
use serde_json::{json, Value};
use youma_server::{Config, Etat};

struct Serveur {
    url: String,
    adresse: SocketAddr,
    _dossier: tempfile::TempDir,
    client: reqwest::Client,
}

async fn serveur() -> Serveur {
    let dossier = tempfile::tempdir().unwrap();
    let ui = dossier.path().join("ui");
    std::fs::create_dir_all(&ui).unwrap();
    std::fs::write(ui.join("index.html"), "<html>Youma</html>").unwrap();
    let config = Config {
        dossier_donnees: dossier.path().to_path_buf(),
        port: 0,
        reseau: false,
        dossier_ui: Some(ui),
        demo: true,
        reseau_sans_licence: false,
    };
    let etat = Etat::ouvrir(config).unwrap();
    let ecoute = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let adresse = ecoute.local_addr().unwrap();
    tokio::spawn(youma_server::servir(etat, ecoute));
    Serveur { url: format!("http://{adresse}/api"), adresse, _dossier: dossier, client: reqwest::Client::new() }
}

impl Serveur {
    async fn connexion(&self, nom_contient: &str, pin: &str) -> String {
        let users: Vec<Value> = self.client.get(format!("{}/connexion/utilisateurs", self.url)).send().await.unwrap().json().await.unwrap();
        let u = users.iter().find(|u| u["nom"].as_str().unwrap().contains(nom_contient)).unwrap();
        let r: Value = self
            .client
            .post(format!("{}/connexion", self.url))
            .json(&json!({ "utilisateur_id": u["id"], "pin": pin }))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        r["jeton"].as_str().unwrap().to_string()
    }

    /// RG-AUT-06 : confirme la session par mot de passe (administration).
    async fn elever(&self, jeton: &str, mot_de_passe: &str) {
        let (code, r) = self.post(jeton, "/session/elever", json!({ "mot_de_passe": mot_de_passe })).await;
        assert_eq!(code, 200, "{r}");
        assert_eq!(r["eleve"], true);
    }

    async fn post(&self, jeton: &str, chemin: &str, corps: Value) -> (u16, Value) {
        self.post_pin(jeton, chemin, corps, None).await
    }

    async fn post_pin(&self, jeton: &str, chemin: &str, corps: Value, pin: Option<&str>) -> (u16, Value) {
        let mut req = self.client.post(format!("{}{chemin}", self.url)).bearer_auth(jeton).json(&corps);
        if let Some(p) = pin {
            req = req.header("X-Autorisation-Pin", p);
        }
        let r = req.send().await.unwrap();
        let s = r.status().as_u16();
        (s, r.json().await.unwrap_or(Value::Null))
    }

    async fn get(&self, jeton: &str, chemin: &str) -> (u16, Value) {
        let r = self.client.get(format!("{}{chemin}", self.url)).bearer_auth(jeton).send().await.unwrap();
        let s = r.status().as_u16();
        (s, r.json().await.unwrap_or(Value::Null))
    }
}

async fn id_produit(s: &Serveur, jeton: &str, nom: &str) -> String {
    let (_, c) = s.get(jeton, "/catalogue").await;
    c["produits"].as_array().unwrap().iter().find(|p| p["nom"] == nom).unwrap()["id"].as_str().unwrap().to_string()
}

async fn id_table(s: &Serveur, jeton: &str, nom: &str) -> String {
    let (_, salle) = s.get(jeton, "/salle").await;
    salle["tables"].as_array().unwrap().iter().find(|t| t["nom"] == nom).unwrap()["id"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn parcours_complet_service() {
    let s = serveur().await;
    let (_, etat) = s.get("", "/etat").await;
    assert_eq!(etat["installe"], true);
    assert_eq!(etat["demo"], true);

    // Sans jeton : 401 au format d'erreur de l'API.
    let (code, e) = s.get("", "/commandes").await;
    assert_eq!(code, 401);
    assert_eq!(e["code"], "NON_AUTHENTIFIE");

    let caissier = s.connexion("Kadi", "3333").await;
    let serveuse = s.connexion("Awa", "4444").await;
    // Commande avant journée : règle RG-JOU-01.
    let table = id_table(&s, &serveuse, "4").await;
    let (code, e) = s.post(&serveuse, "/commandes", json!({ "type": "sur_place", "table_id": table })).await;
    assert_eq!(code, 422);
    assert_eq!(e["regle"], "RG-JOU-01");

    let (code, _) = s.post(&caissier, "/journee/ouvrir", json!({})).await;
    assert_eq!(code, 200);
    let (code, _) = s.post(&caissier, "/caisse/ouvrir", json!({ "fond_compte": 10000, "motif_ecart": "Fond" })).await;
    assert_eq!(code, 200);

    let (_, cid) = s.post(&serveuse, "/commandes", json!({ "type": "sur_place", "table_id": table })).await;
    let cid = cid.as_str().unwrap().to_string();
    let poulet = id_produit(&s, &serveuse, "Poulet braisé").await;
    let biere = id_produit(&s, &serveuse, "Bière blonde").await;
    let (code, _) = s
        .post(&serveuse, &format!("/commandes/{cid}/lignes"), json!([{ "produit_id": poulet, "quantite": 2 }, { "produit_id": biere, "quantite": 3, "commentaire": "bien fraîches" }]))
        .await;
    assert_eq!(code, 200);
    let (code, envois) = s.post(&serveuse, &format!("/commandes/{cid}/envoyer"), json!({})).await;
    assert_eq!(code, 200);
    assert_eq!(envois.as_array().unwrap().len(), 2, "grill + bar");

    // Écran cuisine : la serveuse (cuisine.voir) voit les envois.
    let (_, cuisine) = s.get(&serveuse, "/cuisine").await;
    assert_eq!(cuisine.as_array().unwrap().len(), 2);
    let envoi = cuisine[0]["id"].as_str().unwrap();
    let (code, _) = s.post(&serveuse, &format!("/envois/{envoi}/statut"), json!({ "statut": "pret" })).await;
    assert_eq!(code, 200);

    // Plan de salle : table occupée.
    let (_, salle) = s.get(&serveuse, "/salle").await;
    let t4 = salle["tables"].as_array().unwrap().iter().find(|t| t["nom"] == "4").unwrap().clone();
    assert_eq!(t4["statut"], "occupee");

    // Annulation après envoi : la serveuse doit faire intervenir le gérant.
    let (_, d) = s.get(&serveuse, &format!("/commandes/{cid}")).await;
    let ligne_biere = d["lignes"].as_array().unwrap().iter().find(|l| l["libelle"] == "Bière blonde").unwrap()["id"].as_str().unwrap().to_string();
    let (code, e) = s.post(&serveuse, &format!("/lignes/{ligne_biere}/annuler"), json!({ "quantite": 1, "motif": "Renversée" })).await;
    assert_eq!(code, 403);
    assert_eq!(e["code"], "AUTORISATION_REQUISE");
    assert_eq!(e["permission"], "commande.annuler_envoye");
    let (code, e) = s.post_pin(&serveuse, &format!("/lignes/{ligne_biere}/annuler"), json!({ "quantite": 1, "motif": "Renversée" }), Some("9999")).await;
    assert_eq!(code, 401);
    assert_eq!(e["code"], "PIN_INCORRECT");
    let (code, _) = s.post_pin(&serveuse, &format!("/lignes/{ligne_biere}/annuler"), json!({ "quantite": 1, "motif": "Renversée", "perte": true }), Some("2222")).await;
    assert_eq!(code, 200);

    // Division en 3 parts égales puis paiement mixte.
    let (_, d) = s.get(&caissier, &format!("/commandes/{cid}")).await;
    assert_eq!(d["totaux"]["total"], 9_000);
    let (_, parts) = s.get(&caissier, &format!("/commandes/{cid}/diviser?parts=3")).await;
    assert_eq!(parts, json!([3000, 3000, 3000]));
    let (_, comptes) = s.get(&caissier, "/comptes").await;
    let om = comptes.as_array().unwrap().iter().find(|c| c["nom"] == "Orange Money").unwrap()["id"].as_str().unwrap().to_string();
    let (code, r) = s
        .post(
            &caissier,
            "/caisse/encaisser",
            json!({ "commande_id": cid, "especes_recues": 5000, "parts": [
                { "moyen": "especes", "montant": 3000 },
                { "moyen": "mobile_money", "montant": 6000, "compte_id": om, "reference": "MP260314.0001" }
            ]}),
        )
        .await;
    assert_eq!(code, 200, "{r}");
    assert_eq!(r["rendu"], 2000);
    assert_eq!(r["commande_payee"], true);

    // Ticket client lisible.
    let (_, ticket) = s.get(&caissier, &format!("/commandes/{cid}/ticket")).await;
    let ticket = ticket.as_str().unwrap();
    assert!(ticket.contains("Maquis Le Baobab"));
    assert!(ticket.contains("9 000"));

    // Tableau de bord : réservé aux rôles avec rapport.voir.
    let (code, _) = s.get(&serveuse, "/tableau-de-bord").await;
    assert_eq!(code, 403);
    let gerant = s.connexion("Adama", "2222").await;
    let (_, tdb) = s.get(&gerant, "/tableau-de-bord").await;
    let ca = tdb["indicateurs"].as_array().unwrap().iter().find(|i| i["cle"] == "ca").unwrap();
    assert_eq!(ca["valeur"], 9_000);
    assert!(ca["formule"].as_str().unwrap().starts_with("CA ="));
    assert_eq!(tdb["annulations"], json!([1, 1000]));
    assert_eq!(tdb["mobile_money_a_verifier"], json!([1, 6000]));

    // Export CSV.
    let r = s.client.get(format!("{}/rapports/periode?format=csv", s.url)).bearer_auth(&gerant).send().await.unwrap();
    assert!(r.headers()["content-type"].to_str().unwrap().starts_with("text/csv"));
    let csv = r.text().await.unwrap();
    assert!(csv.contains("Chiffre d'affaires;9000;"));
    assert!(csv.contains("Produits les plus vendus"));

    // Clôture de caisse avec rapport Z.
    let (_, c) = s.get(&caissier, "/caisse").await;
    let sid = c["session"]["id"].as_str().unwrap().to_string();
    let (code, z) = s.post(&caissier, &format!("/caisse/{sid}/cloturer"), json!({ "compte_final": 13000 })).await;
    assert_eq!(code, 200, "{z}");
    assert!(z["z"].as_str().unwrap().contains("RAPPORT Z"));
    let (code, _) = s.post(&caissier, "/journee/cloturer", json!({})).await;
    assert_eq!(code, 200);
    // Journal d'audit consultable par le propriétaire.
    let proprio = s.connexion("Mariam", "1234").await;
    let (_, audit) = s.get(&proprio, "/audit").await;
    assert!(audit.as_array().unwrap().iter().any(|l| l["action"] == "commande.annuler_article" && l["autorise_par"] == "Adama (gérant)"));
    // Sauvegarde de clôture créée.
    // Administration : mot de passe exigé en plus du PIN (RG-AUT-06).
    let (code, e) = s.get(&proprio, "/sauvegardes").await;
    assert_eq!(code, 403);
    assert_eq!(e["code"], "MOT_DE_PASSE_REQUIS");
    s.elever(&proprio, "baobab123").await;
    let (_, sv) = s.get(&proprio, "/sauvegardes").await;
    assert!(sv.as_array().unwrap().iter().any(|x| x["motif"] == "cloture"));
}

#[tokio::test]
async fn websocket_diffuse_les_evenements() {
    let s = serveur().await;
    let caissier = s.connexion("Kadi", "3333").await;
    let (mut ws, _) = tokio_tungstenite::connect_async(format!("ws://{}/api/ws?jeton={caissier}", s.adresse)).await.unwrap();
    let premier = ws.next().await.unwrap().unwrap().into_text().unwrap();
    assert!(premier.contains("connecte"));
    s.post(&caissier, "/journee/ouvrir", json!({})).await;
    let ev = tokio::time::timeout(std::time::Duration::from_secs(5), ws.next()).await.unwrap().unwrap().unwrap().into_text().unwrap();
    assert!(ev.contains("\"journee\""), "{ev}");
    // Jeton invalide : refus.
    assert!(tokio_tungstenite::connect_async(format!("ws://{}/api/ws?jeton=faux", s.adresse)).await.is_err());
}

#[tokio::test]
async fn interface_servie_et_route_inconnue() {
    let s = serveur().await;
    let r = s.client.get(format!("http://{}/caisse/quelque-part", s.adresse)).send().await.unwrap();
    assert_eq!(r.status().as_u16(), 200);
    assert!(r.text().await.unwrap().contains("Youma"), "repli SPA sur index.html");
}

#[tokio::test]
async fn appairage_d_un_telephone() {
    let s = serveur().await;
    let proprio = s.connexion("Mariam", "1234").await;
    let serveuse = s.connexion("Awa", "4444").await;
    let (code, _) = s.post(&serveuse, "/appareils/code", json!({})).await;
    assert_eq!(code, 403);
    s.elever(&proprio, "baobab123").await;
    let (_, code_app) = s.post(&proprio, "/appareils/code", json!({})).await;
    let c = code_app["code"].as_str().unwrap();
    let (code, e) = s.post("", "/appareils/appairer", json!({ "code": "000000", "nom": "Tél. Awa" })).await;
    if c != "000000" {
        assert_eq!(code, 422, "{e}");
    }
    let (code, r) = s.post("", "/appareils/appairer", json!({ "code": c, "nom": "Tél. Awa" })).await;
    assert_eq!(code, 200);
    assert!(r["jeton"].as_str().unwrap().len() > 20);
    // Code à usage unique.
    let (code, _) = s.post("", "/appareils/appairer", json!({ "code": c, "nom": "Autre" })).await;
    assert_eq!(code, 422);
    let (_, liste) = s.get(&proprio, "/appareils").await;
    let id = liste[0]["id"].as_str().unwrap();
    let (code, _) = s.post(&proprio, &format!("/appareils/{id}/revoquer"), json!({})).await;
    assert_eq!(code, 200);
    assert!(!youma_server::est_local("192.168.1.20".parse().unwrap()));
    assert!(youma_server::est_local("127.0.0.1".parse().unwrap()));
}

#[tokio::test]
async fn employes_et_paie_par_api() {
    let s = serveur().await;
    let gerant = s.connexion("Adama", "2222").await;
    // Employé minimal, sans contrat ni INPS.
    let (code, id) = s.post(&gerant, "/employes", json!({ "nom": "Seydou", "type_remuneration": "journalier", "montant_base": 2500 })).await;
    assert_eq!(code, 200, "{id}");
    let id = id.as_str().unwrap().to_string();
    let (code, _) = s
        .post(&gerant, "/presences", json!([
            { "employe_id": id, "date": "2026-03-02", "statut": "present" },
            { "employe_id": id, "date": "2026-03-03", "statut": "present" }
        ]))
        .await;
    assert_eq!(code, 200);
    let (_, ap) = s.get(&gerant, &format!("/paie/apercu?employe={id}&debut=2026-03-01&fin=2026-03-07")).await;
    assert_eq!(ap["net_a_payer"], 5000);
    assert_eq!(ap["type_contrat"], "aucun");
    let (_, refs) = s.get(&gerant, "/employes/references").await;
    assert!(refs["contrats"].as_array().unwrap().contains(&json!("aucun")));
    // La serveuse ne voit pas les salaires.
    let serveuse = s.connexion("Awa", "4444").await;
    let (code, _) = s.get(&serveuse, "/employes").await;
    assert_eq!(code, 403);
}

/// Mode réseau : un appareil distant non appairé ne voit rien ; appairé, il travaille normalement.
#[tokio::test]
async fn appareil_distant_doit_etre_appaire() {
    // Adresse IP locale non bouclée (interface réseau du poste) : sinon, test sans objet sur cette machine.
    let Some(ip) = std::net::UdpSocket::bind("0.0.0.0:0")
        .and_then(|s| s.connect("10.255.255.1:80").map(|_| s))
        .ok()
        .and_then(|s| s.local_addr().ok())
        .map(|a| a.ip())
        .filter(|ip| !ip.is_loopback() && !ip.is_unspecified())
    else {
        eprintln!("pas d'interface réseau : test ignoré");
        return;
    };
    let dossier = tempfile::tempdir().unwrap();
    let config = Config { dossier_donnees: dossier.path().to_path_buf(), port: 0, reseau: true, dossier_ui: None, demo: true, reseau_sans_licence: false };
    let etat = Etat::ouvrir(config).unwrap();
    let ecoute = tokio::net::TcpListener::bind("0.0.0.0:0").await.unwrap();
    let port = ecoute.local_addr().unwrap().port();
    tokio::spawn(youma_server::servir(etat, ecoute));
    let local = format!("http://127.0.0.1:{port}/api");
    let distant = format!("http://{ip}:{port}/api");
    let client = reqwest::Client::new();

    // Téléphone non appairé : invitation à appairer, aucune donnée.
    let e: Value = client.get(format!("{distant}/etat")).send().await.unwrap().json().await.unwrap();
    assert_eq!(e["appairage_requis"], true);
    assert!(e.get("parametres").is_none());
    let r = client.get(format!("{distant}/connexion/utilisateurs")).send().await.unwrap();
    assert_eq!(r.status().as_u16(), 403);

    // Le propriétaire génère un code sur le poste central.
    let users: Vec<Value> = client.get(format!("{local}/connexion/utilisateurs")).send().await.unwrap().json().await.unwrap();
    let proprio = users.iter().find(|u| u["nom"].as_str().unwrap().contains("Mariam")).unwrap();
    let s: Value = client.post(format!("{local}/connexion")).json(&json!({ "utilisateur_id": proprio["id"], "pin": "1234" })).send().await.unwrap().json().await.unwrap();
    let j = s["jeton"].as_str().unwrap();
    let el = client.post(format!("{local}/session/elever")).bearer_auth(j).json(&json!({ "mot_de_passe": "baobab123" })).send().await.unwrap();
    assert_eq!(el.status().as_u16(), 200);
    let code: Value = client.post(format!("{local}/appareils/code")).bearer_auth(s["jeton"].as_str().unwrap()).json(&json!({})).send().await.unwrap().json().await.unwrap();

    // Le téléphone s'appaire puis accède à l'application.
    let a: Value = client.post(format!("{distant}/appareils/appairer")).json(&json!({ "code": code["code"], "nom": "Tél. Awa" })).send().await.unwrap().json().await.unwrap();
    let jeton_appareil = a["jeton"].as_str().unwrap().to_string();
    let e: Value = client.get(format!("{distant}/etat")).header("X-Appareil", &jeton_appareil).send().await.unwrap().json().await.unwrap();
    assert_eq!(e["restaurant"], "Maquis Le Baobab");
    let r = client.get(format!("{distant}/connexion/utilisateurs")).header("X-Appareil", &jeton_appareil).send().await.unwrap();
    assert_eq!(r.status().as_u16(), 200);
    // Jeton inventé : refusé.
    let r = client.get(format!("{distant}/connexion/utilisateurs")).header("X-Appareil", "faux").send().await.unwrap();
    assert_eq!(r.status().as_u16(), 403);
}

/// Bon de sortie : le ticket payé porte un numéro et un code ; le contrôle à la sortie le vérifie.
#[tokio::test]
async fn bon_de_sortie_par_api() {
    let s = serveur().await;
    let caissier = s.connexion("Kadi", "3333").await;
    s.post(&caissier, "/journee/ouvrir", json!({})).await;
    s.post(&caissier, "/caisse/ouvrir", json!({ "fond_compte": 0, "motif_ecart": "x" })).await;
    let (_, cid) = s.post(&caissier, "/commandes", json!({ "type": "emporter" })).await;
    let cid = cid.as_str().unwrap().to_string();
    let coca = id_produit(&s, &caissier, "Coca-Cola").await;
    s.post(&caissier, &format!("/commandes/{cid}/lignes"), json!([{ "produit_id": coca, "quantite": 2 }])).await;
    let (code, r) = s.post(&caissier, "/caisse/encaisser", json!({ "commande_id": cid, "parts": [{ "moyen": "especes", "montant": 1500 }], "especes_recues": 2000 })).await;
    assert_eq!(code, 200, "{r}");
    let (_, ticket) = s.get(&caissier, &format!("/commandes/{cid}/ticket")).await;
    let ticket = ticket.as_str().unwrap().to_string();
    assert!(ticket.contains("TICKET DE CAISSE"), "{ticket}");
    let ligne = ticket.lines().find(|l| l.contains("Code de contrôle")).unwrap();
    let code_ctrl = ligne.rsplit(' ').next().unwrap().trim().to_string();
    let (_, d) = s.get(&caissier, &format!("/commandes/{cid}")).await;
    let numero = d["numero"].as_i64().unwrap();
    let serveuse = s.connexion("Awa", "4444").await;
    let (code, r) = s.post(&serveuse, "/sortie/controle", json!({ "numero": numero, "code": code_ctrl })).await;
    assert_eq!(code, 200, "{r}");
    assert_eq!(r["statut"], "paye");
    let (_, r) = s.post(&serveuse, "/sortie/controle", json!({ "numero": numero, "code": code_ctrl })).await;
    assert_eq!(r["deja_presente"].as_array().unwrap().len(), 1);
    let (code, r) = s.post(&serveuse, "/sortie/controle", json!({ "numero": numero, "code": "ZZZZ" })).await;
    if code_ctrl != "ZZZZ" {
        assert_eq!(code, 422);
        assert_eq!(r["regle"], "RG-SOR-02");
    }
}

/// Commande par QR sur la table et en ligne : routes publiques, file de validation, zones à risque, suivi.
#[tokio::test]
async fn commandes_qr_en_ligne_et_suivi_par_api() {
    let s = serveur().await;
    let proprio = s.connexion("Mariam", "1234").await;
    let caissier = s.connexion("Kadi", "3333").await;
    let serveuse = s.connexion("Awa", "4444").await;
    // Canaux désactivés : aucune route publique.
    let r = s.client.get(format!("{}/public/menu", s.url)).send().await.unwrap();
    assert_eq!(r.status().as_u16(), 403);

    s.elever(&proprio, "baobab123").await;
    let (_, mut p) = s.get(&proprio, "/parametres").await;
    p["canaux"]["qr_table"] = json!(true);
    p["canaux"]["en_ligne"] = json!(true);
    let r = s.client.put(format!("{}/parametres", s.url)).bearer_auth(&proprio).json(&p).send().await.unwrap();
    assert_eq!(r.status().as_u16(), 200);
    let (code, n) = s.post(&proprio, "/tables/codes-qr", json!({})).await;
    assert_eq!(code, 200, "{n}");
    let (_, codes) = s.get(&proprio, "/tables/codes-qr").await;
    let qr = codes.as_array().unwrap().iter().find(|t| t["nom"].as_str().unwrap().ends_with("— 2")).unwrap()["code"].as_str().unwrap().to_string();
    let (code, _) = s.get(&serveuse, "/tables/codes-qr").await;
    assert_eq!(code, 403);

    s.post(&caissier, "/journee/ouvrir", json!({})).await;
    let menu: Value = s.client.get(format!("{}/public/menu?table={qr}", s.url)).send().await.unwrap().json().await.unwrap();
    assert_eq!(menu["table"], "2");
    assert_eq!(menu["ouvert"], true);
    let produit = menu["produits"][0]["id"].as_str().unwrap().to_string();

    // Commande QR : en attente, puis acceptée par la caissière.
    let r: Value = s
        .client
        .post(format!("{}/public/commandes", s.url))
        .json(&json!({ "canal": "qr_table", "code_table": qr, "lignes": [{ "produit_id": produit, "quantite": 1 }] }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(r["statut"], "en_attente", "{r}");
    let (_, file) = s.get(&caissier, "/entrantes").await;
    assert_eq!(file.as_array().unwrap().len(), 1);
    let id = file[0]["commande"]["id"].as_str().unwrap().to_string();
    let (code, _) = s.get(&serveuse, "/entrantes").await;
    assert_eq!(code, 403);
    let (code, e) = s.post(&caissier, &format!("/entrantes/{id}/valider"), json!({ "accepter": true })).await;
    assert_eq!(code, 200, "{e}");
    let suivi: Value = s.client.get(format!("{}/public/suivi/{}", s.url, r["code_suivi"].as_str().unwrap())).send().await.unwrap().json().await.unwrap();
    assert_eq!(suivi["etape"], "en_preparation");

    // Zone à risque la nuit : ici toute la journée, pour le test.
    let (code, e) = s
        .post(&caissier, "/zones-risque", json!({ "nom": "Kalaban", "quartier": "Kalaban Coura", "debut_min": 0, "fin_min": 1440, "action": "bloquer" }))
        .await;
    assert_eq!(code, 403, "{e}");
    let gerant = s.connexion("Adama", "2222").await;
    let (code, e) = s
        .post(&gerant, "/zones-risque", json!({ "nom": "Kalaban", "quartier": "Kalaban Coura", "debut_min": 0, "fin_min": 1440, "action": "bloquer" }))
        .await;
    assert_eq!(code, 200, "{e}");
    let commande = |quartier: &str| {
        json!({ "canal": "en_ligne", "type": "livraison", "client_nom": "Awa", "telephone": "76 00 00 01",
                "telephone_verifie": true, "origine_id": "pirate",
                "livraison": { "quartier": quartier, "repere": "École", "telephone": "76000001" },
                "paiement_mode": "a_la_livraison", "lignes": [{ "produit_id": produit, "quantite": 2 }] })
    };
    let r: Value = s.client.post(format!("{}/public/commandes", s.url)).json(&commande("Kalaban Coura")).send().await.unwrap().json().await.unwrap();
    assert_eq!(r["statut"], "refusee");
    let r: Value = s.client.post(format!("{}/public/commandes", s.url)).json(&commande("Hamdallaye")).send().await.unwrap().json().await.unwrap();
    assert_eq!(r["statut"], "en_attente");
    // Liste noire.
    let (code, _) = s.post(&gerant, "/numeros-bloques", json!({ "telephone": "76000001", "motif": "Faux client" })).await;
    assert_eq!(code, 200);
    let (_, l) = s.get(&gerant, "/numeros-bloques").await;
    assert_eq!(l[0]["telephone"], "76000001");
    let r: Value = s.client.post(format!("{}/public/commandes", s.url)).json(&commande("Hamdallaye")).send().await.unwrap().json().await.unwrap();
    assert_eq!(r["statut"], "refusee");

    // Liens de suivi : le code du livreur est distinct de celui du client.
    let (_, file) = s.get(&caissier, "/entrantes").await;
    let id = file[0]["commande"]["id"].as_str().unwrap().to_string();
    let (code, e) = s.post(&caissier, &format!("/entrantes/{id}/valider"), json!({ "accepter": false, "motif": "" })).await;
    assert_eq!((code, e["regle"].as_str()), (422, Some("RG-CAN-02")));
    s.post(&caissier, &format!("/entrantes/{id}/valider"), json!({ "accepter": true })).await;
    let (_, liens) = s.post(&gerant, &format!("/commandes/{id}/liens"), json!({})).await;
    let livreur = liens["code_livreur"].as_str().unwrap();
    let r = s.client.post(format!("{}/public/position/{livreur}", s.url)).json(&json!({ "lat": 12_640_000, "lon": -8_000_000 })).send().await.unwrap();
    assert_eq!(r.status().as_u16(), 422, "pas encore en route");
}

/// Relais Internet facultatif : vrai poste central + vrai relais. Commande en ligne passée sur le relais,
/// reprise et acceptée par la caisse, suivie par le client, position du livreur remontée au poste.
#[tokio::test]
async fn relais_internet_de_bout_en_bout() {
    const CLE: &str = "cle-du-relais-de-bout-en-bout";
    let dossier_relais = tempfile::tempdir().unwrap();
    let rc = youma_relais::Config { dossier_donnees: dossier_relais.path().to_path_buf(), port: 0, cle: CLE.into(), dossier_ui: None, derriere_proxy: false, sms: youma_relais::FournisseurSms::Simulation };
    let ecoute = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let relais = format!("http://{}", ecoute.local_addr().unwrap());
    tokio::spawn(youma_relais::servir(youma_relais::Etat::ouvrir(&rc).unwrap(), None, ecoute));

    let dossier = tempfile::tempdir().unwrap();
    let config = Config { dossier_donnees: dossier.path().to_path_buf(), port: 0, reseau: false, dossier_ui: None, demo: true, reseau_sans_licence: false };
    let etat = Etat::ouvrir(config).unwrap();
    etat.relais.lock().unwrap().intervalle_ms = 100;
    let ecoute = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let adresse = ecoute.local_addr().unwrap();
    tokio::spawn(youma_server::servir(etat, ecoute));
    let s = Serveur { url: format!("http://{adresse}/api"), adresse, _dossier: dossier, client: reqwest::Client::new() };

    let proprio = s.connexion("Mariam", "1234").await;
    let caissier = s.connexion("Kadi", "3333").await;
    s.elever(&proprio, "baobab123").await;
    let (_, mut p) = s.get(&proprio, "/parametres").await;
    p["canaux"]["en_ligne"] = json!(true);
    p["canaux"]["relais_url"] = json!(format!("{relais}/"));
    p["canaux"]["relais_cle"] = json!(CLE);
    let r = s.client.put(format!("{}/parametres", s.url)).bearer_auth(&proprio).json(&p).send().await.unwrap();
    assert_eq!(r.status().as_u16(), 200);
    // La clé du relais n'est jamais lisible sans connexion ; renvoyer le masque la conserve.
    let (_, etat_public) = s.get("", "/etat").await;
    assert_eq!(etat_public["parametres"]["canaux"]["relais_cle"], "********");
    let mut p2 = etat_public["parametres"].clone();
    p2["canaux"]["paiement_avance"] = json!(true);
    let r = s.client.put(format!("{}/parametres", s.url)).bearer_auth(&proprio).json(&p2).send().await.unwrap();
    assert_eq!(r.status().as_u16(), 200);
    let (_, p3) = s.get(&proprio, "/parametres").await;
    assert_eq!(p3["canaux"]["relais_cle"], CLE);
    let (_, journal) = s.get(&proprio, "/audit?action=parametres.modifier").await;
    assert!(!journal.to_string().contains(CLE), "pas de clé dans le journal");
    s.post(&caissier, "/journee/ouvrir", json!({})).await;

    let attendre = |chemin: String, condition: fn(&Value) -> bool| {
        let c = s.client.clone();
        async move {
            for _ in 0..100 {
                if let Ok(r) = c.get(&chemin).send().await {
                    let v: Value = r.json().await.unwrap_or(Value::Null);
                    if condition(&v) {
                        return v;
                    }
                }
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
            panic!("délai dépassé : {chemin}");
        }
    };
    let menu = attendre(format!("{relais}/api/public/menu"), |m| m["ouvert"] == true).await;
    let produit = menu["produits"][0]["id"].clone();
    let rep: Value = s
        .client
        .post(format!("{relais}/api/public/commandes"))
        .json(&json!({ "canal": "en_ligne", "type": "livraison", "client_nom": "Fanta", "telephone": "76112233",
                       "livraison": { "quartier": "Hamdallaye", "repere": "Mosquée", "telephone": "76112233" },
                       "paiement_mode": "a_la_livraison", "lignes": [{ "produit_id": produit, "quantite": 2 }] }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(rep["statut"], "en_attente", "{rep}");
    let code = rep["code_suivi"].as_str().unwrap().to_string();
    // Le poste reprend la commande : même code de suivi, numéro attribué, publié sur le relais.
    let suivi = attendre(format!("{relais}/api/public/suivi/{code}"), |v| v["numero"].as_i64().unwrap_or(0) > 0).await;
    assert_eq!(suivi["etape"], "recue");
    let (_, relais_etat) = s.get(&proprio, "/relais/etat").await;
    assert_eq!(relais_etat["actif"], true);
    assert!(relais_etat["commandes_recues"].as_u64().unwrap() >= 1);
    assert_eq!(relais_etat["sms"], "simulation");

    let (_, file) = s.get(&caissier, "/entrantes").await;
    let id = file[0]["commande"]["id"].as_str().unwrap().to_string();
    let (c, e) = s.post(&caissier, &format!("/entrantes/{id}/valider"), json!({ "accepter": true })).await;
    assert_eq!(c, 200, "{e}");
    attendre(format!("{relais}/api/public/suivi/{code}"), |v| v["etape"] == "en_preparation").await;

    // Livreur en route : sa position, envoyée au relais (HTTPS), remonte au poste.
    let gerant = s.connexion("Adama", "2222").await;
    let (_, liens) = s.post(&gerant, &format!("/commandes/{id}/liens"), json!({})).await;
    assert_eq!(liens["code_suivi"], code.as_str());
    let livreur_code = liens["code_livreur"].as_str().unwrap().to_string();
    let (_, employes) = s.get(&gerant, "/employes").await;
    let livreur = employes.as_array().unwrap().iter().find(|e| e["fonction"] == "livreur").unwrap()["id"].clone();
    s.post(&gerant, &format!("/livraisons/{id}/assigner"), json!({ "livreur_id": livreur })).await;
    s.post(&gerant, &format!("/livraisons/{id}/statut"), json!({ "statut": "en_route" })).await;
    attendre(format!("{relais}/api/public/suivi/{code}"), |v| v["etape"] == "en_route").await;
    let r = s.client.post(format!("{relais}/api/public/position/{livreur_code}")).json(&json!({ "lat": 12_640_000, "lon": -8_000_000 })).send().await.unwrap();
    assert_eq!(r.status().as_u16(), 200);
    let local = attendre(format!("{}/public/suivi/{code}", s.url), |v| !v["livreur"].is_null()).await;
    assert_eq!(local["livreur"][0], 12_640_000);
}

/// Cloud facultatif de bout en bout : résumé, sauvegarde chiffrée envoyée puis récupérée, espace propriétaire.
#[tokio::test]
async fn cloud_de_bout_en_bout() {
    let dossier_cloud = tempfile::tempdir().unwrap();
    let cle = youma_relais::inscrire_restaurant(dossier_cloud.path(), "Baobab").unwrap();
    let rc = youma_relais::Config {
        dossier_donnees: dossier_cloud.path().to_path_buf(),
        port: 0,
        cle: "cle-relais-inutilisee-ici".into(),
        dossier_ui: None,
        derriere_proxy: false,
        sms: youma_relais::FournisseurSms::Simulation,
    };
    let ecoute = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let cloud = format!("http://{}", ecoute.local_addr().unwrap());
    tokio::spawn(youma_relais::servir(youma_relais::Etat::ouvrir(&rc).unwrap(), None, ecoute));

    let s = serveur().await;
    let proprio = s.connexion("Mariam", "1234").await;
    s.elever(&proprio, "baobab123").await;
    let (_, mut p) = s.get(&proprio, "/parametres").await;
    p["cloud"] = json!({ "url": cloud, "cle": cle, "telephone_proprietaire": "76 00 00 01", "sms_resume": true,
                         "phrase_chiffrement": "phrase du maquis baobab", "mdp_hash": "" });
    let r = s.client.put(format!("{}/parametres", s.url)).bearer_auth(&proprio).json(&p).send().await.unwrap();
    assert_eq!(r.status().as_u16(), 200);
    let (code, e) = s.post(&proprio, "/cloud/mot-de-passe", json!({ "mot_de_passe": "acces-distant-1" })).await;
    assert_eq!(code, 200, "{e}");
    let caissier = s.connexion("Kadi", "3333").await;
    s.post(&caissier, "/journee/ouvrir", json!({})).await;
    let (code, e) = s.post(&proprio, "/sauvegardes", json!({})).await;
    assert_eq!(code, 200, "{e}");

    let (code, etat) = s.post(&proprio, "/cloud/synchroniser", json!({})).await;
    assert_eq!(code, 200, "{etat}");
    assert_eq!(etat["actif"], true);
    assert_eq!(etat["sms"], "simulation");
    assert!(etat["derniere_sauvegarde"].as_str().unwrap().starts_with("youma-"));
    // La sauvegarde stockée dans le cloud est illisible sans la phrase.
    let fichiers: Vec<_> = std::fs::read_dir(dossier_cloud.path().join("sauvegardes")).unwrap().flatten().collect();
    let stocke = std::fs::read_dir(fichiers[0].path()).unwrap().flatten().next().unwrap().path();
    let octets = std::fs::read(stocke).unwrap();
    assert!(octets.starts_with(b"YOUMA1") && !octets.windows(15).any(|w| w == b"SQLite format 3"));

    // Récupération : téléchargée, déchiffrée, rangée avec les sauvegardes locales.
    let (_, liste) = s.get(&proprio, "/cloud/sauvegardes").await;
    let id = liste[0]["id"].as_str().unwrap();
    let (code, r) = s.post(&proprio, &format!("/cloud/sauvegardes/{id}/recuperer"), json!({})).await;
    assert_eq!(code, 200, "{r}");
    let chemin = r["chemin"].as_str().unwrap();
    assert!(chemin.ends_with("-cloud.db"));
    assert!(std::fs::read(chemin).unwrap().starts_with(b"SQLite format 3"));
    // Une deuxième synchronisation ne renvoie pas la même sauvegarde (et ignore celle venue du cloud).
    s.post(&proprio, "/cloud/synchroniser", json!({})).await;
    let (_, liste) = s.get(&proprio, "/cloud/sauvegardes").await;
    assert_eq!(liste.as_array().unwrap().len(), 1);

    // Espace propriétaire : numéro + mot de passe défini sur le poste.
    let c = reqwest::Client::new();
    let r: Value = c
        .post(format!("{cloud}/api/proprietaire/connexion"))
        .json(&json!({ "telephone": "76000001", "mot_de_passe": "acces-distant-1" }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let t: Value = c.get(format!("{cloud}/api/proprietaire/tableau")).bearer_auth(r["jeton"].as_str().unwrap()).send().await.unwrap().json().await.unwrap();
    assert_eq!(t["restaurants"][0]["nom"], "Maquis Le Baobab");
    assert_eq!(t["restaurants"][0]["resumes"][0]["cloturee"], false);
    // La clé du cloud et la phrase ne sont jamais lisibles sans connexion.
    let (_, etat_public) = s.get("", "/etat").await;
    assert_eq!(etat_public["parametres"]["cloud"]["phrase_chiffrement"], "********");
    assert_eq!(etat_public["parametres"]["cloud"]["cle"], "********");
}
