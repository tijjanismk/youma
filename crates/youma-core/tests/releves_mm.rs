//! Rapprochement Mobile Money par relevé d'opérateur (fiche 0016, RG-RMM-01 à 05).

mod commun;

use commun::*;
use youma_core::caisse::{self, Encaissement};
use youma_core::releves_mm;

/// Table servie et payée en Orange Money avec la référence donnée ; renvoie le montant.
fn payer_mm(b: &mut Banc, table: &str, produit: &str, reference: &str) -> i64 {
    let id = b.commande_table(table, &[(produit, 1)]);
    let total = b.total(&id).reste;
    let om = b.compte("Orange Money");
    let c = b.caissier();
    caisse::encaisser(&mut b.db, &c, &Encaissement { commande_id: id, parts: vec![mobile_money(&om, total, reference)], especes_recues: None }).unwrap();
    total
}

fn statut(b: &Banc, reference: &str) -> String {
    caisse::parts_mobile_money(b.db.conn(), None).unwrap().into_iter().find(|p| p.reference.as_deref() == Some(reference)).unwrap().statut
}

#[test]
fn rg_rmm_01_a_04_rapprochement() {
    let mut b = banc();
    b.ouvrir_journee();
    b.ouvrir_caisse(0);
    let m1 = payer_mm(&mut b, "1", "Poulet braisé", "PP.A1");
    payer_mm(&mut b, "2", "Coca-Cola", "PP.B2");
    payer_mm(&mut b, "3", "Bière blonde", "PP.C3");
    let om = b.compte("Orange Money");
    // A1 conforme, B2 montant différent, X9 inconnu en caisse, C3 absent du relevé.
    let csv = format!(
        "Date;Référence;Expéditeur;Montant\n14/03/2026 10:05;pp.a1;70112233;{m1}\n14/03/2026 10:06;PP.B2;70112233;9 999\n14/03/2026 10:07;PP.X9;76000000;2 000\n"
    );
    let s = b.serveur();
    assert!(releves_mm::importer(&mut b.db, &s, &om, "releve.csv", &csv).is_err(), "le serveur ne vérifie pas");
    let caisse_principale = b.compte("Caisse principale");
    let c = Banc::avec_pin_gerant(b.caissier());
    assert_eq!(releves_mm::importer(&mut b.db, &c, &caisse_principale, "x.csv", &csv).unwrap_err().regle_code(), Some("RG-RMM-01"));

    let bilan = releves_mm::importer(&mut b.db, &c, &om, "releve.csv", &csv).unwrap();
    assert_eq!((bilan.lignes_lues, bilan.verifies, bilan.deja_importees), (3, 1, 0));
    assert_eq!(bilan.ecarts.len(), 1);
    assert_eq!((bilan.ecarts[0].reference.as_str(), bilan.ecarts[0].montant_releve), ("PP.B2", 9_999));
    assert_eq!(bilan.inconnues.iter().map(|l| l.reference.as_str()).collect::<Vec<_>>(), ["PP.X9"]);
    assert_eq!(bilan.absents.iter().map(|p| p.reference.clone().unwrap()).collect::<Vec<_>>(), ["PP.C3"]);
    assert_eq!(statut(&b, "PP.A1"), "verifie");
    assert_eq!(statut(&b, "PP.B2"), "a_verifier");

    // Le même relevé rechargé : rien n'est compté deux fois.
    let bis = releves_mm::importer(&mut b.db, &c, &om, "releve.csv", &csv).unwrap();
    assert_eq!((bis.deja_importees, bis.verifies), (3, 0));
    assert_eq!(b.compter("SELECT COUNT(*) FROM lignes_releve_mm"), 3);
    assert_eq!(releves_mm::lister(b.db.conn()).unwrap().len(), 2);
    // Journal en ajout seul.
    assert!(b.db.conn().execute("UPDATE lignes_releve_mm SET montant = 0", []).is_err());
    assert!(b.db.conn().execute("DELETE FROM releves_mm", []).is_err());
}

#[test]
fn rg_rmm_05_relancer_apres_saisie_tardive() {
    let mut b = banc();
    b.ouvrir_journee();
    b.ouvrir_caisse(0);
    let om = b.compte("Orange Money");
    let c = Banc::avec_pin_gerant(b.caissier());
    // Paiement reçu sur le téléphone mais saisi après l'import du relevé.
    let prix: i64 = b.db.conn().query_row("SELECT prix FROM produits WHERE nom = 'Coca-Cola'", [], |r| r.get(0)).unwrap();
    let csv = format!("Transaction ID,Amount,Date\nLATE1,{prix},2026-03-14 10:00:00\n");
    let bilan = releves_mm::importer(&mut b.db, &c, &om, "wave.csv", &csv).unwrap();
    assert_eq!(bilan.inconnues.len(), 1);
    payer_mm(&mut b, "4", "Coca-Cola", "LATE1");
    assert_eq!(statut(&b, "LATE1"), "a_verifier");
    assert_eq!(releves_mm::relancer(&mut b.db, &c, &om).unwrap(), 1);
    assert_eq!(statut(&b, "LATE1"), "verifie");
    assert_eq!(releves_mm::relancer(&mut b.db, &c, &om).unwrap(), 0);
}
