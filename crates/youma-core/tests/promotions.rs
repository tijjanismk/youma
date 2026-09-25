//! Promotions et happy hours (fiche 0017, RG-PRO-01 à 04).

mod commun;

use commun::*;
use youma_core::promotions::{self, Promotion};
use youma_core::commandes;

fn happy_hour(b: &Banc, produit: &str, prix: i64) -> Promotion {
    Promotion {
        id: String::new(),
        nom: "Happy hour".into(),
        produit_id: Some(b.produit(produit)),
        categorie_id: None,
        type_: "prix".into(),
        valeur: prix,
        debut_min: 18 * 60,
        fin_min: 20 * 60,
        jours: 127,
        date_debut: None,
        date_fin: None,
        actif: true,
    }
}

fn categorie(b: &Banc, nom: &str) -> String {
    b.db.conn().query_row("SELECT id FROM categories WHERE nom = ?1", [nom], |r| r.get(0)).unwrap()
}

/// (prix unitaire, prix normal, promotion) de la ligne unique de la commande.
fn ligne(b: &Banc, commande: &str) -> (i64, Option<i64>, Option<String>) {
    b.db.conn()
        .query_row("SELECT prix_unitaire, prix_normal, promotion_id FROM lignes_commande WHERE commande_id = ?1", [commande], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })
        .unwrap()
}

#[test]
fn rg_pro_01_promotion_valide() {
    let mut b = banc();
    let g = b.gerant();
    let mut p = happy_hour(&b, "Bière blonde", 750);
    p.categorie_id = Some(categorie(&b, "Bières"));
    assert_eq!(promotions::enregistrer(&mut b.db, &g, &p).unwrap_err().regle_code(), Some("RG-PRO-01"), "produit ou catégorie, pas les deux");
    let mut p = happy_hour(&b, "Bière blonde", 750);
    p.type_ = "pourcentage".into();
    p.valeur = 0;
    assert_eq!(promotions::enregistrer(&mut b.db, &g, &p).unwrap_err().regle_code(), Some("RG-PRO-01"));
    let mut p = happy_hour(&b, "Bière blonde", 750);
    p.date_debut = Some("2026-04-01".into());
    p.date_fin = Some("2026-03-01".into());
    assert_eq!(promotions::enregistrer(&mut b.db, &g, &p).unwrap_err().regle_code(), Some("RG-PRO-01"));
    let s = b.serveur();
    assert!({ let hh = happy_hour(&b, "Bière blonde", 750); promotions::enregistrer(&mut b.db, &s, &hh) }.is_err());
    { let hh = happy_hour(&b, "Bière blonde", 750); promotions::enregistrer(&mut b.db, &g, &hh) }.unwrap();
    assert_eq!(promotions::lister(b.db.conn()).unwrap().len(), 1);
}

#[test]
fn rg_pro_02_03_prix_du_happy_hour_copie_sur_la_ligne() {
    let mut b = banc();
    b.ouvrir_journee();
    let g = b.gerant();
    { let hh = happy_hour(&b, "Bière blonde", 750); promotions::enregistrer(&mut b.db, &g, &hh) }.unwrap();
    // 10 h : prix normal.
    let c = b.commande_table("1", &[("Bière blonde", 1)]);
    assert_eq!(ligne(&b, &c), (1_000, None, None));
    // 19 h : prix du happy hour, prix normal gardé.
    b.horloge.regler_a("2026-03-14", 19, 0);
    let c = b.commande_table("2", &[("Bière blonde", 2)]);
    let (prix, normal, promo) = ligne(&b, &c);
    assert_eq!((prix, normal), (750, Some(1_000)));
    assert!(promo.is_some());
    assert_eq!(b.total(&c).total, 1_500);
    // RG-PRO-03 : saisie à 19 h 59, envoyée après 20 h : le prix copié ne change pas.
    b.horloge.regler_a("2026-03-14", 19, 59);
    let a = b.serveur();
    let t3 = b.table("3");
    let c3 = commandes::ouvrir(&mut b.db, &a, &commandes::NouvelleCommande { type_: "sur_place".into(), table_id: Some(t3), client_id: None, employe_id: None, couverts: 1, note: String::new(), livraison: None, canal: None }).unwrap();
    let l = b.ligne("Bière blonde", 1);
    commandes::ajouter_lignes(&mut b.db, &a, &c3, &[l]).unwrap();
    b.horloge.regler_a("2026-03-14", 20, 5);
    commandes::envoyer(&mut b.db, &a, &c3).unwrap();
    assert_eq!(ligne(&b, &c3).0, 750);
    // 20 h 05 : fini.
    let c4 = b.commande_table("4", &[("Bière blonde", 1)]);
    assert_eq!(ligne(&b, &c4).0, 1_000);
}

#[test]
fn rg_pro_02_la_promotion_la_plus_avantageuse_l_emporte() {
    let mut b = banc();
    let g = b.gerant();
    let grillades = categorie(&b, "Grillades");
    // −20 % sur les grillades le samedi toute la journée, et poulet à 3 000 le soir.
    let moins_20 = Promotion { nom: "Samedi grillades".into(), produit_id: None, categorie_id: Some(grillades), type_: "pourcentage".into(), valeur: 2_000, debut_min: 0, fin_min: 1440, jours: 32, ..happy_hour(&b, "Poulet braisé", 0) };
    promotions::enregistrer(&mut b.db, &g, &moins_20).unwrap();
    let soir = Promotion { nom: "Poulet du soir".into(), valeur: 3_000, ..happy_hour(&b, "Poulet braisé", 0) };
    promotions::enregistrer(&mut b.db, &g, &soir).unwrap();
    let poulet = b.produit("Poulet braisé");
    let prix = |b: &Banc| promotions::prix_du_moment(b.db.conn(), &poulet, None, b.db.maintenant()).unwrap();
    // Samedi 10 h : 3 500 − 20 % = 2 800.
    assert_eq!((prix(&b).prix, prix(&b).promotion.as_deref()), (2_800, Some("Samedi grillades")));
    // Samedi 19 h : 2 800 reste plus bas que 3 000.
    b.horloge.regler_a("2026-03-14", 19, 0);
    assert_eq!(prix(&b).prix, 2_800);
    // Dimanche 19 h : seulement le poulet du soir.
    b.horloge.regler_a("2026-03-15", 19, 0);
    assert_eq!((prix(&b).prix, prix(&b).promotion.as_deref()), (3_000, Some("Poulet du soir")));
    // Un « prix » promotionnel plus cher que le prix normal n'est jamais appliqué.
    let cher = Promotion { nom: "Erreur".into(), valeur: 9_999, debut_min: 0, fin_min: 1440, ..happy_hour(&b, "Coca-Cola", 0) };
    promotions::enregistrer(&mut b.db, &g, &cher).unwrap();
    let coca = b.produit("Coca-Cola");
    assert_eq!(promotions::prix_du_moment(b.db.conn(), &coca, None, b.db.maintenant()).unwrap().promotion_id, None);
    // Dates : promotion terminée hier, inactive.
    let finie = Promotion { nom: "Fête passée".into(), valeur: 100, debut_min: 0, fin_min: 1440, date_fin: Some("2026-03-14".into()), ..happy_hour(&b, "Eau minérale", 0) };
    promotions::enregistrer(&mut b.db, &g, &finie).unwrap();
    assert!(promotions::prix_en_cours(b.db.conn(), None, b.db.maintenant()).unwrap().keys().all(|k| *k != b.produit("Eau minérale")));
    // Prix en cours pour les boutons : le poulet du soir.
    assert_eq!(promotions::prix_en_cours(b.db.conn(), None, b.db.maintenant()).unwrap()[&poulet].prix, 3_000);
}

#[test]
fn rg_pro_04_bilan_et_manque_a_gagner() {
    let mut b = banc();
    b.ouvrir_journee();
    b.ouvrir_caisse(0);
    let g = b.gerant();
    { let hh = happy_hour(&b, "Bière blonde", 750); promotions::enregistrer(&mut b.db, &g, &hh) }.unwrap();
    b.horloge.regler_a("2026-03-14", 18, 30);
    let c = b.commande_table("5", &[("Bière blonde", 4)]);
    b.payer_especes(&c, 3_000);
    let bilan = promotions::bilan(b.db.conn(), "2026-03-14", "2026-03-14").unwrap();
    assert_eq!(bilan.len(), 1);
    assert_eq!((bilan[0].quantite, bilan[0].ventes, bilan[0].manque_a_gagner), (4, 3_000, 1_000));
}
