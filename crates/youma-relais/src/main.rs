//! youma-relais --cle CLE [--donnees DOSSIER] [--port 8080] [--ui DOSSIER] [--derriere-proxy]
//!
//! À placer derrière un proxy HTTPS (Caddy, nginx) : la géolocalisation du livreur exige HTTPS.
//! La clé peut aussi venir de la variable d'environnement `YOUMA_RELAIS_CLE`.

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
    let cle = arg(&args, "--cle").or_else(|| std::env::var("YOUMA_RELAIS_CLE").ok()).unwrap_or_default();
    if cle.len() < 16 {
        eprintln!("Clé du relais obligatoire (16 caractères au moins) : --cle ou YOUMA_RELAIS_CLE");
        std::process::exit(2);
    }
    let config = Config {
        dossier_donnees: arg(&args, "--donnees").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("donnees-relais")),
        port: arg(&args, "--port").and_then(|p| p.parse().ok()).unwrap_or(8080),
        cle,
        dossier_ui: arg(&args, "--ui").map(PathBuf::from).or_else(|| Some(PathBuf::from("ui/dist")).filter(|p| p.exists())),
        derriere_proxy: args.iter().any(|a| a == "--derriere-proxy"),
    };
    if let Err(e) = youma_relais::demarrer(config).await {
        eprintln!("Erreur : {e}");
        std::process::exit(1);
    }
}
