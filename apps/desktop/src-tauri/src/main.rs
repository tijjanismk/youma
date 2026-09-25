//! Coquille Windows du poste central.
//! Démarre youma-server dans le même processus, puis ouvre une fenêtre sur http://127.0.0.1:<port>.
//! L'interface est exactement celle servie aux téléphones : aucun code propre au mode (fiche 0002).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::net::TcpStream;
use std::path::PathBuf;
use std::time::Duration;

use serde::Deserialize;
use tauri::{WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_autostart::MacosLauncher;

/// `%PROGRAMDATA%\Youma\config.json` (écrit par l'installateur ou l'écran d'administration).
#[derive(Deserialize, Default)]
#[serde(default)]
struct ConfigPoste {
    port: Option<u16>,
    /// Mode B : les téléphones du réseau local se connectent à ce poste.
    reseau: bool,
    /// Base de démonstration (formation, prospection).
    demo: bool,
}

fn dossier_donnees() -> PathBuf {
    std::env::var("PROGRAMDATA").map(|p| PathBuf::from(p).join("Youma")).unwrap_or_else(|_| PathBuf::from("donnees"))
}

fn port_ouvert(port: u16) -> bool {
    TcpStream::connect_timeout(&([127, 0, 0, 1], port).into(), Duration::from_millis(300)).is_ok()
}

fn main() {
    tracing_subscriber::fmt().with_env_filter("info").init();
    let donnees = dossier_donnees();
    let conf: ConfigPoste = std::fs::read_to_string(donnees.join("config.json"))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    let port = conf.port.unwrap_or(7878);
    let ui = std::env::current_exe().ok().and_then(|e| e.parent().map(|d| d.join("ui"))).filter(|d| d.exists());

    // Serveur déjà lancé (autre instance, service) : on se contente d'ouvrir la fenêtre.
    if !port_ouvert(port) {
        let config = youma_server::Config {
            dossier_donnees: donnees,
            port,
            reseau: conf.reseau,
            dossier_ui: ui,
            demo: conf.demo,
            reseau_sans_licence: false,
            // HTTPS local pour installer l'application sur les téléphones (fiche 0020).
            port_https: conf.reseau.then_some(port + 1),
        };
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("runtime tokio");
            if let Err(e) = rt.block_on(youma_server::demarrer(config)) {
                eprintln!("Serveur Youma : {e}");
            }
        });
        for _ in 0..100 {
            if port_ouvert(port) {
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    let url = format!("http://127.0.0.1:{port}/");
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            use tauri::Manager;
            if let Some(w) = app.get_webview_window("principale") {
                let _ = w.set_focus();
            }
        }))
        // Démarrage automatique avec Windows (cahier §22).
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, None))
        .setup(move |app| {
            use tauri_plugin_autostart::ManagerExt;
            let _ = app.autolaunch().enable();
            WebviewWindowBuilder::new(app, "principale", WebviewUrl::External(url.parse().expect("url locale")))
                .title("Youma")
                .inner_size(1280.0, 800.0)
                .min_inner_size(900.0, 600.0)
                .maximized(true)
                .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("fenêtre Youma");
}
