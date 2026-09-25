//! Statistiques avancées (RG-STA-01 à 04).

mod commun;

use commun::*;
use serde_json::json;
use youma_core::rapports::{self, periode_precedente, variation_bp};

fn tableau<'a>(r: &'a rapports::Rapport, titre: &str) -> &'a rapports::Tableau {
    r.tableaux.iter().find(|t| t.titre.starts_with(titre)).unwrap()
}

fn indicateur(r: &rapports::Rapport, cle: &str) -> i64 {
    r.indicateurs.iter().find(|i| i.cle == cle).unwrap().valeur
}

#[test]
fn rg_sta_01_a_03_panier_heures_jours_serveurs() {
    let mut b = banc();
    b.ouvrir_journee();
    b.ouvrir_caisse(0);
    // 10 h : 2 poulets (7 000) ; 19 h : 1 coca (750).
    let c1 = b.commande_table("1", &[("Poulet braisé", 2)]);
    b.payer_especes(&c1, 7_000);
    b.horloge.regler_a("2026-03-14", 19, 0);
    let c2 = b.commande_table("2", &[("Coca-Cola", 1)]);
    b.payer_especes(&c2, 750);
    let r = rapports::rapport_statistiques(b.db.conn(), "2026-03-14", "2026-03-14").unwrap();
    // RG-STA-01 : (7 000 + 750) ÷ 2.
    assert_eq!(indicateur(&r, "panier_moyen"), 3_875);
    // Les commandes de la démo ont 2 couverts : 7 750 ÷ 4.
    assert_eq!(indicateur(&r, "par_couvert"), 1_937);
    // RG-STA-02 : heure et jour du paiement.
    let h = tableau(&r, "Ventes par heure");
    assert_eq!(h.lignes.len(), 2);
    assert_eq!(h.lignes[0], vec![json!("10 h"), json!(1), json!(7_000), json!(7_000)]);
    assert_eq!(h.lignes[1][0], "19 h");
    let j = tableau(&r, "Ventes par jour");
    assert_eq!(j.lignes, vec![vec![json!("samedi"), json!(2), json!(7_750), json!(3_875)]]);
    // RG-STA-03 : la serveuse Awa a ouvert les deux tables.
    let s = tableau(&r, "Serveurs");
    assert_eq!(s.lignes[0][0], "Awa");
    assert_eq!((s.lignes[0][1].clone(), s.lignes[0][2].clone()), (json!(2), json!(7_750)));
    // Produits jamais vendus en tête des moins vendus.
    let m = tableau(&r, "Produits les moins vendus");
    assert_eq!(m.lignes[0][1], 0);
    assert!(m.lignes.len() <= 10);
}

#[test]
fn rg_sta_04_periode_precedente_et_variation() {
    assert_eq!(periode_precedente("2026-03-14", "2026-03-14"), Some(("2026-03-13".into(), "2026-03-13".into())));
    assert_eq!(periode_precedente("2026-03-01", "2026-03-31"), Some(("2026-01-29".into(), "2026-02-28".into())));
    assert_eq!(periode_precedente("x", "2026-03-31"), None);
    assert_eq!(variation_bp(1_500, 1_000), 5_000);
    assert_eq!(variation_bp(750, 1_000), -2_500);
    assert_eq!(variation_bp(750, 0), 0);
    let b = banc();
    let r = rapports::rapport_statistiques(b.db.conn(), "2026-03-14", "2026-03-14").unwrap();
    let c = tableau(&r, "Comparaison avec la période précédente");
    assert_eq!(c.titre, "Comparaison avec la période précédente (du 2026-03-13 au 2026-03-13)");
    assert_eq!(c.lignes[0], vec![json!("Chiffre d'affaires"), json!("0 FCFA"), json!("0 FCFA"), json!("0,00 %")]);
}
