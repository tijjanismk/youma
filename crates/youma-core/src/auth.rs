use std::collections::HashSet;

use argon2::password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::{Algorithm, Argon2, Params, Version};
use base64::Engine;
use rand::RngCore;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};

use crate::db::{trouver, Acteur, Db, Op};
use crate::erreur::{Erreur, Resultat};
use crate::permissions as perm;

const ECHECS_MAX: i64 = 5;
const VERROUILLAGE_MS: i64 = 5 * 60_000;

fn argon() -> Argon2<'static> {
    // Réglage léger : PC anciens, et le PIN est protégé par le verrouillage (RG-AUT-02).
    Argon2::new(Algorithm::Argon2id, Version::V0x13, Params::new(4_096, 2, 1, None).expect("paramètres argon2"))
}

pub fn hacher(secret: &str) -> Resultat<String> {
    let sel = SaltString::generate(&mut OsRng);
    argon()
        .hash_password(secret.as_bytes(), &sel)
        .map(|h| h.to_string())
        .map_err(|e| Erreur::validation(format!("hachage impossible : {e}")))
}

pub fn verifier(secret: &str, hash: &str) -> bool {
    PasswordHash::new(hash).map(|h| argon().verify_password(secret.as_bytes(), &h).is_ok()).unwrap_or(false)
}

pub fn hash_jeton(jeton: &str) -> String {
    hex(&Sha256::digest(jeton.as_bytes()))
}

pub fn hex(octets: &[u8]) -> String {
    octets.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn nouveau_jeton() -> String {
    let mut b = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut b);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(b)
}

/// RG-AUT-01.
fn valider_pin(pin: &str) -> Resultat<()> {
    if !(4..=6).contains(&pin.len()) || !pin.chars().all(|c| c.is_ascii_digit()) {
        return Err(Erreur::regle("RG-AUT-01", "Le code PIN doit contenir 4 à 6 chiffres"));
    }
    Ok(())
}

pub struct Autorisateur {
    pub utilisateur_id: String,
    pub permissions: HashSet<String>,
    pub plafond_remise_pct: i64,
}

pub fn permissions_utilisateur(conn: &Connection, utilisateur_id: &str) -> Resultat<(HashSet<String>, i64)> {
    let (role_id, actif, plafond): (String, bool, i64) = trouver(
        conn.query_row(
            "SELECT u.role_id, u.actif, r.plafond_remise_pct FROM utilisateurs u JOIN roles r ON r.id = u.role_id
             WHERE u.id = ?1",
            params![utilisateur_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        ),
        "Utilisateur",
    )?;
    if !actif {
        return Err(Erreur::NonAuthentifie);
    }
    let mut stmt = conn.prepare_cached("SELECT permission FROM role_permissions WHERE role_id = ?1")?;
    let perms = stmt.query_map(params![role_id], |r| r.get::<_, String>(0))?.collect::<Result<HashSet<_>, _>>()?;
    Ok((perms, plafond))
}

/// Permissions d'un rôle par son code (diagnostic, tests de migration).
pub fn permissions_utilisateur_role(conn: &Connection, code: &str) -> (HashSet<String>, i64) {
    let r = conn.query_row("SELECT id, plafond_remise_pct FROM roles WHERE code = ?1", params![code], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)));
    let Ok((id, plafond)) = r else { return (HashSet::new(), 0) };
    let perms = conn
        .prepare("SELECT permission FROM role_permissions WHERE role_id = ?1")
        .and_then(|mut s| s.query_map(params![id], |r| r.get::<_, String>(0))?.collect::<Result<HashSet<_>, _>>())
        .unwrap_or_default();
    (perms, plafond)
}

/// Trouve l'utilisateur actif qui possède ce PIN.
fn utilisateur_par_pin(conn: &Connection, pin: &str) -> Resultat<Option<String>> {
    let mut stmt = conn.prepare("SELECT id, pin_hash FROM utilisateurs WHERE actif = 1")?;
    let lignes = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
    for l in lignes {
        let (id, h) = l?;
        if verifier(pin, &h) {
            return Ok(Some(id));
        }
    }
    Ok(None)
}

/// RG-AUT-03 : autorisation ponctuelle par le PIN d'un responsable présent.
pub fn verifier_pin_autorisation(conn: &Connection, pin: &str, _maintenant: i64) -> Resultat<Autorisateur> {
    let id = utilisateur_par_pin(conn, pin)?.ok_or(Erreur::PinIncorrect)?;
    let (permissions, plafond) = permissions_utilisateur(conn, &id)?;
    Ok(Autorisateur { utilisateur_id: id, permissions, plafond_remise_pct: plafond })
}

#[derive(Debug, Serialize, Clone)]
pub struct UtilisateurResume {
    pub id: String,
    pub nom: String,
    pub role_code: String,
    pub role_nom: String,
    pub actif: bool,
    pub employe_id: Option<String>,
    pub a_mot_de_passe: bool,
}

pub fn lister_utilisateurs(conn: &Connection, actifs_seulement: bool) -> Resultat<Vec<UtilisateurResume>> {
    let mut stmt = conn.prepare(
        "SELECT u.id, u.nom, r.code, r.nom, u.actif, u.employe_id, u.mot_de_passe_hash IS NOT NULL
         FROM utilisateurs u JOIN roles r ON r.id = u.role_id
         WHERE (?1 = 0 OR u.actif = 1) ORDER BY u.nom",
    )?;
    let v = stmt
        .query_map(params![actifs_seulement], |r| {
            Ok(UtilisateurResume {
                id: r.get(0)?,
                nom: r.get(1)?,
                role_code: r.get(2)?,
                role_nom: r.get(3)?,
                actif: r.get(4)?,
                employe_id: r.get(5)?,
                a_mot_de_passe: r.get(6)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(v)
}

pub fn nombre_utilisateurs(conn: &Connection) -> Resultat<i64> {
    Ok(conn.query_row("SELECT COUNT(*) FROM utilisateurs", [], |r| r.get(0))?)
}

#[derive(Debug, Deserialize)]
pub struct NouvelUtilisateur {
    pub nom: String,
    pub role_code: String,
    pub pin: String,
    #[serde(default)]
    pub mot_de_passe: Option<String>,
    #[serde(default)]
    pub employe_id: Option<String>,
}

fn role_id(conn: &Connection, code: &str) -> Resultat<String> {
    trouver(conn.query_row("SELECT id FROM roles WHERE code = ?1", params![code], |r| r.get(0)), "Rôle")
}

fn inserer_utilisateur(op: &Op, u: &NouvelUtilisateur) -> Resultat<String> {
    valider_pin(&u.pin)?;
    if u.nom.trim().is_empty() {
        return Err(Erreur::validation("Le nom est obligatoire"));
    }
    if utilisateur_par_pin(op, &u.pin)?.is_some() {
        return Err(Erreur::regle("RG-AUT-01", "Ce code PIN est déjà utilisé par un autre utilisateur"));
    }
    let rid = role_id(op, &u.role_code)?;
    let id = op.nouvel_id();
    let mdp = match &u.mot_de_passe {
        Some(m) if !m.is_empty() => {
            valider_mot_de_passe(m)?;
            Some(hacher(m)?)
        }
        _ => None,
    };
    op.execute(
        "INSERT INTO utilisateurs(id, nom, role_id, pin_hash, mot_de_passe_hash, employe_id, cree_le, modifie_le)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
        params![id, u.nom.trim(), rid, hacher(&u.pin)?, mdp, u.employe_id, op.maintenant],
    )?;
    op.audit("utilisateur.creer", "utilisateur", Some(&id), None, Some(json!({"nom": u.nom, "role": u.role_code})), None, None)?;
    Ok(id)
}

/// RG-AUT-06 : 6 caractères au moins.
fn valider_mot_de_passe(m: &str) -> Resultat<()> {
    if m.chars().count() < 6 {
        return Err(Erreur::regle("RG-AUT-06", "Le mot de passe doit contenir au moins 6 caractères"));
    }
    Ok(())
}

/// Première configuration : crée le propriétaire (uniquement si aucun utilisateur).
/// Le mot de passe protège l'administration (RG-AUT-06).
pub fn installer_proprietaire(db: &mut Db, nom: &str, pin: &str, mot_de_passe: &str, nom_restaurant: &str) -> Resultat<String> {
    valider_mot_de_passe(mot_de_passe)?;
    db.executer(&Acteur::systeme(), |op| {
        if nombre_utilisateurs(op)? > 0 {
            return Err(Erreur::validation("L'installation est déjà faite"));
        }
        if !nom_restaurant.trim().is_empty() {
            op.execute(
                "UPDATE restaurant SET nom = ?1, modifie_le = ?2",
                params![nom_restaurant.trim(), op.maintenant],
            )?;
        }
        inserer_utilisateur(
            op,
            &NouvelUtilisateur {
                nom: nom.into(),
                role_code: "proprietaire".into(),
                pin: pin.into(),
                mot_de_passe: Some(mot_de_passe.into()),
                employe_id: None,
            },
        )
    })
}

pub fn creer_utilisateur(db: &mut Db, acteur: &Acteur, u: &NouvelUtilisateur) -> Resultat<String> {
    db.executer(acteur, |op| {
        op.exiger(perm::UTILISATEUR_GERER)?;
        inserer_utilisateur(op, u)
    })
}

#[derive(Debug, Deserialize)]
pub struct ModifUtilisateur {
    pub id: String,
    pub nom: Option<String>,
    pub role_code: Option<String>,
    pub pin: Option<String>,
    pub actif: Option<bool>,
    pub employe_id: Option<Option<String>>,
}

pub fn modifier_utilisateur(db: &mut Db, acteur: &Acteur, m: &ModifUtilisateur) -> Resultat<()> {
    db.executer(acteur, |op| {
        op.exiger(perm::UTILISATEUR_GERER)?;
        let role_actuel: String = trouver(
            op.query_row(
                "SELECT r.code FROM utilisateurs u JOIN roles r ON r.id = u.role_id WHERE u.id = ?1",
                params![m.id],
                |r| r.get(0),
            ),
            "Utilisateur",
        )?;
        if role_actuel == "proprietaire" {
            let nb: i64 = op.query_row(
                "SELECT COUNT(*) FROM utilisateurs u JOIN roles r ON r.id = u.role_id
                 WHERE r.code = 'proprietaire' AND u.actif = 1",
                [],
                |r| r.get(0),
            )?;
            let retire = m.actif == Some(false) || m.role_code.as_deref().is_some_and(|c| c != "proprietaire");
            if retire && nb <= 1 {
                return Err(Erreur::regle("RG-AUT-05", "Il faut garder au moins un propriétaire actif"));
            }
        }
        if let Some(nom) = &m.nom {
            op.execute("UPDATE utilisateurs SET nom = ?1 WHERE id = ?2", params![nom, m.id])?;
        }
        if let Some(code) = &m.role_code {
            let rid = role_id(op, code)?;
            op.execute("UPDATE utilisateurs SET role_id = ?1 WHERE id = ?2", params![rid, m.id])?;
        }
        if let Some(pin) = &m.pin {
            valider_pin(pin)?;
            if let Some(autre) = utilisateur_par_pin(op, pin)? {
                if autre != m.id {
                    return Err(Erreur::regle("RG-AUT-01", "Ce code PIN est déjà utilisé"));
                }
            }
            op.execute("UPDATE utilisateurs SET pin_hash = ?1 WHERE id = ?2", params![hacher(pin)?, m.id])?;
        }
        if let Some(actif) = m.actif {
            op.execute("UPDATE utilisateurs SET actif = ?1 WHERE id = ?2", params![actif, m.id])?;
        }
        if let Some(e) = &m.employe_id {
            op.execute("UPDATE utilisateurs SET employe_id = ?1 WHERE id = ?2", params![e, m.id])?;
        }
        op.execute(
            "UPDATE utilisateurs SET modifie_le = ?1, version = version + 1 WHERE id = ?2",
            params![op.maintenant, m.id],
        )?;
        op.audit(
            "utilisateur.modifier",
            "utilisateur",
            Some(&m.id),
            None,
            Some(json!({"nom": m.nom, "role": m.role_code, "actif": m.actif, "pin_change": m.pin.is_some()})),
            None,
            None,
        )?;
        Ok(())
    })
}

#[derive(Debug, Serialize)]
pub struct Session {
    pub jeton: String,
    pub utilisateur: UtilisateurResume,
    pub permissions: Vec<String>,
    pub plafond_remise_pct: i64,
    /// Session confirmée par mot de passe : administration accessible (RG-AUT-06).
    pub eleve: bool,
}

/// Connexion par PIN (RG-AUT-01/02). Hors garde d'horloge : un responsable doit
/// pouvoir se connecter pour corriger la date.
pub fn connexion_pin(db: &mut Db, utilisateur_id: &str, pin: &str, appareil_id: Option<&str>) -> Resultat<Session> {
    let r = db.executer_sans_garde(&Acteur::systeme(), |op| {
        let (hash, echecs, verrou): (String, i64, Option<i64>) = trouver(
            op.query_row(
                "SELECT pin_hash, echecs_pin, verrouille_jusqu_a FROM utilisateurs WHERE id = ?1 AND actif = 1",
                params![utilisateur_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            ),
            "Utilisateur",
        )?;
        if verrou.is_some_and(|v| v > op.maintenant) {
            return Ok(Err(Erreur::Verrouille));
        }
        if !verifier(pin, &hash) {
            let echecs = echecs + 1;
            let verrou = (echecs >= ECHECS_MAX).then_some(op.maintenant + VERROUILLAGE_MS);
            op.execute(
                "UPDATE utilisateurs SET echecs_pin = ?1, verrouille_jusqu_a = ?2 WHERE id = ?3",
                params![if verrou.is_some() { 0 } else { echecs }, verrou, utilisateur_id],
            )?;
            if verrou.is_some() {
                op.audit("utilisateur.verrouille", "utilisateur", Some(utilisateur_id), None, None, None, None)?;
                return Ok(Err(Erreur::Verrouille));
            }
            return Ok(Err(Erreur::PinIncorrect));
        }
        op.execute(
            "UPDATE utilisateurs SET echecs_pin = 0, verrouille_jusqu_a = NULL WHERE id = ?1",
            params![utilisateur_id],
        )?;
        let jeton = nouveau_jeton();
        op.execute(
            "INSERT INTO sessions(jeton_hash, utilisateur_id, appareil_id, cree_le, derniere_activite) VALUES (?1, ?2, ?3, ?4, ?4)",
            params![hash_jeton(&jeton), utilisateur_id, appareil_id, op.maintenant],
        )?;
        Ok(Ok(jeton))
    })?;
    let jeton = r?;
    session_de(db.conn(), &jeton, utilisateur_id, false)
}

fn session_de(conn: &Connection, jeton: &str, utilisateur_id: &str, eleve: bool) -> Resultat<Session> {
    let utilisateur = lister_utilisateurs(conn, false)?
        .into_iter()
        .find(|u| u.id == utilisateur_id)
        .ok_or_else(|| Erreur::NonTrouve("Utilisateur".into()))?;
    let (perms, plafond) = permissions_utilisateur(conn, utilisateur_id)?;
    let mut permissions: Vec<String> = perms.into_iter().collect();
    permissions.sort();
    Ok(Session { jeton: jeton.to_string(), utilisateur, permissions, plafond_remise_pct: plafond, eleve })
}

#[derive(Debug)]
pub struct InfoSession {
    pub utilisateur_id: String,
    pub appareil_id: Option<String>,
    pub eleve: bool,
}

/// Valide un jeton et renouvelle l'activité (RG-AUT-04).
pub fn verifier_session(db: &Db, jeton: &str) -> Resultat<InfoSession> {
    let conn = db.conn();
    let maintenant = db.maintenant();
    let delai = crate::parametres::lire(conn)?.verrouillage_minutes.max(1) * 60_000;
    let h = hash_jeton(jeton);
    let (uid, appareil, activite, eleve): (String, Option<String>, i64, bool) = conn
        .query_row(
            "SELECT s.utilisateur_id, s.appareil_id, s.derniere_activite, s.eleve FROM sessions s
             JOIN utilisateurs u ON u.id = s.utilisateur_id
             WHERE s.jeton_hash = ?1 AND s.fermee = 0 AND u.actif = 1",
            params![h],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .optional()?
        .ok_or(Erreur::NonAuthentifie)?;
    // Une horloge qui recule ne doit pas bloquer la session : on ne compare que vers l'avant.
    if maintenant - activite > delai {
        conn.execute("UPDATE sessions SET fermee = 1 WHERE jeton_hash = ?1", params![h])?;
        return Err(Erreur::NonAuthentifie);
    }
    conn.execute("UPDATE sessions SET derniere_activite = ?1 WHERE jeton_hash = ?2", params![maintenant.max(activite), h])?;
    Ok(InfoSession { utilisateur_id: uid, appareil_id: appareil, eleve })
}

pub fn session_courante(db: &Db, jeton: &str) -> Resultat<Session> {
    let s = verifier_session(db, jeton)?;
    session_de(db.conn(), jeton, &s.utilisateur_id, s.eleve)
}

/// RG-AUT-06 : confirme la session par le mot de passe de l'utilisateur connecté.
/// Les échecs comptent avec ceux du PIN (verrouillage RG-AUT-02).
pub fn elever_session(db: &mut Db, jeton: &str, mot_de_passe: &str) -> Resultat<Session> {
    let s = verifier_session(db, jeton)?;
    let uid = s.utilisateur_id.clone();
    let r = db.executer_sans_garde(&Acteur::systeme(), |op| {
        let (hash, echecs, verrou): (Option<String>, i64, Option<i64>) = trouver(
            op.query_row(
                "SELECT mot_de_passe_hash, echecs_pin, verrouille_jusqu_a FROM utilisateurs WHERE id = ?1",
                params![uid],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            ),
            "Utilisateur",
        )?;
        if verrou.is_some_and(|v| v > op.maintenant) {
            return Ok(Err(Erreur::Verrouille));
        }
        let Some(hash) = hash else {
            return Ok(Err(Erreur::regle("RG-AUT-06", "Définissez d'abord votre mot de passe")));
        };
        if !verifier(mot_de_passe, &hash) {
            let echecs = echecs + 1;
            let verrou = (echecs >= ECHECS_MAX).then_some(op.maintenant + VERROUILLAGE_MS);
            op.execute(
                "UPDATE utilisateurs SET echecs_pin = ?1, verrouille_jusqu_a = ?2 WHERE id = ?3",
                params![if verrou.is_some() { 0 } else { echecs }, verrou, uid],
            )?;
            return Ok(Err(if verrou.is_some() { Erreur::Verrouille } else { Erreur::validation("Mot de passe incorrect") }));
        }
        op.execute("UPDATE utilisateurs SET echecs_pin = 0, verrouille_jusqu_a = NULL WHERE id = ?1", params![uid])?;
        op.execute("UPDATE sessions SET eleve = 1 WHERE jeton_hash = ?1", params![hash_jeton(jeton)])?;
        Ok(Ok(()))
    })?;
    r?;
    session_de(db.conn(), jeton, &s.utilisateur_id, true)
}

/// Définit ou change un mot de passe. Pour soi : l'ancien est exigé s'il existe.
/// Pour un autre utilisateur : administration (session confirmée).
pub fn definir_mot_de_passe(db: &mut Db, acteur: &Acteur, cible: &str, ancien: Option<&str>, nouveau: &str) -> Resultat<()> {
    valider_mot_de_passe(nouveau)?;
    db.executer(acteur, |op| {
        let soi = op.utilisateur() == Some(cible);
        if soi {
            let actuel: Option<String> = trouver(
                op.query_row("SELECT mot_de_passe_hash FROM utilisateurs WHERE id = ?1", params![cible], |r| r.get(0)),
                "Utilisateur",
            )?;
            if let Some(h) = actuel {
                if !ancien.is_some_and(|a| verifier(a, &h)) {
                    return Err(Erreur::validation("Ancien mot de passe incorrect"));
                }
            }
        } else {
            op.exiger(perm::UTILISATEUR_GERER)?;
        }
        let n = op.execute(
            "UPDATE utilisateurs SET mot_de_passe_hash = ?1, modifie_le = ?2 WHERE id = ?3",
            params![hacher(nouveau)?, op.maintenant, cible],
        )?;
        if n == 0 {
            return Err(Erreur::NonTrouve("Utilisateur".into()));
        }
        op.audit("utilisateur.mot_de_passe", "utilisateur", Some(cible), None, None, None, None)?;
        Ok(())
    })
}

pub fn deconnexion(db: &Db, jeton: &str) -> Resultat<()> {
    db.conn().execute("UPDATE sessions SET fermee = 1 WHERE jeton_hash = ?1", params![hash_jeton(jeton)])?;
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct Role {
    pub id: String,
    pub code: String,
    pub nom: String,
    pub plafond_remise_pct: i64,
    pub permissions: Vec<String>,
}

pub fn lister_roles(conn: &Connection) -> Resultat<Vec<Role>> {
    let mut stmt = conn.prepare("SELECT id, code, nom, plafond_remise_pct FROM roles ORDER BY nom")?;
    let roles = stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?
        .collect::<Result<Vec<(String, String, String, i64)>, _>>()?;
    let mut v = Vec::new();
    for (id, code, nom, plafond) in roles {
        let mut s = conn.prepare_cached("SELECT permission FROM role_permissions WHERE role_id = ?1 ORDER BY permission")?;
        let permissions = s.query_map(params![id], |r| r.get(0))?.collect::<Result<Vec<String>, _>>()?;
        v.push(Role { id, code, nom, plafond_remise_pct: plafond, permissions });
    }
    Ok(v)
}

#[derive(Debug, Deserialize)]
pub struct ModifRole {
    pub code: String,
    pub plafond_remise_pct: i64,
    pub permissions: Vec<String>,
}

pub fn modifier_role(db: &mut Db, acteur: &Acteur, m: &ModifRole) -> Resultat<()> {
    db.executer(acteur, |op| {
        op.exiger(perm::UTILISATEUR_GERER)?;
        if m.code == "proprietaire" {
            return Err(Erreur::regle("RG-AUT-05", "Le rôle propriétaire garde toutes les permissions"));
        }
        if !(0..=100).contains(&m.plafond_remise_pct) {
            return Err(Erreur::validation("Plafond de remise entre 0 et 100 %"));
        }
        for p in &m.permissions {
            if !perm::TOUTES.contains(&p.as_str()) {
                return Err(Erreur::validation(format!("Permission inconnue : {p}")));
            }
        }
        let rid = role_id(op, &m.code)?;
        let avant: Vec<String> = op
            .prepare("SELECT permission FROM role_permissions WHERE role_id = ?1")?
            .query_map(params![rid], |r| r.get(0))?
            .collect::<Result<_, _>>()?;
        op.execute("DELETE FROM role_permissions WHERE role_id = ?1", params![rid])?;
        for p in &m.permissions {
            op.execute("INSERT OR IGNORE INTO role_permissions(role_id, permission) VALUES (?1, ?2)", params![rid, p])?;
        }
        op.execute(
            "UPDATE roles SET plafond_remise_pct = ?1, modifie_le = ?2 WHERE id = ?3",
            params![m.plafond_remise_pct, op.maintenant, rid],
        )?;
        op.audit(
            "role.modifier",
            "role",
            Some(&rid),
            Some(json!({ "permissions": avant })),
            Some(json!({ "permissions": m.permissions, "plafond": m.plafond_remise_pct })),
            None,
            None,
        )?;
        Ok(())
    })
}

/// RG-SYS-01 : un responsable confirme que l'heure actuelle du PC est la bonne.
pub fn accepter_heure(db: &mut Db, acteur: &Acteur) -> Resultat<()> {
    let dernier = crate::db::dernier_horodatage(db.conn())?;
    db.executer_sans_garde(acteur, |op| {
        let autorise_par = op.exiger(perm::HORLOGE_FORCER)?;
        op.audit(
            "horloge.accepter",
            "systeme",
            None,
            Some(json!({ "dernier_evenement": dernier })),
            Some(json!({ "heure_pc": op.maintenant })),
            Some("Changement de date détecté"),
            autorise_par.as_deref(),
        )?;
        op.execute(
            "INSERT INTO systeme(cle, valeur) VALUES ('dernier_horodatage', ?1)
             ON CONFLICT(cle) DO UPDATE SET valeur = excluded.valeur",
            params![op.maintenant.to_string()],
        )?;
        Ok(())
    })?;
    // executer_sans_garde réécrit max(maintenant, dernier) : on force la nouvelle valeur.
    db.conn().execute(
        "UPDATE systeme SET valeur = ?1 WHERE cle = 'dernier_horodatage'",
        params![db.maintenant().to_string()],
    )?;
    Ok(())
}
