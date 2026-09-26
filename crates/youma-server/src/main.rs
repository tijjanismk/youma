//! youma-server [--donnees DOSSIER] [--port 7878] [--reseau] [--port-https 7879] [--ui DOSSIER] [--demo]

use std::path::PathBuf;

use youma_server::Config;

fn arg(args: &[String], nom: &str) -> Option<String> {
    args.iter().position(|a| a == nom).and_then(|i| args.get(i + 1)).cloned()
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();
    let args: Vec<String> = std::env::args().collect();
    let donnees = arg(&args, "--donnees").map(PathBuf::from).unwrap_or_else(dossier_par_defaut);
    let port = arg(&args, "--port").and_then(|p| p.parse().ok()).unwrap_or(7878);
    // `--reseau`, ou l'interrupteur d'Administration (écrit dans config.json, fiche 0024).
    let reseau = args.iter().any(|a| a == "--reseau") || youma_server::poste::reseau(&donnees).unwrap_or(false);
    let mut config = Config {
        dossier_donnees: donnees,
        port,
        reseau,
        dossier_ui: arg(&args, "--ui").map(PathBuf::from).or_else(|| Some(PathBuf::from("ui/dist")).filter(|p| p.exists())),
        demo: args.iter().any(|a| a == "--demo"),
        reseau_sans_licence: args.iter().any(|a| a == "--reseau-sans-licence"),
        port_https: None,
    };
    // HTTPS local (fiche 0020) : par défaut en réseau, sur le port suivant ; `--port-https 0` le coupe.
    config.port_https = match arg(&args, "--port-https").and_then(|p| p.parse::<u16>().ok()) {
        Some(0) => None,
        Some(p) => Some(p),
        None => reseau.then_some(port + 1),
    };
    if let Err(e) = youma_server::demarrer(config).await {
        eprintln!("Erreur : {e}");
        std::process::exit(1);
    }
}

/// `%PROGRAMDATA%\Youma` sous Windows, `./donnees` ailleurs.
fn dossier_par_defaut() -> PathBuf {
    std::env::var("PROGRAMDATA").map(|p| PathBuf::from(p).join("Youma")).unwrap_or_else(|_| PathBuf::from("donnees"))
}
