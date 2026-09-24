//! Appareils autorisés en mode réseau (téléphones des serveurs, tablette cuisine).
//! Appairage par code à 6 chiffres affiché sur le poste central (valable 10 minutes).

use rand::Rng;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use serde_json::json;

use crate::auth::{hash_jeton, nouveau_jeton};
use crate::db::{Acteur, Db};
use crate::erreur::{Erreur, Resultat};
use crate::permissions as perm;

const VALIDITE_MS: i64 = 10 * 60_000;

#[derive(Debug, Serialize)]
pub struct CodeAppairage {
    pub code: String,
    pub expire_le: i64,
}

pub fn generer_code(db: &mut Db, acteur: &Acteur) -> Resultat<CodeAppairage> {
    db.executer(acteur, |op| {
        op.exiger(perm::APPAREIL_GERER)?;
        let code = format!("{:06}", rand::thread_rng().gen_range(0..1_000_000));
        let expire_le = op.maintenant + VALIDITE_MS;
        op.execute(
            "INSERT INTO systeme(cle, valeur) VALUES ('appairage', ?1) ON CONFLICT(cle) DO UPDATE SET valeur = excluded.valeur",
            params![json!({ "hash": hash_jeton(&code), "expire": expire_le }).to_string()],
        )?;
        Ok(CodeAppairage { code, expire_le })
    })
}

/// Échange un code valide contre un jeton d'appareil (conservé par le navigateur).
pub fn appairer(db: &mut Db, code: &str, nom: &str, type_: &str) -> Resultat<(String, String)> {
    db.executer(&Acteur::systeme(), |op| {
        let v: Option<String> = op.query_row("SELECT valeur FROM systeme WHERE cle = 'appairage'", [], |r| r.get(0)).optional()?;
        let v: serde_json::Value = v.and_then(|s| serde_json::from_str(&s).ok()).ok_or_else(|| Erreur::validation("Aucun code d'appairage actif"))?;
        let expire = v.get("expire").and_then(|e| e.as_i64()).unwrap_or(0);
        if v.get("hash").and_then(|h| h.as_str()) != Some(hash_jeton(code.trim()).as_str()) || expire < op.maintenant {
            return Err(Erreur::validation("Code incorrect ou expiré"));
        }
        let jeton = nouveau_jeton();
        let id = op.nouvel_id();
        let nom = if nom.trim().is_empty() { "Appareil" } else { nom.trim() };
        op.execute(
            "INSERT INTO appareils(id, nom, type, jeton_hash, cree_le) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, nom, type_, hash_jeton(&jeton), op.maintenant],
        )?;
        op.execute("DELETE FROM systeme WHERE cle = 'appairage'", [])?;
        op.audit("appareil.appairer", "appareil", Some(&id), None, Some(json!({ "nom": nom })), None, None)?;
        Ok((id, jeton))
    })
}

/// Renvoie l'identifiant de l'appareil si le jeton est valide et non révoqué.
pub fn verifier(conn: &Connection, jeton: &str, maintenant: i64) -> Resultat<Option<String>> {
    let h = hash_jeton(jeton);
    let id: Option<String> =
        conn.query_row("SELECT id FROM appareils WHERE jeton_hash = ?1 AND actif = 1", params![h], |r| r.get(0)).optional()?;
    if let Some(id) = &id {
        conn.execute("UPDATE appareils SET derniere_vue = ?1 WHERE id = ?2", params![maintenant, id])?;
    }
    Ok(id)
}

#[derive(Debug, Serialize)]
pub struct Appareil {
    pub id: String,
    pub nom: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub actif: bool,
    pub cree_le: i64,
    pub derniere_vue: Option<i64>,
}

pub fn lister(conn: &Connection) -> Resultat<Vec<Appareil>> {
    let mut s = conn.prepare("SELECT id, nom, type, actif, cree_le, derniere_vue FROM appareils ORDER BY actif DESC, nom")?;
    let v = s
        .query_map([], |r| {
            Ok(Appareil { id: r.get(0)?, nom: r.get(1)?, type_: r.get(2)?, actif: r.get(3)?, cree_le: r.get(4)?, derniere_vue: r.get(5)? })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(v)
}

pub fn revoquer(db: &mut Db, acteur: &Acteur, id: &str) -> Resultat<()> {
    db.executer(acteur, |op| {
        op.exiger(perm::APPAREIL_GERER)?;
        op.execute("UPDATE appareils SET actif = 0 WHERE id = ?1", params![id])?;
        op.execute("UPDATE sessions SET fermee = 1 WHERE appareil_id = ?1", params![id])?;
        op.audit("appareil.revoquer", "appareil", Some(id), None, None, None, None)?;
        Ok(())
    })
}
