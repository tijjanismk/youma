//! RG-CAI-15 : paiement par carte sur un TPE non relié à Youma.

mod commun;

use commun::*;
use youma_core::caisse::{self, Encaissement, PartSaisie};

fn carte(compte_id: &str, montant: i64, autorisation: &str) -> PartSaisie {
    PartSaisie {
        moyen: "carte".into(),
        montant,
        compte_id: Some(compte_id.into()),
        reference: Some(autorisation.into()),
        numero_payeur: None,
        client_id: None,
        par_livreur: false,
    }
}

#[test]
fn carte_sur_tpe_numero_d_autorisation_une_fois_par_jour() {
    let mut b = banc();
    b.ouvrir_journee();
    b.ouvrir_caisse(10_000);
    let banque = b.compte("Banque (TPE)");
    let om = b.compte("Orange Money");
    let a = b.caissier();
    let payer = |b: &mut Banc, commande: &str, part: PartSaisie| {
        caisse::encaisser(&mut b.db, &a, &Encaissement { commande_id: commande.into(), parts: vec![part], especes_recues: None })
    };
    let c1 = b.commande_table("1", &[("Coca-Cola", 2)]);
    // Numéro d'autorisation obligatoire.
    assert_eq!(payer(&mut b, &c1, carte(&banque, 1_500, "  ")).unwrap_err().regle_code(), Some("RG-CAI-15"));
    // Une carte ne va que sur un compte bancaire.
    assert!(payer(&mut b, &c1, carte(&om, 1_500, "A12345")).is_err());
    let r = payer(&mut b, &c1, carte(&banque, 1_500, "A12345")).unwrap();
    assert!(r.commande_payee);
    assert_eq!(b.solde("Banque (TPE)"), 1_500);
    assert_eq!(b.solde("Caisse principale"), 10_000, "la carte n'entre pas dans le tiroir");
    // Le même ticket de TPE ne paie pas une deuxième addition le même jour.
    let c2 = b.commande_table("2", &[("Coca-Cola", 2)]);
    assert_eq!(payer(&mut b, &c2, carte(&banque, 1_500, "A12345")).unwrap_err().regle_code(), Some("RG-CAI-15"));
    assert_eq!(b.total(&c2).paye, 0);
    payer(&mut b, &c2, carte(&banque, 1_500, "B67890")).unwrap();
}
