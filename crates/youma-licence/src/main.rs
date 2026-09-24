//! Outil FOURNISSEUR. Ne jamais l'installer chez un client : il manipule la clé privée.
//!
//!   youma-licence generer-cles
//!   youma-licence emettre --cle-privee <base64> --restaurant "Nom" --machine XXXX-XXXX-XXXX-XXXX
//!                         [--modules reseau,livraison] [--maintenance AAAA-MM-JJ] [--numero L-0001]

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use ed25519_dalek::SigningKey;
use youma_core::licence::{signer, Licence, MODULES};

fn arg(args: &[String], nom: &str) -> Option<String> {
    args.iter().position(|a| a == nom).and_then(|i| args.get(i + 1)).cloned()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("generer-cles") => {
            let cle = SigningKey::generate(&mut rand::rngs::OsRng);
            println!("Clé privée (à garder secrète) : {}", B64.encode(cle.to_bytes()));
            println!("Clé publique (YOUMA_CLE_PUBLIQUE) : {}", B64.encode(cle.verifying_key().to_bytes()));
        }
        Some("emettre") => {
            let Some(privee) = arg(&args, "--cle-privee") else { return erreur("--cle-privee manquant") };
            let Some(restaurant) = arg(&args, "--restaurant") else { return erreur("--restaurant manquant") };
            let Some(machine) = arg(&args, "--machine") else { return erreur("--machine manquant") };
            let octets: [u8; 32] = match B64.decode(privee.trim()).ok().and_then(|v| v.try_into().ok()) {
                Some(o) => o,
                None => return erreur("clé privée invalide"),
            };
            let modules: Vec<String> = arg(&args, "--modules")
                .map(|m| m.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect())
                .unwrap_or_default();
            if let Some(inconnu) = modules.iter().find(|m| !MODULES.contains(&m.as_str())) {
                return erreur(&format!("module inconnu : {inconnu} (connus : {})", MODULES.join(", ")));
            }
            let l = Licence {
                numero: arg(&args, "--numero").unwrap_or_else(|| "L-0001".into()),
                restaurant,
                code_machine: machine.trim().to_uppercase(),
                modules,
                emise_le: chrono_aujourdhui(),
                maintenance_jusqua: arg(&args, "--maintenance"),
            };
            match signer(&l, &SigningKey::from_bytes(&octets)) {
                Ok(texte) => println!("{texte}"),
                Err(e) => erreur(&e.to_string()),
            }
        }
        _ => {
            eprintln!("Usage : youma-licence generer-cles | emettre --cle-privee … --restaurant … --machine … [--modules …] [--maintenance AAAA-MM-JJ]");
            std::process::exit(2);
        }
    }
}

fn chrono_aujourdhui() -> String {
    let jours = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() / 86_400).unwrap_or(0) as i64;
    // Conversion jours → date civile (algorithme de Howard Hinnant).
    let z = jours + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    format!("{y:04}-{m:02}-{d:02}")
}

fn erreur(m: &str) {
    eprintln!("Erreur : {m}");
    std::process::exit(1);
}
