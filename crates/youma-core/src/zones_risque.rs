//! Zones à risque (fiche 0013) : une livraison dans tel quartier (ou tel cercle GPS), à telle heure,
//! tel jour, est bloquée, payée d'avance ou soumise à la validation d'un responsable.

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::db::{Acteur, Db};
use crate::erreur::{Erreur, Resultat};
use crate::permissions as perm;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneRisque {
    #[serde(default)]
    pub id: String,
    pub nom: String,
    #[serde(default)]
    pub quartier: Option<String>,
    /// Microdegrés (entiers).
    #[serde(default)]
    pub lat: Option<i64>,
    #[serde(default)]
    pub lon: Option<i64>,
    #[serde(default)]
    pub rayon_m: Option<i64>,
    /// Minutes depuis minuit ; une plage peut passer minuit (21 h → 6 h).
    pub debut_min: i64,
    pub fin_min: i64,
    /// Bits : lundi = 1, mardi = 2 … dimanche = 64.
    #[serde(default = "tous_les_jours")]
    pub jours: i64,
    /// bloquer | paiement_avance | validation_manuelle
    pub action: String,
    #[serde(default)]
    pub message: String,
    #[serde(default = "vrai")]
    pub actif: bool,
}

fn tous_les_jours() -> i64 {
    127
}
fn vrai() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Decision {
    pub zone_id: String,
    pub zone: String,
    pub action: String,
    pub message: String,
}

pub const ACTIONS: &[&str] = &["validation_manuelle", "paiement_avance", "bloquer"];

fn gravite(action: &str) -> usize {
    ACTIONS.iter().position(|a| *a == action).unwrap_or(0)
}

pub fn lister(conn: &Connection) -> Resultat<Vec<ZoneRisque>> {
    let mut s = conn.prepare(
        "SELECT id, nom, quartier, lat, lon, rayon_m, debut_min, fin_min, jours, action, message, actif FROM zones_risque ORDER BY nom",
    )?;
    let v = s
        .query_map([], |r| {
            Ok(ZoneRisque {
                id: r.get(0)?,
                nom: r.get(1)?,
                quartier: r.get(2)?,
                lat: r.get(3)?,
                lon: r.get(4)?,
                rayon_m: r.get(5)?,
                debut_min: r.get(6)?,
                fin_min: r.get(7)?,
                jours: r.get(8)?,
                action: r.get(9)?,
                message: r.get(10)?,
                actif: r.get(11)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(v)
}

/// RG-ZON-01 : une zone est un quartier et/ou un cercle GPS, avec plage horaire, jours et action.
pub fn enregistrer(db: &mut Db, acteur: &Acteur, z: &ZoneRisque) -> Resultat<String> {
    db.executer(acteur, |op| {
        op.exiger(perm::ZONE_OUTREPASSER)?;
        crate::catalogue::non_vide(&z.nom, "nom de la zone")?;
        let quartier = z.quartier.as_deref().map(str::trim).filter(|q| !q.is_empty());
        let gps = z.lat.is_some() && z.lon.is_some() && z.rayon_m.is_some_and(|r| r > 0);
        if quartier.is_none() && !gps {
            return Err(Erreur::regle("RG-ZON-01", "Indiquez un quartier ou un point GPS avec un rayon"));
        }
        if !(0..1440).contains(&z.debut_min) || !(0..=1440).contains(&z.fin_min) || z.debut_min == z.fin_min {
            return Err(Erreur::regle("RG-ZON-01", "Plage horaire invalide"));
        }
        if !(1..=127).contains(&z.jours) || !ACTIONS.contains(&z.action.as_str()) {
            return Err(Erreur::regle("RG-ZON-01", "Jours ou action invalides"));
        }
        let id = if z.id.is_empty() { op.nouvel_id() } else { z.id.clone() };
        op.execute(
            "INSERT INTO zones_risque(id, nom, quartier, lat, lon, rayon_m, debut_min, fin_min, jours, action, message, actif, modifie_le)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
             ON CONFLICT(id) DO UPDATE SET nom = excluded.nom, quartier = excluded.quartier, lat = excluded.lat,
               lon = excluded.lon, rayon_m = excluded.rayon_m, debut_min = excluded.debut_min, fin_min = excluded.fin_min,
               jours = excluded.jours, action = excluded.action, message = excluded.message, actif = excluded.actif,
               modifie_le = excluded.modifie_le",
            params![
                id,
                z.nom.trim(),
                quartier,
                if gps { z.lat } else { None },
                if gps { z.lon } else { None },
                if gps { z.rayon_m } else { None },
                z.debut_min,
                z.fin_min,
                z.jours,
                z.action,
                z.message.trim(),
                z.actif,
                op.maintenant
            ],
        )?;
        op.audit("zone_risque.enregistrer", "zone_risque", Some(&id), None, Some(json!(z)), None, None)?;
        op.outbox("zone_risque", &id, "enregistrer")?;
        Ok(id)
    })
}

/// Distance en mètres entre deux points en microdegrés (formule de haversine ; pas de montant, les flottants sont admis).
pub fn distance_m(lat1: i64, lon1: i64, lat2: i64, lon2: i64) -> i64 {
    let r = 6_371_000.0_f64;
    let (a1, a2) = ((lat1 as f64 / 1e6).to_radians(), (lat2 as f64 / 1e6).to_radians());
    let dlat = a2 - a1;
    let dlon = ((lon2 - lon1) as f64 / 1e6).to_radians();
    let h = (dlat / 2.0).sin().powi(2) + a1.cos() * a2.cos() * (dlon / 2.0).sin().powi(2);
    (2.0 * r * h.sqrt().asin()).round() as i64
}

/// RG-ZON-02 : règle applicable à ce lieu et à cette heure ; la plus stricte l'emporte
/// (bloquer > paiement d'avance > validation manuelle).
pub fn evaluer(conn: &Connection, quartier: Option<&str>, position: Option<(i64, i64)>, ms: i64) -> Resultat<Option<Decision>> {
    let fuseau = crate::parametres::lire(conn)?.fuseau_minutes;
    let q = quartier.map(|s| s.trim().to_lowercase()).filter(|s| !s.is_empty());
    let mut retenue: Option<Decision> = None;
    for z in lister(conn)?.into_iter().filter(|z| z.actif) {
        let lieu = z.quartier.as_deref().is_some_and(|zq| q.as_deref() == Some(zq.trim().to_lowercase().as_str()))
            || match (position, z.lat, z.lon, z.rayon_m) {
                (Some((la, lo)), Some(zla), Some(zlo), Some(r)) => distance_m(la, lo, zla, zlo) <= r,
                _ => false,
            };
        if !lieu || !crate::horloge::plage_active(z.debut_min, z.fin_min, z.jours, ms, fuseau) {
            continue;
        }
        if retenue.as_ref().is_none_or(|r| gravite(&z.action) > gravite(&r.action)) {
            let message = if z.message.is_empty() {
                match z.action.as_str() {
                    "bloquer" => format!("Livraison indisponible à {} à cette heure", z.nom),
                    "paiement_avance" => format!("{} : paiement Mobile Money d'avance obligatoire à cette heure", z.nom),
                    _ => format!("{} : commande soumise à l'accord d'un responsable", z.nom),
                }
            } else {
                z.message.clone()
            };
            retenue = Some(Decision { zone_id: z.id, zone: z.nom, action: z.action, message });
        }
    }
    Ok(retenue)
}

// ───────────── Liste noire ─────────────

/// Numéro malien normalisé : chiffres seuls, sans indicatif +223 / 00223.
pub fn normaliser_telephone(t: &str) -> String {
    let c: String = t.chars().filter(|c| c.is_ascii_digit()).collect();
    let c = c.strip_prefix("00").unwrap_or(&c).to_string();
    if c.len() == 11 && c.starts_with("223") {
        c[3..].to_string()
    } else {
        c
    }
}

pub fn est_bloque(conn: &Connection, telephone: &str) -> Resultat<Option<String>> {
    Ok(conn
        .query_row("SELECT motif FROM numeros_bloques WHERE telephone = ?1", params![normaliser_telephone(telephone)], |r| r.get(0))
        .optional()?)
}

pub fn bloquer_numero(db: &mut Db, acteur: &Acteur, telephone: &str, motif: &str) -> Resultat<()> {
    db.executer(acteur, |op| {
        op.exiger(perm::ZONE_OUTREPASSER)?;
        let t = normaliser_telephone(telephone);
        if t.len() < 8 || motif.trim().is_empty() {
            return Err(Erreur::validation("Numéro et motif obligatoires"));
        }
        op.execute(
            "INSERT INTO numeros_bloques(telephone, motif, cree_le, cree_par) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(telephone) DO UPDATE SET motif = excluded.motif",
            params![t, motif.trim(), op.maintenant, op.utilisateur()],
        )?;
        op.audit("numero.bloquer", "numero", Some(&t), None, None, Some(motif.trim()), None)?;
        Ok(())
    })
}

pub fn debloquer_numero(db: &mut Db, acteur: &Acteur, telephone: &str) -> Resultat<()> {
    db.executer(acteur, |op| {
        op.exiger(perm::ZONE_OUTREPASSER)?;
        let t = normaliser_telephone(telephone);
        op.execute("DELETE FROM numeros_bloques WHERE telephone = ?1", params![t])?;
        op.audit("numero.debloquer", "numero", Some(&t), None, None, None, None)?;
        Ok(())
    })
}

pub fn numeros_bloques(conn: &Connection) -> Resultat<Vec<(String, String, i64)>> {
    let mut s = conn.prepare("SELECT telephone, motif, cree_le FROM numeros_bloques ORDER BY cree_le DESC")?;
    let v = s.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?.collect::<Result<Vec<_>, _>>()?;
    Ok(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plages_et_distances() {
        // Bamako : ~1,1 km entre ces deux points.
        let d = distance_m(12_639_000, -8_002_000, 12_649_000, -8_002_000);
        assert!((1_100..1_120).contains(&d), "{d}");
        assert_eq!(normaliser_telephone("+223 76 00 00 01"), "76000001");
        assert_eq!(normaliser_telephone("0022376000001"), "76000001");
        assert_eq!(normaliser_telephone("76-00-00-01"), "76000001");
    }
}
