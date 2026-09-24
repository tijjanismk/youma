use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::Serialize;
use serde_json::json;

use crate::db::{Acteur, Db};
use crate::erreur::{Erreur, Resultat};
use crate::horloge::date_exploitation;
use crate::permissions as perm;

#[derive(Debug, Clone, Serialize)]
pub struct Journee {
    pub id: String,
    pub date_exploitation: String,
    pub statut: String,
    pub ouverte_le: i64,
    pub cloturee_le: Option<i64>,
}

fn depuis(r: &Row) -> rusqlite::Result<Journee> {
    Ok(Journee {
        id: r.get(0)?,
        date_exploitation: r.get(1)?,
        statut: r.get(2)?,
        ouverte_le: r.get(3)?,
        cloturee_le: r.get(4)?,
    })
}

const COLS: &str = "id, date_exploitation, statut, ouverte_le, cloturee_le";

pub fn ouverte(conn: &Connection) -> Resultat<Option<Journee>> {
    Ok(conn
        .query_row(&format!("SELECT {COLS} FROM journees WHERE statut = 'ouverte'"), [], depuis)
        .optional()?)
}

pub fn lister(conn: &Connection, limite: i64) -> Resultat<Vec<Journee>> {
    let mut s = conn.prepare(&format!("SELECT {COLS} FROM journees ORDER BY date_exploitation DESC LIMIT ?1"))?;
    let v = s.query_map(params![limite], depuis)?.collect::<Result<Vec<_>, _>>()?;
    Ok(v)
}

pub fn par_id(conn: &Connection, id: &str) -> Resultat<Journee> {
    crate::db::trouver(
        conn.query_row(&format!("SELECT {COLS} FROM journees WHERE id = ?1"), params![id], depuis),
        "Journée",
    )
}

pub fn par_date(conn: &Connection, date: &str) -> Resultat<Option<Journee>> {
    Ok(conn
        .query_row(&format!("SELECT {COLS} FROM journees WHERE date_exploitation = ?1"), params![date], depuis)
        .optional()?)
}

/// RG-JOU-02.
pub fn ouvrir(db: &mut Db, acteur: &Acteur) -> Resultat<Journee> {
    let j = db.executer(acteur, |op| {
        op.exiger(perm::JOURNEE_GERER)?;
        if ouverte(op)?.is_some() {
            return Err(Erreur::regle("RG-JOU-02", "Une journée est déjà ouverte"));
        }
        let date = date_exploitation(op.maintenant, op.params.fuseau_minutes, op.params.heure_bascule);
        let derniere: Option<(String, String)> = op
            .query_row(
                "SELECT id, date_exploitation FROM journees ORDER BY date_exploitation DESC LIMIT 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        if let Some((jid, d)) = derniere {
            if date == d {
                // Clients arrivés après la clôture : on rouvre la même journée (audité).
                op.execute(
                    "UPDATE journees SET statut = 'ouverte', cloturee_le = NULL, cloturee_par = NULL WHERE id = ?1",
                    params![jid],
                )?;
                op.audit("journee.rouvrir", "journee", Some(&jid), None, Some(json!({ "date": d })), None, None)?;
                op.evenement("journee", Some(&jid));
                return Ok(jid);
            }
            if date < d {
                return Err(Erreur::regle(
                    "RG-JOU-02",
                    format!("La date du PC ({date}) est antérieure à la dernière journée ({d}). Vérifiez la date."),
                ));
            }
        }
        let id = op.nouvel_id();
        op.execute(
            "INSERT INTO journees(id, date_exploitation, statut, ouverte_le, ouverte_par) VALUES (?1, ?2, 'ouverte', ?3, ?4)",
            params![id, date, op.maintenant, op.utilisateur()],
        )?;
        op.audit("journee.ouvrir", "journee", Some(&id), None, Some(json!({ "date": date })), None, None)?;
        op.outbox("journee", &id, "creer")?;
        op.evenement("journee", Some(&id));
        Ok(id)
    })?;
    par_id(db.conn(), &j)
}

/// RG-JOU-04.
pub fn cloturer(db: &mut Db, acteur: &Acteur) -> Resultat<Journee> {
    let id = db.executer(acteur, |op| {
        op.exiger(perm::JOURNEE_GERER)?;
        let j = op.journee_ouverte()?;
        let commandes: i64 = op.query_row(
            "SELECT COUNT(*) FROM commandes WHERE journee_id = ?1 AND statut = 'ouverte'",
            params![j.id],
            |r| r.get(0),
        )?;
        if commandes > 0 {
            return Err(Erreur::regle(
                "RG-JOU-04",
                format!("{commandes} addition(s) encore ouverte(s). Encaissez-les ou transférez-les."),
            ));
        }
        let sessions: i64 = op.query_row(
            "SELECT COUNT(*) FROM sessions_caisse WHERE statut = 'ouverte'",
            [],
            |r| r.get(0),
        )?;
        if sessions > 0 {
            return Err(Erreur::regle("RG-JOU-04", "Clôturez d'abord les sessions de caisse ouvertes"));
        }
        op.execute(
            "UPDATE journees SET statut = 'cloturee', cloturee_le = ?1, cloturee_par = ?2 WHERE id = ?3",
            params![op.maintenant, op.utilisateur(), j.id],
        )?;
        // Les additions payées de la journée sont clôturées avec elle.
        op.execute(
            "UPDATE commandes SET statut = 'cloturee', cloturee_le = ?1 WHERE journee_id = ?2 AND statut = 'payee'",
            params![op.maintenant, j.id],
        )?;
        op.audit("journee.cloturer", "journee", Some(&j.id), None, None, None, None)?;
        op.outbox("journee", &j.id, "cloturer")?;
        op.evenement("journee", Some(&j.id));
        Ok(j.id)
    })?;
    par_id(db.conn(), &id)
}
