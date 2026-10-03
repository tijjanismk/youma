//! Constat C2 (fiche 0042) : un poste central compilé en production doit embarquer la vraie clé publique des licences.
//! Sans `YOUMA_CLE_PUBLIQUE`, `youma-core` retombe sur la clé de développement, publique : n'importe qui pourrait
//! signer des licences. Échappatoire explicite pour les essais : `YOUMA_CLE_DEV=1`.
fn main() {
    println!("cargo:rerun-if-env-changed=YOUMA_CLE_PUBLIQUE");
    println!("cargo:rerun-if-env-changed=YOUMA_CLE_DEV");
    let production = std::env::var("PROFILE").as_deref() == Ok("release");
    let cle = std::env::var("YOUMA_CLE_PUBLIQUE").is_ok_and(|c| !c.trim().is_empty());
    let essai = std::env::var("YOUMA_CLE_DEV").as_deref() == Ok("1");
    if production && !cle && !essai {
        panic!(
            "YOUMA_CLE_PUBLIQUE absente : un poste compilé en production accepterait les licences de la clé de développement. \
             Donnez la clé publique (youma-licence generer-cles), ou YOUMA_CLE_DEV=1 pour une version d'essai."
        );
    }
}
