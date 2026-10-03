//! Outil FOURNISSEUR. Ne jamais l'installer chez un client : il manipule la clé privée.
//!
//!   youma-licence generer-cles [--fichier cle-privee.txt]
//!   youma-licence emettre --restaurant "Nom" --machine XXXX-XXXX-XXXX-XXXX
//!                         [--modules reseau,livraison] [--maintenance AAAA-MM-JJ] [--numero L-0001]
//!   youma-licence secours --demande XXXX-XXXX-XXXX
//! Clé privée (emettre, secours) : `--cle-privee-fichier <chemin>`, sinon la variable `YOUMA_CLE_PRIVEE`, sinon
//! `--cle-privee <base64>` (déconseillé : elle reste dans l'historique du terminal). Guide : docs/guides/licences.md.
//!       (mot de passe d'administration oublié, RG-AUT-07 : réponse à renvoyer au propriétaire)

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
            match arg(&args, "--fichier") {
                Some(f) => {
                    if std::path::Path::new(&f).exists() {
                        return erreur(&format!("{f} existe déjà : on n'écrase jamais une clé privée"));
                    }
                    if let Err(e) = std::fs::write(&f, B64.encode(cle.to_bytes())) {
                        return erreur(&format!("écriture de {f} : {e}"));
                    }
                    println!("Clé privée écrite dans {f} (à garder secrète, deux copies hors du dépôt)");
                }
                None => println!("Clé privée (à garder secrète) : {}", B64.encode(cle.to_bytes())),
            }
            println!("Clé publique (YOUMA_CLE_PUBLIQUE) : {}", B64.encode(cle.verifying_key().to_bytes()));
        }
        Some("emettre") => {
            let Some(privee) = lire_cle_privee(&args) else { return erreur(MANQUE_CLE) };
            let Some(restaurant) = arg(&args, "--restaurant") else { return erreur("--restaurant manquant") };
            let Some(machine) = arg(&args, "--machine") else { return erreur("--machine manquant") };
            let Some(cle) = cle_privee(&privee) else { return erreur("clé privée invalide") };
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
            match signer(&l, &cle) {
                Ok(texte) => println!("{texte}"),
                Err(e) => erreur(&e.to_string()),
            }
        }
        Some("secours") => {
            let Some(privee) = lire_cle_privee(&args) else { return erreur(MANQUE_CLE) };
            let Some(demande) = arg(&args, "--demande") else { return erreur("--demande manquant") };
            let Some(cle) = cle_privee(&privee) else { return erreur("clé privée invalide") };
            println!("{}", youma_core::secours::signer_reponse(&demande, &cle));
        }
        _ => {
            eprintln!(
                "Usage : youma-licence generer-cles [--fichier …] | secours --demande … | emettre --restaurant … --machine … [--modules …] [--maintenance AAAA-MM-JJ]\n\
                 Clé privée : --cle-privee-fichier …, ou variable YOUMA_CLE_PRIVEE (guide : docs/guides/licences.md)"
            );
            std::process::exit(2);
        }
    }
}

const MANQUE_CLE: &str = "clé privée manquante : --cle-privee-fichier <chemin> ou variable YOUMA_CLE_PRIVEE";

/// Clé privée : fichier, sinon variable d'environnement, sinon argument (déconseillé : historique du terminal).
fn lire_cle_privee(args: &[String]) -> Option<String> {
    if let Some(f) = arg(args, "--cle-privee-fichier") {
        return std::fs::read_to_string(f).ok();
    }
    std::env::var("YOUMA_CLE_PRIVEE").ok().filter(|c| !c.trim().is_empty()).or_else(|| arg(args, "--cle-privee"))
}

fn cle_privee(b64: &str) -> Option<SigningKey> {
    let octets: [u8; 32] = B64.decode(b64.trim()).ok()?.try_into().ok()?;
    Some(SigningKey::from_bytes(&octets))
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
