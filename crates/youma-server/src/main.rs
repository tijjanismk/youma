//! youma-server [--donnees DOSSIER] [--port 7878] [--reseau] [--ui DOSSIER] [--demo]

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
    let config = Config {
        dossier_donnees: donnees,
        port: arg(&args, "--port").and_then(|p| p.parse().ok()).unwrap_or(7878),
        reseau: args.iter().any(|a| a == "--reseau"),
        dossier_ui: arg(&args, "--ui").map(PathBuf::from).or_else(|| Some(PathBuf::from("ui/dist")).filter(|p| p.exists())),
        demo: args.iter().any(|a| a == "--demo"),
        reseau_sans_licence: args.iter().any(|a| a == "--reseau-sans-licence"),
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
