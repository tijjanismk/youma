//! `config.json` du poste central (dans le dossier des données, `%PROGRAMDATA%\Youma` sous Windows) :
//! réglages lus au démarrage par la coquille Windows et par `youma-server`. Ce n'est pas une donnée métier :
//! il n'est pas dans la base ni dans les sauvegardes.

use std::path::Path;

use serde_json::{json, Value};

const FICHIER: &str = "config.json";

fn lire_valeur(dossier: &Path) -> Value {
    std::fs::read_to_string(dossier.join(FICHIER)).ok().and_then(|s| serde_json::from_str(&s).ok()).filter(Value::is_object).unwrap_or(json!({}))
}

/// Mode réseau demandé dans `config.json` ; `None` si le fichier ne le dit pas.
pub fn reseau(dossier: &Path) -> Option<bool> {
    lire_valeur(dossier)["reseau"].as_bool()
}

/// Change seulement `reseau` : les autres réglages du fichier (port, démonstration…) sont gardés.
pub fn ecrire_reseau(dossier: &Path, actif: bool) -> std::io::Result<()> {
    let mut v = lire_valeur(dossier);
    v["reseau"] = json!(actif);
    std::fs::create_dir_all(dossier)?;
    let provisoire = dossier.join("config.json.nouveau");
    std::fs::write(&provisoire, serde_json::to_string_pretty(&v).unwrap_or_default())?;
    std::fs::rename(provisoire, dossier.join(FICHIER))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn garde_les_autres_reglages() {
        let d = tempfile::tempdir().unwrap();
        assert_eq!(reseau(d.path()), None);
        std::fs::write(d.path().join(FICHIER), r#"{ "port": 8000, "demo": true }"#).unwrap();
        ecrire_reseau(d.path(), true).unwrap();
        assert_eq!(reseau(d.path()), Some(true));
        let v = lire_valeur(d.path());
        assert_eq!(v["port"], 8000);
        assert_eq!(v["demo"], true);
        ecrire_reseau(d.path(), false).unwrap();
        assert_eq!(reseau(d.path()), Some(false));
    }
}
