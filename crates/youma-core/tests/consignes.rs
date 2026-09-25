//! Consignes : bouteilles et casiers (fiche 0015, RG-CON-01 à 06).

mod commun;

use commun::*;
use youma_core::achats::{self, NouvelAchat};
use youma_core::consignes::{self, ConsigneAchat, Emballage, MouvementEmballage, RetourFournisseur};

fn emballage(b: &Banc, nom: &str) -> String {
    b.db.conn().query_row("SELECT id FROM emballages WHERE nom = ?1", [nom], |r| r.get(0)).unwrap()
}

fn fournisseur(b: &Banc) -> String {
    b.db.conn().query_row("SELECT id FROM fournisseurs WHERE nom LIKE 'Dépôt%'", [], |r| r.get(0)).unwrap()
}

fn etat(b: &Banc, nom: &str) -> consignes::EtatEmballage {
    consignes::etats(b.db.conn()).unwrap().into_iter().find(|e| e.emballage.nom == nom).unwrap()
}

const BOUTEILLE: &str = "Bouteille bière 65 cl";
const CASIER: &str = "Casier bière (12)";

#[test]
fn rg_con_01_emballage_et_articles_lies() {
    let mut b = banc();
    let g = b.gerant();
    let e = Emballage { id: String::new(), nom: "Bouteille Coca".into(), valeur: -1, actif: true, articles: vec![] };
    assert_eq!(consignes::enregistrer(&mut b.db, &g, &e).unwrap_err().regle_code(), Some("RG-CON-01"));
    let coca = b.article("Coca-Cola 33 cl");
    let id = consignes::enregistrer(&mut b.db, &g, &Emballage { valeur: 100, articles: vec![coca.clone()], ..e }).unwrap();
    let lu = consignes::lister(b.db.conn()).unwrap().into_iter().find(|x| x.id == id).unwrap();
    assert_eq!(lu.articles, vec![coca]);
    // 48 Coca en stock, aucune bouteille déclarée détenue : 48 pleins, −48 vides → l'inventaire des vides corrigera.
    let s = etat(&b, "Bouteille Coca");
    assert_eq!((s.detenus, s.pleins, s.vides), (0, Some(48), Some(-48)));
}

#[test]
fn rg_con_02_consigne_a_la_livraison() {
    let mut b = banc();
    b.ouvrir_journee();
    // Démo : 36 bouteilles (150 FCFA) et 3 casiers (2 500 FCFA) reçus avec le stock initial, à crédit.
    let e = etat(&b, BOUTEILLE);
    assert_eq!((e.detenus, e.consigne_versee, e.valeur_detenus), (36, 5_400, 5_400));
    let f = fournisseur(&b);
    let dette = achats::dette(b.db.conn(), &f).unwrap();
    // Nouvelle livraison : 2 casiers pleins reçus, 1 casier vide et 12 bouteilles vides rendus.
    let g = b.gerant();
    let biere = b.article("Bière blonde 65 cl");
    let achat = |consignes: Vec<ConsigneAchat>| NouvelAchat {
        fournisseur_id: Some(f.clone()),
        mode: "credit".into(),
        compte_id: None,
        lignes: vec![achats::LigneAchatSaisie { article_id: biere.clone(), conditionnement_id: None, quantite: 24, prix_total: 14_400 }],
        consignes,
        note: String::new(),
    };
    let c = vec![
        ConsigneAchat { emballage_id: emballage(&b, BOUTEILLE), recus: 24, rendus: 12 },
        ConsigneAchat { emballage_id: emballage(&b, CASIER), recus: 2, rendus: 1 },
    ];
    achats::receptionner(&mut b.db, &g, &achat(c)).unwrap();
    // Total = 14 400 + (24 − 12) × 150 + (2 − 1) × 2 500 = 18 700.
    assert_eq!(achats::dette(b.db.conn(), &f).unwrap(), dette + 18_700);
    assert_eq!(etat(&b, BOUTEILLE).detenus, 48);
    assert_eq!(etat(&b, CASIER).detenus, 4);
    assert_eq!(consignes::consignes_par_fournisseur(b.db.conn()).unwrap()[0].2, 5_400 + 7_500 + 1_800 + 2_500);
    // Plus de vides rendus que détenus : refusé, rien n'est enregistré.
    let trop = vec![ConsigneAchat { emballage_id: emballage(&b, CASIER), recus: 0, rendus: 10 }];
    assert_eq!(achats::receptionner(&mut b.db, &g, &achat(trop)).unwrap_err().regle_code(), Some("RG-CON-02"));
    assert_eq!(etat(&b, CASIER).detenus, 4);
    // Un achat ne peut pas devenir négatif.
    let mut sans = achat(vec![ConsigneAchat { emballage_id: emballage(&b, CASIER), recus: 0, rendus: 4 }]);
    sans.lignes[0].prix_total = 1_000;
    assert_eq!(achats::receptionner(&mut b.db, &g, &sans).unwrap_err().regle_code(), Some("RG-CON-02"));
}

#[test]
fn rg_con_03_vides_apres_la_vente() {
    let mut b = banc();
    b.ouvrir_journee();
    assert_eq!(etat(&b, BOUTEILLE).vides, Some(0));
    b.commande_table("2", &[("Bière blonde", 3)]);
    // La bouteille reste au restaurant : 3 vides, détenus inchangés.
    let e = etat(&b, BOUTEILLE);
    assert_eq!((e.detenus, e.pleins, e.vides), (36, Some(33), Some(3)));
    assert_eq!(etat(&b, CASIER).vides, None, "casier sans article lié");
}

#[test]
fn rg_con_04_casse_et_bouteilles_emportees() {
    let mut b = banc();
    let s = b.serveur();
    let m = |type_: &str, quantite: i64, motif: &str| MouvementEmballage { emballage_id: String::new(), type_: type_.into(), quantite, motif: motif.into() };
    let bouteille = emballage(&b, BOUTEILLE);
    let avec = |mut x: MouvementEmballage| {
        x.emballage_id = bouteille.clone();
        x
    };
    let g = b.gerant();
    assert_eq!(consignes::mouvement(&mut b.db, &g, &avec(m("casse", 2, " "))).unwrap_err().regle_code(), Some("RG-CON-04"));
    assert_eq!(consignes::mouvement(&mut b.db, &g, &avec(m("reception", 2, "x"))).unwrap_err().regle_code(), Some("RG-CON-04"));
    assert!(consignes::mouvement(&mut b.db, &s, &avec(m("casse", 1, "Tombée"))).is_err(), "le serveur n'a pas le droit");
    consignes::mouvement(&mut b.db, &g, &avec(m("casse", 2, "Casier tombé"))).unwrap();
    consignes::mouvement(&mut b.db, &g, &avec(m("sortie_client", 3, "Emportées par un client"))).unwrap();
    consignes::mouvement(&mut b.db, &g, &avec(m("retour_client", 1, "Rapportée"))).unwrap();
    assert_eq!(etat(&b, BOUTEILLE).detenus, 36 - 2 - 3 + 1);
    // Journal en ajout seul.
    assert!(b.db.conn().execute("UPDATE mouvements_emballages SET quantite = 0", []).is_err());
    assert!(b.db.conn().execute("DELETE FROM mouvements_emballages", []).is_err());
}

#[test]
fn rg_con_05_inventaire_des_vides() {
    let mut b = banc();
    b.ouvrir_journee();
    b.commande_table("2", &[("Bière blonde", 4)]);
    let bouteille = emballage(&b, BOUTEILLE);
    let s = b.serveur();
    assert!(consignes::inventaire(&mut b.db, &s, &bouteille, 3).is_err());
    let g = b.gerant();
    // 4 vides attendus, 3 comptés : une bouteille manque.
    assert_eq!(consignes::inventaire(&mut b.db, &g, &bouteille, 3).unwrap(), -1);
    let e = etat(&b, BOUTEILLE);
    assert_eq!((e.detenus, e.vides), (35, Some(3)));
    // Casier (sans article lié) : on compte tous les casiers détenus.
    let casier = emballage(&b, CASIER);
    assert_eq!(consignes::inventaire(&mut b.db, &g, &casier, 5).unwrap(), 2);
    assert_eq!(consignes::inventaire(&mut b.db, &g, &casier, 5).unwrap(), 0);
}

#[test]
fn rg_con_06_retour_de_vides_hors_livraison() {
    let mut b = banc();
    b.ouvrir_journee();
    b.ouvrir_caisse(0);
    let f = fournisseur(&b);
    let casier = emballage(&b, CASIER);
    let c = b.caissier();
    let r = |quantite: i64, remboursement: &str| RetourFournisseur {
        fournisseur_id: f.clone(),
        emballage_id: casier.clone(),
        quantite,
        remboursement: remboursement.into(),
        compte_id: None,
        note: String::new(),
    };
    let especes = b.solde("Caisse principale");
    let g = b.gerant();
    // Le gérant n'a pas de session de caisse : il faut un compte ou une session.
    assert!(consignes::retour_fournisseur(&mut b.db, &g, &r(1, "especes")).is_err());
    // La caissière encaisse le remboursement, avec l'accord du gérant (achat.gerer).
    assert_eq!(code(&consignes::retour_fournisseur(&mut b.db, &c, &r(1, "especes")).unwrap_err()), "AUTORISATION_REQUISE");
    consignes::retour_fournisseur(&mut b.db, &Banc::avec_pin_gerant(c), &r(1, "especes")).unwrap();
    assert_eq!(b.solde("Caisse principale"), especes + 2_500);
    let dette = achats::dette(b.db.conn(), &f).unwrap();
    consignes::retour_fournisseur(&mut b.db, &g, &r(1, "dette")).unwrap();
    assert_eq!(achats::dette(b.db.conn(), &f).unwrap(), dette - 2_500);
    assert_eq!(etat(&b, CASIER).detenus, 1);
    // Plus que ce que le fournisseur détient en consigne : refusé.
    assert_eq!(consignes::retour_fournisseur(&mut b.db, &g, &r(2, "dette")).unwrap_err().regle_code(), Some("RG-CON-06"));
    assert_eq!(consignes::historique(b.db.conn(), &casier).unwrap().len(), 3);
}
