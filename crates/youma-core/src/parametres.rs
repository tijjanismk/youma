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
    /// Canaux de commande, chacun activable indépendamment (fiche 0013).
    pub canaux: Canaux,
}

/// RG-CAN-01 : le menu papier (saisie par le serveur) est toujours disponible ; les autres canaux sont optionnels.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Canaux {
    /// Commandes prises au téléphone par le caissier.
    pub telephone: bool,
    /// QR sur la table : le client commande depuis son téléphone (Wi-Fi du restaurant ou relais).
    pub qr_table: bool,
    /// Commandes en ligne (livraison, à emporter) via le relais Internet optionnel.
    pub en_ligne: bool,
    /// Paiement Mobile Money d'avance autorisé pour les commandes à distance.
    pub paiement_avance: bool,
    /// Paiement à la livraison / au retrait autorisé.
    pub paiement_a_la_livraison: bool,
    /// Au-delà de ce montant, paiement d'avance obligatoire (0 = pas de limite).
    pub plafond_paiement_livraison: i64,
    /// Nouveau client (numéro jamais servi) : paiement d'avance obligatoire.
    pub avance_nouveau_client: bool,
    /// aucune | sms | rappel (RG-CAN-04).
    pub verification_numero: String,
    /// Relais Internet optionnel (étape B) : adresse et clé du restaurant.
    pub relais_url: String,
    pub relais_cle: String,
    /// Fournisseur SMS (modèle d'URL avec {telephone} et {message}) ; vide = pas de SMS, rappel par le caissier.
    pub sms_url: String,
}

impl Default for Canaux {
    fn default() -> Self {
        Canaux {
            telephone: true,
            qr_table: false,
            en_ligne: false,
            paiement_avance: true,
            paiement_a_la_livraison: true,
            plafond_paiement_livraison: 0,
            avance_nouveau_client: false,
            verification_numero: "rappel".into(),
            relais_url: String::new(),
            relais_cle: String::new(),
            sms_url: String::new(),
        }
    }
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
/// Taux en points de base (1 % = 100), saisis à la main par le restaurateur (aucun taux imposé).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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
            canaux: Canaux::default(),
        }
    }
}

impl Default for ParametresPaie {
    fn default() -> Self {
        ParametresPaie { deduire_absences: false, jours_ouvrables_mois: 26, plafond_avance_pct: 50 }
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

// ───────────── Restaurant, paramètres, journal d'audit (écriture auditée) ─────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Restaurant {
    pub nom: String,
    #[serde(default)]
    pub adresse: String,
    #[serde(default)]
    pub telephone: String,
    #[serde(default)]
    pub ville: String,
    #[serde(default)]
    pub nif: String,
    #[serde(default)]
    pub pied_ticket: String,
}

pub fn restaurant(conn: &Connection) -> Resultat<Restaurant> {
    Ok(conn.query_row("SELECT nom, adresse, telephone, ville, nif, pied_ticket FROM restaurant LIMIT 1", [], |r| {
        Ok(Restaurant { nom: r.get(0)?, adresse: r.get(1)?, telephone: r.get(2)?, ville: r.get(3)?, nif: r.get(4)?, pied_ticket: r.get(5)? })
    })?)
}

pub fn modifier_restaurant(db: &mut crate::Db, acteur: &crate::Acteur, r: &Restaurant) -> Resultat<()> {
    db.executer(acteur, |op| {
        op.exiger(crate::permissions::PARAMETRE_GERER)?;
        crate::catalogue::non_vide(&r.nom, "nom du restaurant")?;
        op.execute(
            "UPDATE restaurant SET nom = ?1, adresse = ?2, telephone = ?3, ville = ?4, nif = ?5, pied_ticket = ?6, modifie_le = ?7",
            params![r.nom.trim(), r.adresse, r.telephone, r.ville, r.nif, r.pied_ticket, op.maintenant],
        )?;
        op.audit("restaurant.modifier", "restaurant", None, None, Some(serde_json::json!(r)), None, None)?;
        Ok(())
    })
}

pub fn modifier(db: &mut crate::Db, acteur: &crate::Acteur, p: &Parametres) -> Resultat<()> {
    db.executer(acteur, |op| {
        op.exiger(crate::permissions::PARAMETRE_GERER)?;
        if !(0..=12).contains(&p.heure_bascule) || p.arrondi < 1 || p.seuil_ecart_caisse < 0 || p.largeur_ticket < 24 {
            return Err(crate::Erreur::validation("Paramètre hors limites"));
        }
        // RG-PAI-07 : taux saisis à la main ; activer sans taux n'aurait aucun effet visible.
        let c = &p.cotisations;
        let hors = |bp: i64| !(0..=5_000).contains(&bp);
        if [c.inps_salarie_bp, c.inps_employeur_bp, c.amo_salarie_bp, c.amo_employeur_bp].into_iter().any(hors) {
            return Err(crate::Erreur::validation("Taux de cotisation entre 0 et 50 %"));
        }
        if (c.inps_active && c.inps_salarie_bp == 0) || (c.amo_active && c.amo_salarie_bp == 0) {
            return Err(crate::Erreur::regle("RG-PAI-07", "Saisissez le taux de cotisation avant de l'activer"));
        }
        let avant = serde_json::to_value(&op.params)?;
        ecrire(op, p)?;
        op.audit("parametres.modifier", "parametres", None, Some(avant), Some(serde_json::to_value(p)?), None, None)?;
        Ok(())
    })
}

#[derive(Debug, Serialize)]
pub struct LigneAudit {
    pub horodatage: i64,
    pub utilisateur: Option<String>,
    pub autorise_par: Option<String>,
    pub appareil: Option<String>,
    pub action: String,
    pub entite: String,
    pub avant: Option<String>,
    pub apres: Option<String>,
    pub motif: Option<String>,
}

pub fn journal_audit(conn: &Connection, action: Option<&str>, limite: i64) -> Resultat<Vec<LigneAudit>> {
    let mut s = conn.prepare(
        "SELECT a.horodatage, u.nom, v.nom, p.nom, a.action, a.entite, a.avant, a.apres, a.motif FROM journal_audit a
         LEFT JOIN utilisateurs u ON u.id = a.utilisateur_id LEFT JOIN utilisateurs v ON v.id = a.autorise_par
         LEFT JOIN appareils p ON p.id = a.appareil_id
         WHERE (?1 IS NULL OR a.action LIKE ?1 || '%') ORDER BY a.horodatage DESC, a.rowid DESC LIMIT ?2",
    )?;
    let v = s
        .query_map(params![action, limite], |r| {
            Ok(LigneAudit {
                horodatage: r.get(0)?,
                utilisateur: r.get(1)?,
                autorise_par: r.get(2)?,
                appareil: r.get(3)?,
                action: r.get(4)?,
                entite: r.get(5)?,
                avant: r.get(6)?,
                apres: r.get(7)?,
                motif: r.get(8)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(v)
}
