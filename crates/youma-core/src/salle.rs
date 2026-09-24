use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::catalogue::non_vide;
use crate::db::{Acteur, Db};
use crate::erreur::{Erreur, Resultat};
use crate::permissions as perm;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Zone {
    #[serde(default)]
    pub id: String,
    pub nom: String,
    #[serde(default)]
    pub ordre: i64,
    #[serde(default = "vrai")]
    pub actif: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Table {
    #[serde(default)]
    pub id: String,
    pub zone_id: String,
    pub nom: String,
    #[serde(default = "quatre")]
    pub capacite: i64,
    #[serde(default = "vrai")]
    pub actif: bool,
}

fn vrai() -> bool {
    true
}
fn quatre() -> i64 {
    4
}

pub fn lister_zones(conn: &Connection) -> Resultat<Vec<Zone>> {
    let mut s = conn.prepare("SELECT id, nom, ordre, actif FROM zones ORDER BY ordre, nom")?;
    let v = s
        .query_map([], |r| Ok(Zone { id: r.get(0)?, nom: r.get(1)?, ordre: r.get(2)?, actif: r.get(3)? }))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(v)
}

pub fn enregistrer_zone(db: &mut Db, acteur: &Acteur, z: &Zone) -> Resultat<String> {
    db.executer(acteur, |op| {
        op.exiger(perm::SALLE_GERER)?;
        non_vide(&z.nom, "nom de la zone")?;
        let id = if z.id.is_empty() { op.nouvel_id() } else { z.id.clone() };
        op.execute(
            "INSERT INTO zones(id, nom, ordre, actif, modifie_le) VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(id) DO UPDATE SET nom = excluded.nom, ordre = excluded.ordre, actif = excluded.actif,
               modifie_le = excluded.modifie_le",
            params![id, z.nom.trim(), z.ordre, z.actif, op.maintenant],
        )?;
        op.audit("zone.enregistrer", "zone", Some(&id), None, Some(json!(z)), None, None)?;
        Ok(id)
    })
}

pub fn enregistrer_table(db: &mut Db, acteur: &Acteur, t: &Table) -> Resultat<String> {
    db.executer(acteur, |op| {
        op.exiger(perm::SALLE_GERER)?;
        non_vide(&t.nom, "nom de la table")?;
        let id = if t.id.is_empty() { op.nouvel_id() } else { t.id.clone() };
        op.execute(
            "INSERT INTO tables_salle(id, zone_id, nom, capacite, actif, modifie_le) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(id) DO UPDATE SET zone_id = excluded.zone_id, nom = excluded.nom,
               capacite = excluded.capacite, actif = excluded.actif, modifie_le = excluded.modifie_le",
            params![id, t.zone_id, t.nom.trim(), t.capacite, t.actif, op.maintenant],
        )?;
        op.audit("table.enregistrer", "table", Some(&id), None, Some(json!(t)), None, None)?;
        Ok(id)
    })
}

/// Crée `nombre` tables numérotées d'un coup (assistant de configuration).
pub fn creer_tables_serie(db: &mut Db, acteur: &Acteur, zone_id: &str, prefixe: &str, debut: i64, nombre: i64) -> Resultat<usize> {
    db.executer(acteur, |op| {
        op.exiger(perm::SALLE_GERER)?;
        if !(1..=200).contains(&nombre) {
            return Err(Erreur::validation("Entre 1 et 200 tables"));
        }
        for i in debut..debut + nombre {
            op.execute(
                "INSERT INTO tables_salle(id, zone_id, nom, modifie_le) VALUES (?1, ?2, ?3, ?4)",
                params![op.nouvel_id(), zone_id, format!("{prefixe}{i}").trim(), op.maintenant],
            )?;
        }
        Ok(nombre as usize)
    })
}

#[derive(Debug, Serialize)]
pub struct TablePlan {
    pub id: String,
    pub zone_id: String,
    pub nom: String,
    pub capacite: i64,
    /// libre | occupee | reservee | a_nettoyer
    pub statut: String,
    pub commande_id: Option<String>,
    pub commande_numero: Option<i64>,
    pub serveur: Option<String>,
    pub ouverte_le: Option<i64>,
    pub a_envoyer: i64,
    pub pretes: i64,
}

/// Plan de salle avec statut dérivé des additions ouvertes.
pub fn plan(conn: &Connection) -> Resultat<Vec<TablePlan>> {
    let mut s = conn.prepare(
        "SELECT t.id, t.zone_id, t.nom, t.capacite, t.a_nettoyer, t.reservee, c.id, c.numero, u.nom, c.cree_le,
            (SELECT COUNT(*) FROM lignes_commande l WHERE l.commande_id = c.id AND l.statut = 'brouillon'),
            (SELECT COUNT(*) FROM envois e WHERE e.commande_id = c.id AND e.statut = 'pret')
         FROM tables_salle t
         JOIN zones z ON z.id = t.zone_id
         LEFT JOIN commandes c ON c.table_id = t.id AND c.statut = 'ouverte'
         LEFT JOIN utilisateurs u ON u.id = c.serveur_id
         WHERE t.actif = 1 AND z.actif = 1
         ORDER BY z.ordre, z.nom, length(t.nom), t.nom",
    )?;
    let v = s
        .query_map([], |r| {
            let commande_id: Option<String> = r.get(6)?;
            let a_nettoyer: bool = r.get(4)?;
            let reservee: bool = r.get(5)?;
            let statut = if commande_id.is_some() {
                "occupee"
            } else if a_nettoyer {
                "a_nettoyer"
            } else if reservee {
                "reservee"
            } else {
                "libre"
            };
            Ok(TablePlan {
                id: r.get(0)?,
                zone_id: r.get(1)?,
                nom: r.get(2)?,
                capacite: r.get(3)?,
                statut: statut.into(),
                commande_id,
                commande_numero: r.get(7)?,
                serveur: r.get(8)?,
                ouverte_le: r.get(9)?,
                a_envoyer: r.get(10)?,
                pretes: r.get(11)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(v)
}

/// Marque réservée / à nettoyer (n'affecte pas une table occupée).
pub fn marquer_table(db: &mut Db, acteur: &Acteur, table_id: &str, reservee: Option<bool>, a_nettoyer: Option<bool>) -> Resultat<()> {
    db.executer(acteur, |op| {
        op.exiger(perm::COMMANDE_CREER)?;
        if let Some(r) = reservee {
            op.execute("UPDATE tables_salle SET reservee = ?1, modifie_le = ?2 WHERE id = ?3", params![r, op.maintenant, table_id])?;
        }
        if let Some(n) = a_nettoyer {
            op.execute("UPDATE tables_salle SET a_nettoyer = ?1, modifie_le = ?2 WHERE id = ?3", params![n, op.maintenant, table_id])?;
        }
        op.evenement("table", Some(table_id));
        Ok(())
    })
}

pub fn zone_de_table(conn: &Connection, table_id: &str) -> Resultat<String> {
    crate::db::trouver(
        conn.query_row("SELECT zone_id FROM tables_salle WHERE id = ?1 AND actif = 1", params![table_id], |r| r.get(0)),
        "Table",
    )
}
