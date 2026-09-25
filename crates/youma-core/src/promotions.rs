//! Promotions et happy hours (fiche 0017, RG-PRO-01 à 04).
//! « Bière à 750 FCFA de 18 h à 20 h en semaine », « −20 % sur les grillades le dimanche ».

use std::collections::HashMap;

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::db::{Acteur, Db};
use crate::erreur::{Erreur, Resultat};
use crate::permissions as perm;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Promotion {
    #[serde(default)]
    pub id: String,
    pub nom: String,
    #[serde(default)]
    pub produit_id: Option<String>,
    #[serde(default)]
    pub categorie_id: Option<String>,
    /// prix : prix fixe (FCFA) ; pourcentage : remise en points de base (2 500 = 25 %).
    #[serde(rename = "type")]
    pub type_: String,
    pub valeur: i64,
    pub debut_min: i64,
    pub fin_min: i64,
    #[serde(default = "tous_les_jours")]
    pub jours: i64,
    #[serde(default)]
    pub date_debut: Option<String>,
    #[serde(default)]
    pub date_fin: Option<String>,
    #[serde(default = "vrai")]
    pub actif: bool,
}

fn tous_les_jours() -> i64 {
    127
}
fn vrai() -> bool {
    true
}

pub fn lister(conn: &Connection) -> Resultat<Vec<Promotion>> {
    let mut s = conn.prepare(
        "SELECT id, nom, produit_id, categorie_id, type, valeur, debut_min, fin_min, jours, date_debut, date_fin, actif
         FROM promotions ORDER BY actif DESC, nom",
    )?;
    let v = s
        .query_map([], |r| {
            Ok(Promotion {
                id: r.get(0)?,
                nom: r.get(1)?,
                produit_id: r.get(2)?,
                categorie_id: r.get(3)?,
                type_: r.get(4)?,
                valeur: r.get(5)?,
                debut_min: r.get(6)?,
                fin_min: r.get(7)?,
                jours: r.get(8)?,
                date_debut: r.get(9)?,
                date_fin: r.get(10)?,
                actif: r.get(11)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(v)
}

fn date_valide(d: &Option<String>) -> bool {
    d.as_deref().is_none_or(|d| chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").is_ok())
}

/// RG-PRO-01 : un produit ou une catégorie ; prix fixe ≥ 0 ou remise de 1 à 100 % ; plage horaire, jours, dates.
pub fn enregistrer(db: &mut Db, acteur: &Acteur, p: &Promotion) -> Resultat<String> {
    db.executer(acteur, |op| {
        op.exiger(perm::CATALOGUE_GERER)?;
        crate::catalogue::non_vide(&p.nom, "nom de la promotion")?;
        if p.produit_id.is_some() == p.categorie_id.is_some() {
            return Err(Erreur::regle("RG-PRO-01", "Choisissez un produit ou une catégorie"));
        }
        match p.type_.as_str() {
            "prix" if p.valeur >= 0 => {}
            "pourcentage" if (1..=10_000).contains(&p.valeur) => {}
            _ => return Err(Erreur::regle("RG-PRO-01", "Prix promotionnel ou pourcentage (1 à 100 %) invalide")),
        }
        if !(0..1440).contains(&p.debut_min) || !(0..=1440).contains(&p.fin_min) || p.debut_min == p.fin_min || !(1..=127).contains(&p.jours) {
            return Err(Erreur::regle("RG-PRO-01", "Plage horaire ou jours invalides"));
        }
        if !date_valide(&p.date_debut) || !date_valide(&p.date_fin) || matches!((&p.date_debut, &p.date_fin), (Some(d), Some(f)) if d > f) {
            return Err(Erreur::regle("RG-PRO-01", "Dates invalides"));
        }
        let id = if p.id.is_empty() { op.nouvel_id() } else { p.id.clone() };
        op.execute(
            "INSERT INTO promotions(id, nom, produit_id, categorie_id, type, valeur, debut_min, fin_min, jours, date_debut, date_fin, actif, modifie_le)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
             ON CONFLICT(id) DO UPDATE SET nom = excluded.nom, produit_id = excluded.produit_id, categorie_id = excluded.categorie_id,
               type = excluded.type, valeur = excluded.valeur, debut_min = excluded.debut_min, fin_min = excluded.fin_min,
               jours = excluded.jours, date_debut = excluded.date_debut, date_fin = excluded.date_fin, actif = excluded.actif,
               modifie_le = excluded.modifie_le",
            params![
                id,
                p.nom.trim(),
                p.produit_id,
                p.categorie_id,
                p.type_,
                p.valeur,
                p.debut_min,
                p.fin_min,
                p.jours,
                p.date_debut,
                p.date_fin,
                p.actif,
                op.maintenant
            ],
        )?;
        op.audit("promotion.enregistrer", "promotion", Some(&id), None, Some(json!(p)), None, None)?;
        op.outbox("promotion", &id, "enregistrer")?;
        op.evenement("catalogue", None);
        Ok(id)
    })
}

fn active(p: &Promotion, ms: i64, fuseau: i64) -> bool {
    let jour = crate::horloge::date_locale(ms, fuseau);
    p.actif
        && p.date_debut.as_deref().is_none_or(|d| jour.as_str() >= d)
        && p.date_fin.as_deref().is_none_or(|f| jour.as_str() <= f)
        && crate::horloge::plage_active(p.debut_min, p.fin_min, p.jours, ms, fuseau)
}

fn appliquer(p: &Promotion, prix: i64) -> i64 {
    match p.type_.as_str() {
        // Un « prix » promotionnel supérieur au prix normal n'est jamais appliqué.
        "prix" => p.valeur.min(prix),
        // Arrondi au franc le plus proche.
        _ => (prix * (10_000 - p.valeur) + 5_000) / 10_000,
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PrixDuMoment {
    pub prix: i64,
    pub prix_normal: i64,
    pub promotion_id: Option<String>,
    pub promotion: Option<String>,
}

/// RG-PRO-02 : prix de la zone, remplacé par le plus bas des prix promotionnels actifs à cet instant.
pub fn prix_du_moment(conn: &Connection, produit_id: &str, zone_id: Option<&str>, ms: i64) -> Resultat<PrixDuMoment> {
    let normal = crate::catalogue::prix_effectif(conn, produit_id, zone_id)?;
    let categorie: String = conn.query_row("SELECT categorie_id FROM produits WHERE id = ?1", params![produit_id], |r| r.get(0))?;
    let fuseau = crate::parametres::lire(conn)?.fuseau_minutes;
    let mut meilleur = PrixDuMoment { prix: normal, prix_normal: normal, promotion_id: None, promotion: None };
    for p in lister(conn)?
        .into_iter()
        .filter(|p| p.produit_id.as_deref() == Some(produit_id) || p.categorie_id.as_deref() == Some(categorie.as_str()))
        .filter(|p| active(p, ms, fuseau))
    {
        let prix = appliquer(&p, normal);
        if prix < meilleur.prix {
            meilleur = PrixDuMoment { prix, prix_normal: normal, promotion_id: Some(p.id), promotion: Some(p.nom) };
        }
    }
    Ok(meilleur)
}

/// Prix promotionnels en cours pour une zone (affichage des boutons de prise de commande et du menu client).
pub fn prix_en_cours(conn: &Connection, zone_id: Option<&str>, ms: i64) -> Resultat<HashMap<String, PrixDuMoment>> {
    let fuseau = crate::parametres::lire(conn)?.fuseau_minutes;
    if !lister(conn)?.iter().any(|p| active(p, ms, fuseau)) {
        return Ok(HashMap::new());
    }
    let ids: Vec<String> = conn.prepare("SELECT id FROM produits WHERE actif = 1")?.query_map([], |r| r.get(0))?.collect::<Result<_, _>>()?;
    let mut v = HashMap::new();
    for id in ids {
        let p = prix_du_moment(conn, &id, zone_id, ms)?;
        if p.promotion_id.is_some() {
            v.insert(id, p);
        }
    }
    Ok(v)
}

#[derive(Debug, Serialize)]
pub struct BilanPromotion {
    pub promotion: String,
    pub quantite: i64,
    pub ventes: i64,
    /// Σ (prix normal − prix promotionnel) × quantité vendue.
    pub manque_a_gagner: i64,
}

/// RG-PRO-04 : ventes en promotion et manque à gagner sur une période de journées d'exploitation.
pub fn bilan(conn: &Connection, debut: &str, fin: &str) -> Resultat<Vec<BilanPromotion>> {
    let mut s = conn.prepare(
        "SELECT pr.nom, SUM(l.quantite - l.quantite_annulee), SUM((l.quantite - l.quantite_annulee) * l.prix_unitaire),
                SUM((l.quantite - l.quantite_annulee) * (l.prix_normal - l.prix_unitaire))
         FROM lignes_commande l JOIN promotions pr ON pr.id = l.promotion_id
         JOIN commandes c ON c.id = l.commande_id JOIN journees j ON j.id = c.journee_id
         WHERE c.statut IN ('payee','cloturee') AND l.offert = 0 AND j.date_exploitation BETWEEN ?1 AND ?2
         GROUP BY pr.id ORDER BY pr.nom",
    )?;
    let v = s
        .query_map(params![debut, fin], |r| Ok(BilanPromotion { promotion: r.get(0)?, quantite: r.get(1)?, ventes: r.get(2)?, manque_a_gagner: r.get(3)? }))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(v)
}
