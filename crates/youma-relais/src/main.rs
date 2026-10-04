//! youma-relais --cle CLE [--donnees DOSSIER] [--port 8080] [--ui DOSSIER] [--derriere-proxy]
//! youma-relais --ajouter-restaurant "NOM" [--donnees DOSSIER]   (cloud : affiche la clé du restaurant)
//!
//! À placer derrière un proxy HTTPS (Caddy, nginx) : la géolocalisation du livreur exige HTTPS.
//! La clé peut aussi venir de la variable d'environnement `YOUMA_RELAIS_CLE`.
//! SMS Orange Mali : `YOUMA_ORANGE_CLIENT_ID`, `YOUMA_ORANGE_CLIENT_SECRET`, `YOUMA_ORANGE_EXPEDITEUR` (+223…),
//! `YOUMA_ORANGE_NOM_EXPEDITEUR` (facultatif). Sans eux : simulation (code affiché au client).
//! WhatsApp (facultatif, fiche 0046) : `YOUMA_WHATSAPP_JETON`, `YOUMA_WHATSAPP_NUMERO_ID`, `YOUMA_WHATSAPP_MODELE`
//! (`youma_code`), `YOUMA_WHATSAPP_LANGUE` (`fr`).

use std::path::PathBuf;

use youma_relais::Config;

fn arg(args: &[String], nom: &str) -> Option<String> {
    args.iter().position(|a| a == nom).and_then(|i| args.get(i + 1)).cloned()
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();
    let args: Vec<String> = std::env::args().collect();
    let donnees = arg(&args, "--donnees").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("donnees-relais"));
    // Cloud (fiche 0018) : inscription d'un restaurant par le fournisseur.
    if let Some(nom) = arg(&args, "--ajouter-restaurant") {
        match youma_relais::inscrire_restaurant(&donnees, &nom) {
            Ok(cle) => {
                let slug = youma_relais::adresse_restaurant(&donnees, &cle).ok().flatten().unwrap_or_default();
                println!("Restaurant « {nom} » inscrit. Clé à saisir sur son poste central : {cle}");
                println!("Adresse de ses commandes en ligne (Adresse du relais) : https://VOTRE-RELAIS/r/{slug}");
            }
            Err(e) => {
                eprintln!("Erreur : {e}");
                std::process::exit(1);
            }
        }
        return;
    }
    let cle = arg(&args, "--cle").or_else(|| std::env::var("YOUMA_RELAIS_CLE").ok()).unwrap_or_default();
    if cle.len() < 16 {
        eprintln!("Clé du relais obligatoire (16 caractères au moins) : --cle ou YOUMA_RELAIS_CLE");
        std::process::exit(2);
    }
    let config = Config {
        dossier_donnees: donnees,
        port: arg(&args, "--port").and_then(|p| p.parse().ok()).unwrap_or(8080),
        cle,
        dossier_ui: arg(&args, "--ui").map(PathBuf::from).or_else(|| Some(PathBuf::from("ui/dist")).filter(|p| p.exists())),
        derriere_proxy: args.iter().any(|a| a == "--derriere-proxy"),
        sms: youma_relais::FournisseurSms::depuis_environnement(),
        whatsapp: youma_relais::WhatsApp::depuis_environnement(),
    };
    tracing::info!("SMS : {} ; WhatsApp : {}", config.sms.nom(), if config.whatsapp.is_some() { "configuré" } else { "non configuré" });
    if let Err(e) = youma_relais::demarrer(config).await {
        eprintln!("Erreur : {e}");
        std::process::exit(1);
    }
}
