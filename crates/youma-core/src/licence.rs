//! Licence signée (Ed25519), activation hors ligne (fiche 0007).
//! RG-SYS-06 : la licence ne conditionne que les modules, jamais la vente.

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};

use crate::db::{installation_id, Acteur, Db};
use crate::erreur::{Erreur, Resultat};
use crate::permissions as perm;

/// Clé publique du fournisseur. En production, fournie à la compilation par `YOUMA_CLE_PUBLIQUE`
/// (base64). Par défaut : clé de DÉVELOPPEMENT (sa clé privée est dans `outils/cle-dev.txt`).
const CLE_DEV: &str = "WP3WQdFMZgUtvk/I0GO67IbJvk6ylvIeUgHGvg27O8s=";

pub fn cle_publique() -> Resultat<VerifyingKey> {
    let b64 = option_env!("YOUMA_CLE_PUBLIQUE").unwrap_or(CLE_DEV);
    decoder_cle(b64)
}

pub fn decoder_cle(b64: &str) -> Resultat<VerifyingKey> {
    let octets: [u8; 32] = B64
        .decode(b64.trim())
        .ok()
        .and_then(|v| v.try_into().ok())
        .ok_or_else(|| Erreur::validation("Clé publique invalide"))?;
    VerifyingKey::from_bytes(&octets).map_err(|_| Erreur::validation("Clé publique invalide"))
}

pub const MODULES: &[&str] = &["reseau", "livraison", "cloud", "qr", "multi_etablissement"];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Licence {
    pub numero: String,
    pub restaurant: String,
    pub code_machine: String,
    pub modules: Vec<String>,
    /// AAAA-MM-JJ
    pub emise_le: String,
    /// Fin du contrat de maintenance (mises à jour, support). AAAA-MM-JJ.
    pub maintenance_jusqua: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenceSignee {
    pub licence: Licence,
    pub signature: String,
}

/// Code machine : lié à l'installation ET au PC (nom de machine).
/// Restaurer la base sur un autre PC change le code : réactivation nécessaire (transfert).
pub fn code_machine(conn: &Connection) -> Resultat<String> {
    let machine = std::env::var("COMPUTERNAME").or_else(|_| std::env::var("HOSTNAME")).unwrap_or_default();
    let h = Sha256::digest(format!("{}|{}", installation_id(conn)?, machine.to_uppercase()).as_bytes());
    let hex = crate::auth::hex(&h[..8]).to_uppercase();
    Ok(format!("{}-{}-{}-{}", &hex[0..4], &hex[4..8], &hex[8..12], &hex[12..16]))
}

fn message(l: &Licence) -> Resultat<Vec<u8>> {
    Ok(serde_json::to_vec(l)?)
}

/// Côté fournisseur (outil `youma-licence`).
pub fn signer(l: &Licence, cle: &SigningKey) -> Resultat<String> {
    let sig = cle.sign(&message(l)?);
    let s = LicenceSignee { licence: l.clone(), signature: B64.encode(sig.to_bytes()) };
    Ok(B64.encode(serde_json::to_vec(&s)?))
}

/// Décode et vérifie une licence (texte base64 envoyé par WhatsApp / SMS / clé USB).
pub fn verifier(texte: &str, cle: &VerifyingKey) -> Resultat<Licence> {
    let brut: String = texte.chars().filter(|c| !c.is_whitespace()).collect();
    let json = B64.decode(brut).map_err(|_| Erreur::validation("Code de licence illisible"))?;
    let s: LicenceSignee = serde_json::from_slice(&json).map_err(|_| Erreur::validation("Code de licence illisible"))?;
    let sig: [u8; 64] = B64
        .decode(&s.signature)
        .ok()
        .and_then(|v| v.try_into().ok())
        .ok_or_else(|| Erreur::validation("Signature illisible"))?;
    cle.verify(&message(&s.licence)?, &Signature::from_bytes(&sig))
        .map_err(|_| Erreur::validation("Licence non authentique"))?;
    Ok(s.licence)
}

pub fn installer(db: &mut Db, acteur: &Acteur, texte: &str) -> Resultat<Licence> {
    installer_avec_cle(db, acteur, texte, &cle_publique()?)
}

pub fn installer_avec_cle(db: &mut Db, acteur: &Acteur, texte: &str, cle: &VerifyingKey) -> Resultat<Licence> {
    let l = verifier(texte, cle)?;
    let code = code_machine(db.conn())?;
    if l.code_machine != code {
        return Err(Erreur::validation(format!(
            "Cette licence est pour la machine {}, ce PC est {code}. Demandez un transfert.",
            l.code_machine
        )));
    }
    db.executer(acteur, |op| {
        op.exiger(perm::LICENCE_GERER)?;
        op.execute(
            "INSERT INTO systeme(cle, valeur) VALUES ('licence', ?1) ON CONFLICT(cle) DO UPDATE SET valeur = excluded.valeur",
            params![texte.trim()],
        )?;
        op.audit("licence.installer", "systeme", None, None, Some(json!(l)), None, None)?;
        Ok(())
    })?;
    Ok(l)
}

#[derive(Debug, Serialize)]
pub struct EtatLicence {
    pub code_machine: String,
    pub licence: Option<Licence>,
    pub valide: bool,
    pub maintenance_active: bool,
    pub modules: Vec<String>,
    pub message: String,
}

pub fn etat(conn: &Connection, date_du_jour: &str) -> Resultat<EtatLicence> {
    etat_avec_cle(conn, date_du_jour, &cle_publique()?)
}

pub fn etat_avec_cle(conn: &Connection, date_du_jour: &str, cle: &VerifyingKey) -> Resultat<EtatLicence> {
    let code = code_machine(conn)?;
    let texte: Option<String> =
        conn.query_row("SELECT valeur FROM systeme WHERE cle = 'licence'", [], |r| r.get(0)).optional()?;
    let Some(texte) = texte else {
        return Ok(EtatLicence {
            code_machine: code,
            licence: None,
            valide: false,
            maintenance_active: false,
            modules: vec![],
            message: "Aucune licence : toutes les fonctions locales du poste restent disponibles.".into(),
        });
    };
    match verifier(&texte, cle) {
        Ok(l) if l.code_machine == code => {
            let maintenance = l.maintenance_jusqua.as_deref().is_none_or(|d| d >= date_du_jour);
            let message = if maintenance {
                "Licence valide.".to_string()
            } else {
                "Maintenance expirée : les ventes continuent, les mises à jour et modules cloud sont suspendus.".to_string()
            };
            // RG-SYS-06 : maintenance expirée → seuls les modules cloud s'arrêtent.
            let modules = l
                .modules
                .iter()
                .filter(|m| maintenance || !["cloud", "qr"].contains(&m.as_str()))
                .cloned()
                .collect();
            Ok(EtatLicence { code_machine: code, valide: true, maintenance_active: maintenance, modules, message, licence: Some(l) })
        }
        Ok(l) => Ok(EtatLicence {
            code_machine: code,
            licence: Some(l),
            valide: false,
            maintenance_active: false,
            modules: vec![],
            message: "Licence d'un autre PC : demandez un transfert. Les ventes continuent.".into(),
        }),
        Err(_) => Ok(EtatLicence {
            code_machine: code,
            licence: None,
            valide: false,
            maintenance_active: false,
            modules: vec![],
            message: "Licence illisible. Les ventes continuent.".into(),
        }),
    }
}

pub fn module_actif(conn: &Connection, module: &str, date_du_jour: &str) -> bool {
    etat(conn, date_du_jour).map(|e| e.modules.iter().any(|m| m == module)).unwrap_or(false)
}
