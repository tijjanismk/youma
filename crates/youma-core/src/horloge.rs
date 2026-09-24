use std::sync::atomic::{AtomicI64, Ordering};

use chrono::{DateTime, Duration, NaiveDate, Utc};

/// Source de temps, injectable pour les tests (horloge remise à 2000, etc.).
pub trait Horloge: Send + Sync {
    /// Millisecondes UTC.
    fn maintenant_ms(&self) -> i64;
}

pub struct HorlogeSysteme;

impl Horloge for HorlogeSysteme {
    fn maintenant_ms(&self) -> i64 {
        Utc::now().timestamp_millis()
    }
}

/// Horloge pilotée à la main (tests, démonstration).
pub struct HorlogeFixe(AtomicI64);

impl HorlogeFixe {
    pub fn new(ms: i64) -> Self {
        HorlogeFixe(AtomicI64::new(ms))
    }
    /// Horloge à une date/heure UTC donnée, ex. `("2026-03-14", 20, 30)`.
    pub fn a(date: &str, heure: u32, minute: u32) -> Self {
        Self::new(ms_de(date, heure, minute))
    }
    pub fn regler(&self, ms: i64) {
        self.0.store(ms, Ordering::SeqCst)
    }
    pub fn regler_a(&self, date: &str, heure: u32, minute: u32) {
        self.regler(ms_de(date, heure, minute))
    }
    pub fn avancer_minutes(&self, minutes: i64) {
        self.0.fetch_add(minutes * 60_000, Ordering::SeqCst);
    }
}

impl Horloge for HorlogeFixe {
    fn maintenant_ms(&self) -> i64 {
        self.0.load(Ordering::SeqCst)
    }
}

impl<T: Horloge + ?Sized> Horloge for std::sync::Arc<T> {
    fn maintenant_ms(&self) -> i64 {
        (**self).maintenant_ms()
    }
}

pub fn ms_de(date: &str, heure: u32, minute: u32) -> i64 {
    let d = NaiveDate::parse_from_str(date, "%Y-%m-%d").expect("date AAAA-MM-JJ");
    d.and_hms_opt(heure, minute, 0).expect("heure valide").and_utc().timestamp_millis()
}

/// Tolérance avant de considérer que l'horloge a reculé (RG-SYS-01).
pub const TOLERANCE_RECUL_MS: i64 = 5 * 60_000;

pub fn format_ms(ms: i64) -> String {
    DateTime::<Utc>::from_timestamp_millis(ms)
        .map(|d| d.format("%d/%m/%Y %H:%M").to_string())
        .unwrap_or_else(|| ms.to_string())
}

/// Date locale (Mali = UTC+0, décalage configurable) décalée de l'heure de bascule.
/// RG-JOU-02 : 01:30 avec bascule 6 h → date de la veille.
pub fn date_exploitation(ms: i64, fuseau_minutes: i64, heure_bascule: i64) -> String {
    let local = DateTime::<Utc>::from_timestamp_millis(ms).unwrap_or_default()
        + Duration::minutes(fuseau_minutes)
        - Duration::hours(heure_bascule);
    local.format("%Y-%m-%d").to_string()
}

pub fn date_locale(ms: i64, fuseau_minutes: i64) -> String {
    date_exploitation(ms, fuseau_minutes, 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apres_minuit_appartient_a_la_veille() {
        let ms = ms_de("2026-03-15", 1, 30);
        assert_eq!(date_exploitation(ms, 0, 6), "2026-03-14");
        assert_eq!(date_exploitation(ms_de("2026-03-15", 6, 0), 0, 6), "2026-03-15");
    }
}
