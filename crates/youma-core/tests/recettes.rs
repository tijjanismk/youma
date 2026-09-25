//! Recettes et consommation théorique (fiche 0014, RG-REC-01 à 04).

mod commun;

use commun::*;
use youma_core::commandes::{self, LigneSaisie};
use youma_core::recettes::{self, LigneRecette, Recette, RecetteOption};
use youma_core::catalogue;

fn l(article: &str, quantite: i64) -> LigneRecette {
    LigneRecette { article_id: article.into(), quantite, article_nom: String::new(), unite: String::new(), cout_unitaire: 0 }
}

fn option(b: &Banc, produit: &str, nom: &str) -> String {
    let p = catalogue::produit(b.db.conn(), &b.produit(produit)).unwrap();
    p.groupes_options.iter().flat_map(|g| g.options.iter()).find(|o| o.nom == nom).unwrap().id.clone()
}

/// Commande à table de `quantite` frites avec l'option donnée, envoyée ; renvoie (commande, ligne).
fn frites(b: &mut Banc, quantite: i64, taille: &str) -> (String, String) {
    let a = b.serveur();
    let table = b.table("6");
    let id = commandes::ouvrir(&mut b.db, &a, &commandes::NouvelleCommande {
        type_: "sur_place".into(),
        table_id: Some(table),
        client_id: None,
        employe_id: None,
        couverts: 1,
        note: String::new(),
        livraison: None,
        canal: None,
    })
    .unwrap();
    let ligne = LigneSaisie { produit_id: b.produit("Frites"), quantite, options: vec![option(b, "Frites", taille)], commentaire: String::new() };
    let ids = commandes::ajouter_lignes(&mut b.db, &a, &id, &[ligne]).unwrap();
    commandes::envoyer(&mut b.db, &a, &id).unwrap();
    (id, ids[0].clone())
}

#[test]
fn rg_rec_01_recette_valide_et_droits() {
    let mut b = banc();
    let pdt = b.article("Pommes de terre");
    let alloco = b.produit("Alloco");
    let g = b.gerant();
    let r = |lignes: Vec<LigneRecette>| Recette { produit_id: alloco.clone(), lignes, options: vec![], cout: 0 };
    assert_eq!(recettes::definir(&mut b.db, &g, &r(vec![l(&pdt, 0)])).unwrap_err().regle_code(), Some("RG-REC-01"));
    assert_eq!(recettes::definir(&mut b.db, &g, &r(vec![l(&pdt, 10), l(&pdt, 20)])).unwrap_err().regle_code(), Some("RG-REC-01"));
    let s = b.serveur();
    assert!(recettes::definir(&mut b.db, &s, &r(vec![l(&pdt, 10)])).is_err());
    // Option d'un autre produit : refusée.
    let grande = option(&b, "Frites", "Grande");
    let mut autre = r(vec![l(&pdt, 10)]);
    autre.options = vec![RecetteOption { option_id: grande, option_nom: String::new(), lignes: vec![l(&pdt, 5)] }];
    assert!(recettes::definir(&mut b.db, &g, &autre).is_err());
    // Recette valide → suivi « recette » ; recette vidée → « aucun ».
    recettes::definir(&mut b.db, &g, &r(vec![l(&pdt, 200)])).unwrap();
    assert_eq!(catalogue::produit(b.db.conn(), &alloco).unwrap().suivi_stock, "recette");
    recettes::definir(&mut b.db, &g, &r(vec![])).unwrap();
    assert_eq!(catalogue::produit(b.db.conn(), &alloco).unwrap().suivi_stock, "aucun");
    assert_eq!(b.compter("SELECT COUNT(*) FROM journal_audit WHERE action = 'recette.definir'"), 2 + 1, "démo + 2 modifications");
}

#[test]
fn rg_rec_02_vente_sort_les_ingredients_du_plat_et_des_options() {
    let mut b = banc();
    b.ouvrir_journee();
    let (pdt, huile) = (b.stock("Pommes de terre"), b.stock("Huile"));
    // 2 frites « Grande » : (250 + 150) g × 2 et 30 ml × 2.
    let (_, ligne) = frites(&mut b, 2, "Grande");
    assert_eq!(b.stock("Pommes de terre"), pdt - 800);
    assert_eq!(b.stock("Huile"), huile - 60);
    // Coût d'une unité : 400 g × 1 + 30 ml × 2 = 460 FCFA (bénéfice estimé).
    let cout: i64 = b.db.conn().query_row("SELECT cout_unitaire FROM lignes_commande WHERE id = ?1", [&ligne], |r| r.get(0)).unwrap();
    assert_eq!(cout, 460);
    // Normale : sans l'ingrédient de l'option.
    frites(&mut b, 1, "Normale");
    assert_eq!(b.stock("Pommes de terre"), pdt - 1_050);
    assert_eq!(b.compter("SELECT COUNT(*) FROM mouvements_stock WHERE type = 'vente' AND reference_type = 'ligne_commande'"), 4);
}

#[test]
fn rg_rec_03_annulation_rend_les_ingredients_sauf_perte() {
    let mut b = banc();
    b.ouvrir_journee();
    let (pdt, huile) = (b.stock("Pommes de terre"), b.stock("Huile"));
    let (_, ligne) = frites(&mut b, 3, "Grande");
    assert_eq!(b.stock("Pommes de terre"), pdt - 1_200);
    let s = Banc::avec_pin_gerant(b.serveur());
    // Annulée sans perte (pas encore préparée) : 400 g rendus.
    commandes::annuler_ligne(&mut b.db, &s, &ligne, 1, "Erreur de saisie", false).unwrap();
    assert_eq!(b.stock("Pommes de terre"), pdt - 800);
    // Annulée avec perte (déjà cuite) : rien ne revient.
    commandes::annuler_ligne(&mut b.db, &s, &ligne, 1, "Tombée par terre", true).unwrap();
    assert_eq!(b.stock("Pommes de terre"), pdt - 800);
    // La dernière, sans perte : le reste revient, rien de plus.
    commandes::annuler_ligne(&mut b.db, &s, &ligne, 1, "Client parti", false).unwrap();
    assert_eq!(b.stock("Pommes de terre"), pdt - 400);
    // Huile : 90 ml sortis, 2 × 30 ml rendus.
    assert_eq!(b.stock("Huile"), huile - 30);
}

#[test]
fn rg_rec_04_cout_matiere_et_options_conservees() {
    let mut b = banc();
    let c = recettes::couts_matiere(b.db.conn()).unwrap();
    let f = c.iter().find(|x| x.nom == "Frites").unwrap();
    // 250 g × 1 + 30 ml × 2 = 310 FCFA pour 750 FCFA : 41,33 %.
    assert_eq!((f.cout, f.prix, f.part_bp), (310, 750, 4_133));
    // Modifier le produit (prix) garde la recette de l'option « Grande ».
    let frites_id = b.produit("Frites");
    let mut p = catalogue::produit(b.db.conn(), &frites_id).unwrap();
    p.prix = 1_000;
    let g = b.proprietaire();
    catalogue::enregistrer_produit(&mut b.db, &g, &p).unwrap();
    let r = recettes::lire(b.db.conn(), &frites_id).unwrap();
    assert_eq!(r.options.len(), 1);
    assert_eq!(r.options[0].option_nom, "Grande");
    // Option supprimée : sa recette disparaît, le produit s'enregistre.
    p.groupes_options[0].options.retain(|o| o.nom != "Grande");
    catalogue::enregistrer_produit(&mut b.db, &g, &p).unwrap();
    assert!(recettes::lire(b.db.conn(), &frites_id).unwrap().options.is_empty());
    assert_eq!(recettes::lire(b.db.conn(), &frites_id).unwrap().cout, 310);
}

#[test]
fn disponibles_articles_et_portions() {
    let mut b = banc();
    let d = youma_core::catalogue::disponibles(b.db.conn()).unwrap();
    // Démo : 36 bières reçues ; frites sans stock de pommes de terre → 0 portion.
    assert_eq!(d[&b.produit("Bière blonde")], 36);
    assert_eq!(d[&b.produit("Frites")], 0);
    assert!(!d.contains_key(&b.produit("Poulet braisé")), "sans suivi de stock");
    // 1 kg de pommes de terre et 1 L d'huile : 1000 ÷ 250 = 4 portions, 1000 ÷ 30 = 33 → 4.
    let g = b.gerant();
    for (article, q) in [("Pommes de terre", 1_000), ("Huile", 1_000)] {
        let a = b.article(article);
        youma_core::stock::mouvement_manuel(&mut b.db, &g, &youma_core::stock::MouvementManuel { article_id: a, type_: "regularisation".into(), quantite: q, motif: "Achat au marché".into(), conditionnement_id: None }).unwrap();
    }
    assert_eq!(youma_core::catalogue::disponibles(b.db.conn()).unwrap()[&b.produit("Frites")], 4);
}
