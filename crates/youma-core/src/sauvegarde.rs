//! Sauvegardes (`VACUUM INTO` = copie cohérente même pendant le service), rotation,
//! contrôle d'intégrité et restauration guidée (cahier §20).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Datelike, Utc};
use rusqlite::{params, Connection, OpenFlags};
use serde::Serialize;
use serde_json::json;

use crate::db::{version_schema, Acteur, Db};
use crate::erreur::{Erreur, Resultat};
use crate::permissions as perm;

const PREFIXE: &str = "youma-";

#[derive(Debug, Serialize, Clone)]
pub struct Sauvegarde {
    pub chemin: String,
    pub taille: u64,
    pub horodatage: i64,
    pub motif: String,
}

fn nom_fichier(ms: i64, motif: &str) -> String {
    let d = DateTime::<Utc>::from_timestamp_millis(ms).unwrap_or_default();
    let motif: String = motif.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect();
    format!("{PREFIXE}{}-{motif}.db", d.format("%Y%m%d-%H%M%S"))
}

/// Sauvegarde dans `dossier` (et copie vers le dossier externe si configuré).
pub fn sauvegarder(db: &mut Db, dossier: &Path, motif: &str) -> Resultat<Sauvegarde> {
    std::fs::create_dir_all(dossier)?;
    let maintenant = db.maintenant();
    let chemin = dossier.join(nom_fichier(maintenant, motif));
    let _ = std::fs::remove_file(&chemin);
    let r = db.conn().execute("VACUUM INTO ?1", params![chemin.to_string_lossy()]);
    let taille = std::fs::metadata(&chemin).map(|m| m.len()).unwrap_or(0);
    enregistrer(db.conn(), maintenant, &chemin, motif, taille, false, r.as_ref().err().map(|e| e.to_string()))?;
    r?;
    let p = crate::parametres::lire(db.conn())?;
    if !p.dossier_sauvegarde_externe.trim().is_empty() {
        let ext = PathBuf::from(p.dossier_sauvegarde_externe.trim());
        let dest = ext.join(chemin.file_name().unwrap_or_default());
        let res = std::fs::create_dir_all(&ext).and_then(|_| std::fs::copy(&chemin, &dest));
        enregistrer(db.conn(), maintenant, &dest, motif, taille, true, res.err().map(|e| e.to_string()))?;
    }
    Ok(Sauvegarde { chemin: chemin.to_string_lossy().into(), taille, horodatage: maintenant, motif: motif.into() })
}

fn enregistrer(conn: &Connection, ms: i64, chemin: &Path, motif: &str, taille: u64, externe: bool, erreur: Option<String>) -> Resultat<()> {
    conn.execute(
        "INSERT INTO sauvegardes(id, horodatage, chemin, motif, taille, externe, reussie, erreur) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![uuid::Uuid::now_v7().to_string(), ms, chemin.to_string_lossy(), motif, taille as i64, externe, erreur.is_none(), erreur],
    )?;
    Ok(())
}

/// Copie vers une clé USB en un clic.
pub fn exporter(db: &mut Db, acteur: &Acteur, dossier: &Path) -> Resultat<Sauvegarde> {
    db.executer(acteur, |op| {
        op.exiger(perm::SAUVEGARDE_GERER)?;
        op.audit("sauvegarde.exporter", "systeme", None, None, Some(json!({ "dossier": dossier.to_string_lossy() })), None, None)?;
        Ok(())
    })?;
    let s = sauvegarder(db, dossier, "export")?;
    db.conn().execute("UPDATE sauvegardes SET externe = 1 WHERE chemin = ?1", params![s.chemin])?;
    Ok(s)
}

pub fn lister(dossier: &Path) -> Resultat<Vec<Sauvegarde>> {
    let mut v = Vec::new();
    let Ok(entrees) = std::fs::read_dir(dossier) else { return Ok(v) };
    for e in entrees.flatten() {
        let nom = e.file_name().to_string_lossy().to_string();
        if !(nom.starts_with(PREFIXE) && nom.ends_with(".db")) {
            continue;
        }
        let meta = e.metadata()?;
        let horodatage = horodatage_nom(&nom).unwrap_or(0);
        let motif = nom.trim_end_matches(".db").splitn(4, '-').nth(3).unwrap_or("").to_string();
        v.push(Sauvegarde { chemin: e.path().to_string_lossy().into(), taille: meta.len(), horodatage, motif });
    }
    v.sort_by(|a, b| b.horodatage.cmp(&a.horodatage));
    Ok(v)
}

fn horodatage_nom(nom: &str) -> Option<i64> {
    let s = nom.strip_prefix(PREFIXE)?;
    let d = s.get(0..15)?;
    chrono::NaiveDateTime::parse_from_str(d, "%Y%m%d-%H%M%S").ok().map(|d| d.and_utc().timestamp_millis())
}

/// Rotation : 7 journalières, 4 hebdomadaires, 12 mensuelles (les plus récentes de chaque période).
/// Les sauvegardes « avant-… » (mise à jour, restauration) et les exports ne sont jamais supprimés.
pub fn rotation(dossier: &Path) -> Resultat<Vec<String>> {
    let toutes: Vec<Sauvegarde> = lister(dossier)?
        .into_iter()
        .filter(|s| !s.motif.starts_with("avant") && s.motif != "export")
        .collect();
    let mut garder: BTreeSet<String> = BTreeSet::new();
    let mut jours = BTreeSet::new();
    let mut semaines = BTreeSet::new();
    let mut mois = BTreeSet::new();
    for s in &toutes {
        let d = DateTime::<Utc>::from_timestamp_millis(s.horodatage).unwrap_or_default();
        let jour = d.format("%Y-%m-%d").to_string();
        let semaine = format!("{}-{}", d.iso_week().year(), d.iso_week().week());
        let m = d.format("%Y-%m").to_string();
        if jours.len() < 7 && jours.insert(jour) {
            garder.insert(s.chemin.clone());
        }
        if semaines.len() < 4 && semaines.insert(semaine) {
            garder.insert(s.chemin.clone());
        }
        if mois.len() < 12 && mois.insert(m) {
            garder.insert(s.chemin.clone());
        }
    }
    let mut supprimees = Vec::new();
    for s in toutes {
        if !garder.contains(&s.chemin) && std::fs::remove_file(&s.chemin).is_ok() {
            supprimees.push(s.chemin);
        }
    }
    Ok(supprimees)
}

#[derive(Debug, Serialize, Clone)]
pub struct RapportIntegrite {
    pub complet: bool,
    pub ok: bool,
    pub messages: Vec<String>,
    pub horodatage: i64,
}

/// Contrôle rapide (démarrage) ou complet (périodique).
pub fn verifier_integrite(conn: &Connection, complet: bool, maintenant: i64) -> Resultat<RapportIntegrite> {
    let pragma = if complet { "integrity_check" } else { "quick_check" };
    let mut s = conn.prepare(&format!("PRAGMA {pragma}"))?;
    let mut messages: Vec<String> = s.query_map([], |r| r.get(0))?.collect::<Result<_, _>>()?;
    let ok_base = messages.len() == 1 && messages[0] == "ok";
    let mut fk = conn.prepare("PRAGMA foreign_key_check")?;
    let violations: Vec<String> = fk
        .query_map([], |r| Ok(format!("Clé étrangère invalide dans {}", r.get::<_, String>(0)?)))?
        .collect::<Result<_, _>>()?;
    let ok = ok_base && violations.is_empty();
    messages.extend(violations);
    if ok {
        messages = vec!["Base de données saine".into()];
    }
    let r = RapportIntegrite { complet, ok, messages, horodatage: maintenant };
    conn.execute(
        "INSERT INTO systeme(cle, valeur) VALUES ('dernier_controle_integrite', ?1)
         ON CONFLICT(cle) DO UPDATE SET valeur = excluded.valeur",
        params![serde_json::to_string(&r)?],
    )?;
    Ok(r)
}

/// Vérifie qu'un fichier est une sauvegarde Youma saine et compatible.
pub fn verifier_fichier(chemin: &Path) -> Resultat<i64> {
    let c = Connection::open_with_flags(chemin, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|_| Erreur::validation("Fichier illisible"))?;
    let version: i64 = c.pragma_query_value(None, "user_version", |r| r.get(0))?;
    if version == 0 || version > version_schema() {
        return Err(Erreur::validation("Ce fichier n'est pas une sauvegarde compatible avec cette version"));
    }
    let integ: String = c.query_row("PRAGMA quick_check", [], |r| r.get(0))?;
    if integ != "ok" {
        return Err(Erreur::validation("La sauvegarde est endommagée"));
    }
    c.query_row("SELECT valeur FROM systeme WHERE cle = 'installation_id'", [], |r| r.get::<_, String>(0))
        .map_err(|_| Erreur::validation("Ce fichier n'est pas une base Youma"))?;
    Ok(version)
}

/// Restauration : sauvegarde de l'état actuel, remplacement du fichier, réouverture.
pub fn restaurer(db: &mut Db, acteur: &Acteur, fichier: &Path, dossier_sauvegardes: &Path) -> Resultat<()> {
    db.executer(acteur, |op| {
        op.exiger(perm::SAUVEGARDE_GERER)?;
        Ok(())
    })?;
    verifier_fichier(fichier)?;
    let cible = db.chemin().map(Path::to_path_buf).ok_or_else(|| Erreur::validation("Base en mémoire"))?;
    sauvegarder(db, dossier_sauvegardes, "avant-restauration")?;
    let temp = cible.with_extension("restauration.tmp");
    std::fs::copy(fichier, &temp)?;
    db.fermer_pour_restauration()?;
    for ext in ["db-wal", "db-shm"] {
        let _ = std::fs::remove_file(cible.with_extension(ext));
    }
    std::fs::rename(&temp, &cible)?;
    let conn = Connection::open(&cible)?;
    db.remplacer_connexion(conn)?;
    // L'horloge de la sauvegarde peut être plus ancienne : le repère repart de maintenant.
    let maintenant = db.maintenant();
    db.conn().execute(
        "INSERT INTO systeme(cle, valeur) VALUES ('dernier_horodatage', ?1) ON CONFLICT(cle) DO UPDATE SET valeur = excluded.valeur",
        params![maintenant.max(crate::db::dernier_horodatage(db.conn())?).to_string()],
    )?;
    db.executer(acteur, |op| {
        op.audit("sauvegarde.restaurer", "systeme", None, None, Some(json!({ "fichier": fichier.to_string_lossy() })), None, None)
    })
    .or_else(|_| db.executer(&Acteur::systeme(), |op| op.audit("sauvegarde.restaurer", "systeme", None, None, Some(json!({ "fichier": fichier.to_string_lossy() })), None, None)))?;
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct EtatSauvegardes {
    pub derniere: Option<i64>,
    pub derniere_externe: Option<i64>,
    pub alerte_externe: bool,
    pub espace_libre: Option<u64>,
    pub alerte_disque: bool,
    pub dernier_controle: Option<RapportIntegrite>,
}

pub fn etat(conn: &Connection, dossier: &Path, maintenant: i64) -> Resultat<EtatSauvegardes> {
    let derniere: Option<i64> = conn.query_row("SELECT MAX(horodatage) FROM sauvegardes WHERE reussie = 1", [], |r| r.get(0))?;
    let derniere_externe: Option<i64> =
        conn.query_row("SELECT MAX(horodatage) FROM sauvegardes WHERE reussie = 1 AND externe = 1", [], |r| r.get(0))?;
    let p = crate::parametres::lire(conn)?;
    let seuil = p.alerte_sauvegarde_jours.max(1) * 86_400_000;
    let alerte_externe = derniere_externe.is_none_or(|d| maintenant - d > seuil);
    let espace_libre = fs2::available_space(dossier).ok().or_else(|| fs2::available_space(".").ok());
    let dernier_controle = conn
        .query_row("SELECT valeur FROM systeme WHERE cle = 'dernier_controle_integrite'", [], |r| r.get::<_, String>(0))
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| {
            Some(RapportIntegrite {
                complet: v.get("complet")?.as_bool()?,
                ok: v.get("ok")?.as_bool()?,
                messages: v.get("messages")?.as_array()?.iter().filter_map(|m| m.as_str().map(str::to_owned)).collect(),
                horodatage: v.get("horodatage")?.as_i64()?,
            })
        });
    Ok(EtatSauvegardes {
        derniere,
        derniere_externe,
        alerte_externe,
        alerte_disque: espace_libre.is_some_and(|e| e < 1_000_000_000),
        espace_libre,
        dernier_controle,
    })
}
