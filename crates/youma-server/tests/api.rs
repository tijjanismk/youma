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
