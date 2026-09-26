//! Mot de passe d'administration oublié (RG-AUT-07, fiche 0031), sans Internet.
//!
//! Le propriétaire, connecté avec son PIN, choisit un nouveau mot de passe en prouvant qu'il est bien le
//! propriétaire par l'un des deux moyens :
//! - le **code de secours** donné à l'installation (à noter sur papier), renouvelable dans l'administration ;
//! - la **réponse du fournisseur** : le poste affiche un code de demande, le fournisseur le signe avec la clé des
//!   licences (`youma-licence secours`) et renvoie la réponse (WhatsApp, SMS).
//!
//! Chaque code ne sert qu'une fois : après une réinitialisation, un nouveau code de secours est donné.

use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;
use base64::Engine;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::Rng;
use rusqlite::{params, Connection};

use crate::auth::{hacher, hash_jeton, verifier, verifier_session};
use crate::db::{definir_valeur_systeme, trouver, valeur_systeme, Acteur, Db, Op};
use crate::erreur::{Erreur, Resultat};
use crate::licence::cle_publique;
use crate::permissions as perm;

const CLE_CODE: &str = "secours.code_hash";
const CLE_DEMANDE: &str = "secours.demande";
/// Sans lettres ni chiffres qui se confondent (0/O, 1/I/L) : le code est recopié à la main.
const ALPHABET: &[u8] = b"ABCDEFGHJKMNPQRSTUVWXYZ23456789";
const ECHECS_MAX: i64 = 5;
const VERROUILLAGE_MS: i64 = 5 * 60_000;

/// Preuve apportée pour réinitialiser le mot de passe.
pub enum Preuve<'a> {
    CodeSecours(&'a str),
    ReponseFournisseur(&'a str),
}

fn tirer(groupes: usize) -> String {
    let mut r = rand::thread_rng();
    (0..groupes)
        .map(|_| (0..4).map(|_| ALPHABET[r.gen_range(0..ALPHABET.len())] as char).collect::<String>())
        .collect::<Vec<_>>()
        .join("-")
}

/// Majuscules, sans espaces ni tirets : « abcd efgh » et « ABCD-EFGH » sont le même code.
fn normaliser(code: &str) -> String {
    code.chars().filter(|c| c.is_ascii_alphanumeric()).map(|c| c.to_ascii_uppercase()).collect()
}

/// Tire un nouveau code de secours (16 caractères, environ 79 bits), n'en garde que l'empreinte, le renvoie en clair.
fn remplacer_code(conn: &Connection) -> Resultat<String> {
    let code = tirer(4);
    definir_valeur_systeme(conn, CLE_CODE, &hacher(&normaliser(&code))?)?;
    Ok(code)
}

/// À l'installation : premier code de secours, affiché une seule fois.
pub(crate) fn code_initial(op: &Op) -> Resultat<String> {
    remplacer_code(op)
}

pub fn code_existe(conn: &Connection) -> Resultat<bool> {
    Ok(valeur_systeme(conn, CLE_CODE)?.is_some())
}

/// Administration : nouveau code de secours (l'ancien ne sert plus). Mot de passe exigé (RG-AUT-06).
pub fn renouveler_code(db: &mut Db, acteur: &Acteur) -> Resultat<String> {
    db.executer(acteur, |op| {
        op.exiger(perm::UTILISATEUR_GERER)?;
        let code = remplacer_code(op)?;
        op.audit("secours.code_renouvele", "systeme", None, None, None, None, None)?;
        Ok(code)
    })
}

/// Code de demande à dicter au fournisseur ; le même tant qu'il n'a pas servi.
pub fn code_demande(conn: &Connection) -> Resultat<String> {
    if let Some(d) = valeur_systeme(conn, CLE_DEMANDE)? {
        return Ok(d);
    }
    let d = tirer(3);
    definir_valeur_systeme(conn, CLE_DEMANDE, &d)?;
    Ok(d)
}

fn message(demande: &str) -> Vec<u8> {
    format!("youma-secours:{}", normaliser(demande)).into_bytes()
}

/// Côté fournisseur (outil `youma-licence secours`).
pub fn signer_reponse(demande: &str, cle: &SigningKey) -> String {
    B64.encode(cle.sign(&message(demande)).to_bytes())
}

fn reponse_valide(demande: &str, reponse: &str, cle: &VerifyingKey) -> bool {
    let brut: String = reponse.chars().filter(|c| !c.is_whitespace()).collect();
    B64.decode(brut.trim_end_matches('='))
        .ok()
        .and_then(|v| <[u8; 64]>::try_from(v).ok())
        .is_some_and(|s| cle.verify(&message(demande), &Signature::from_bytes(&s)).is_ok())
}

pub fn reinitialiser(db: &mut Db, jeton: &str, preuve: Preuve, nouveau: &str) -> Resultat<String> {
    reinitialiser_avec_cle(db, jeton, preuve, nouveau, &cle_publique()?)
}

/// RG-AUT-07 : le propriétaire connecté (PIN) remplace son mot de passe d'administration grâce au code de secours
/// ou à la réponse du fournisseur. Les échecs comptent avec ceux du PIN (verrouillage RG-AUT-02).
/// Renvoie le nouveau code de secours ; la session est confirmée (administration ouverte).
pub fn reinitialiser_avec_cle(db: &mut Db, jeton: &str, preuve: Preuve, nouveau: &str, cle: &VerifyingKey) -> Resultat<String> {
    crate::auth::valider_mot_de_passe(nouveau)?;
    let s = verifier_session(db, jeton)?;
    let uid = s.utilisateur_id.clone();
    let acteur = Acteur::utilisateur(uid.clone());
    db.executer_sans_garde(&acteur, |op| {
        let (role, echecs, verrou): (String, i64, Option<i64>) = trouver(
            op.query_row(
                "SELECT r.code, u.echecs_pin, u.verrouille_jusqu_a FROM utilisateurs u JOIN roles r ON r.id = u.role_id WHERE u.id = ?1",
                params![uid],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            ),
            "Utilisateur",
        )?;
        if role != "proprietaire" {
            return Ok(Err(Erreur::regle(
                "RG-AUT-07",
                "Seul le propriétaire peut réinitialiser son mot de passe ainsi ; les autres le demandent au propriétaire",
            )));
        }
        if verrou.is_some_and(|v| v > op.maintenant) {
            return Ok(Err(Erreur::Verrouille));
        }
        let valide = match preuve {
            Preuve::CodeSecours(code) => valeur_systeme(op, CLE_CODE)?.is_some_and(|h| verifier(&normaliser(code), &h)),
            Preuve::ReponseFournisseur(reponse) => valeur_systeme(op, CLE_DEMANDE)?.is_some_and(|d| reponse_valide(&d, reponse, cle)),
        };
        if !valide {
            let echecs = echecs + 1;
            let verrou = (echecs >= ECHECS_MAX).then_some(op.maintenant + VERROUILLAGE_MS);
            op.execute(
                "UPDATE utilisateurs SET echecs_pin = ?1, verrouille_jusqu_a = ?2 WHERE id = ?3",
                params![if verrou.is_some() { 0 } else { echecs }, verrou, uid],
            )?;
            return Ok(Err(if verrou.is_some() {
                Erreur::Verrouille
            } else {
                Erreur::validation(match preuve {
                    Preuve::CodeSecours(_) => "Code de secours incorrect",
                    Preuve::ReponseFournisseur(_) => "Réponse du fournisseur incorrecte (elle correspond à un autre code de demande ?)",
                })
            }));
        }
        op.execute(
            "UPDATE utilisateurs SET mot_de_passe_hash = ?1, echecs_pin = 0, verrouille_jusqu_a = NULL, modifie_le = ?2 WHERE id = ?3",
            params![hacher(nouveau)?, op.maintenant, uid],
        )?;
        // Les autres sessions de l'utilisateur sont fermées ; celle-ci est confirmée.
        op.execute("UPDATE sessions SET fermee = 1 WHERE utilisateur_id = ?1 AND jeton_hash <> ?2", params![uid, hash_jeton(jeton)])?;
        op.execute("UPDATE sessions SET eleve = 1 WHERE jeton_hash = ?1", params![hash_jeton(jeton)])?;
        op.execute("DELETE FROM systeme WHERE cle = ?1", params![CLE_DEMANDE])?;
        let code = remplacer_code(op)?;
        let moyen = match preuve {
            Preuve::CodeSecours(_) => "code_secours",
            Preuve::ReponseFournisseur(_) => "fournisseur",
        };
        op.audit("utilisateur.mot_de_passe_reinitialise", "utilisateur", Some(&uid), None, Some(serde_json::json!({ "moyen": moyen })), None, None)?;
        Ok(Ok(code))
    })?
}
