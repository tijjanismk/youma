use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::erreur::Resultat;

/// Paramètres du restaurant (JSON unique sous la clé `general`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Parametres {
    /// Heure de bascule de la journée d'exploitation (RG-JOU-02).
    pub heure_bascule: i64,
    /// Décalage local en minutes (Mali : 0).
    pub fuseau_minutes: i64,
    /// Arrondi des parts et remises (FCFA).
    pub arrondi: i64,
    /// Seuil d'écart de caisse au-delà duquel un motif est exigé (RG-CAI-09).
    pub seuil_ecart_caisse: i64,
    /// RG-CAI-04.
    pub reference_mm_obligatoire: bool,
    /// Ordre de paiement par type de commande (RG-CMD-11).
    pub paiement_avant_comptoir: bool,
    pub paiement_avant_emporter: bool,
    /// Minutes d'inactivité avant verrouillage (RG-AUT-04).
    pub verrouillage_minutes: i64,
    pub paie: ParametresPaie,
    pub cotisations: Cotisations,
    /// Coupures proposées pour le billetage.
    pub coupures: Vec<i64>,
    /// Quartiers de livraison et frais (RG-LIV-01).
    pub quartiers: Vec<QuartierLivraison>,
    pub largeur_ticket: usize,
    /// Imprimante des tickets clients et rapports : '', 'tcp:IP:PORT', 'fichier:CHEMIN', 'windows:NOM'.
    pub imprimante_caisse: String,
    /// Ouvrir le tiroir-caisse (via l'imprimante) à chaque encaissement espèces.
    pub ouvrir_tiroir: bool,
    /// Second emplacement de sauvegarde (clé USB, autre disque).
    pub dossier_sauvegarde_externe: String,
    /// Alerte si aucune sauvegarde externe depuis N jours.
    pub alerte_sauvegarde_jours: i64,
    /// Sauvegarde automatique pendant le service (minutes).
    pub intervalle_sauvegarde_minutes: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ParametresPaie {
    /// RG-PAI-08.
    pub deduire_absences: bool,
    pub jours_ouvrables_mois: i64,
    /// RG-PAI-02 : plafond d'avance en % du salaire de base (0 = pas de plafond).
    pub plafond_avance_pct: i64,
}

/// RG-PAI-07 : cotisations sociales facultatives, désactivées par défaut.
/// Taux en points de base (1 % = 100). [HYPOTHÈSE] valeurs indicatives, à faire valider.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Cotisations {
    pub inps_active: bool,
    pub inps_salarie_bp: i64,
    pub inps_employeur_bp: i64,
    pub amo_active: bool,
    pub amo_salarie_bp: i64,
    pub amo_employeur_bp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuartierLivraison {
    pub nom: String,
    pub frais: i64,
}

impl Default for Parametres {
    fn default() -> Self {
        Parametres {
            heure_bascule: 6,
            fuseau_minutes: 0,
            arrondi: 25,
            seuil_ecart_caisse: 500,
            reference_mm_obligatoire: true,
            paiement_avant_comptoir: true,
            paiement_avant_emporter: true,
            verrouillage_minutes: 15,
            paie: ParametresPaie::default(),
            cotisations: Cotisations::default(),
            coupures: vec![10_000, 5_000, 2_000, 1_000, 500, 250, 200, 100, 50, 25, 10, 5],
            quartiers: vec![],
            largeur_ticket: 42,
            imprimante_caisse: String::new(),
            ouvrir_tiroir: false,
            dossier_sauvegarde_externe: String::new(),
            alerte_sauvegarde_jours: 3,
            intervalle_sauvegarde_minutes: 30,
        }
    }
}

impl Default for ParametresPaie {
    fn default() -> Self {
        ParametresPaie { deduire_absences: false, jours_ouvrables_mois: 26, plafond_avance_pct: 50 }
    }
}

impl Default for Cotisations {
    fn default() -> Self {
        Cotisations {
            inps_active: false,
            inps_salarie_bp: 360,
            inps_employeur_bp: 1640,
            amo_active: false,
            amo_salarie_bp: 306,
            amo_employeur_bp: 350,
        }
    }
}

pub fn lire(conn: &Connection) -> Resultat<Parametres> {
    let v: Option<String> = conn
        .query_row("SELECT valeur FROM parametres WHERE cle = 'general'", [], |r| r.get(0))
        .optional()?;
    Ok(match v {
        Some(s) => serde_json::from_str(&s)?,
        None => Parametres::default(),
    })
}

pub fn ecrire(conn: &Connection, p: &Parametres) -> Resultat<()> {
    conn.execute(
        "INSERT INTO parametres(cle, valeur) VALUES ('general', ?1)
         ON CONFLICT(cle) DO UPDATE SET valeur = excluded.valeur",
        params![serde_json::to_string(p)?],
    )?;
    Ok(())
}

/// Arrondi au multiple le plus proche (demi vers le haut). Entiers uniquement.
pub fn arrondir(montant: i64, pas: i64) -> i64 {
    if pas <= 1 {
        return montant;
    }
    let signe = if montant < 0 { -1 } else { 1 };
    let m = montant.abs();
    signe * ((m + pas / 2) / pas) * pas
}

/// Pourcentage en points de base, arrondi à l'entier le plus proche.
pub fn appliquer_bp(montant: i64, bp: i64) -> i64 {
    (montant * bp + 5_000) / 10_000
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arrondi_au_multiple() {
        assert_eq!(arrondir(4_166, 25), 4_175);
        assert_eq!(arrondir(4_162, 25), 4_150);
        assert_eq!(arrondir(4_163, 1), 4_163);
        assert_eq!(arrondir(-4_166, 25), -4_175);
    }

    #[test]
    fn points_de_base() {
        assert_eq!(appliquer_bp(100_000, 360), 3_600);
        assert_eq!(appliquer_bp(75_000, 306), 2_295);
    }
}
