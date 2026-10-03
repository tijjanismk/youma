//! Fiche 0039 : fidélité par points, cartes cadeaux et bons d'avoir, contrats société, clients privilégiés.
mod commun;

use commun::*;
use youma_core::caisse::{self, Encaissement, PartSaisie};
use youma_core::{cartes, commandes, contrats, fidelite, parametres};

fn part(moyen: &str, montant: i64) -> PartSaisie {
    PartSaisie { moyen: moyen.into(), ..especes(montant) }
}

/// RG-FID-01 à 04 : points gagnés sur l'addition soldée, utilisés en remise, retirés si le paiement est annulé.
#[test]
fn rg_fid_points_gagnes_utilises_annules() {
    let mut b = banc();
    b.ouvrir_journee();
    b.ouvrir_caisse(10_000);
    let c = b.caissier();
    let gp = Banc::avec_pin_gerant(b.caissier());
    let modibo = b.client("Modibo Diallo");
    // 25 bières à 1 000 FCFA : 25 points (1 point par 1 000 FCFA, démo).
    let cmd = b.commande_table("1", &[("Bière blonde", 25)]);
    commandes::definir_client(&mut b.db, &c, &cmd, Some(&modibo)).unwrap();
    let r = b.payer_especes(&cmd, 25_000);
    assert!(r.commande_payee);
    assert_eq!(fidelite::solde(b.db.conn(), &modibo).unwrap(), 25);

    // Sans client sur l'addition : aucun point.
    let anonyme = b.commande_table("2", &[("Bière blonde", 3)]);
    b.payer_especes(&anonyme, 3_000);
    assert_eq!(b.compter("SELECT COUNT(*) FROM mouvements_fidelite WHERE type = 'gain'"), 1);

    // Utilisation : 20 points (minimum) = 1 000 FCFA de remise ; pas plus que le solde ni sous le minimum.
    let cmd2 = b.commande_table("3", &[("Bière blonde", 4)]);
    assert_eq!(fidelite::utiliser(&mut b.db, &c, &cmd2, 5).unwrap_err().regle_code(), Some("RG-FID-03"), "sans client");
    commandes::definir_client(&mut b.db, &c, &cmd2, Some(&modibo)).unwrap();
    assert_eq!(fidelite::utiliser(&mut b.db, &c, &cmd2, 10).unwrap_err().regle_code(), Some("RG-FID-03"), "sous le minimum");
    assert_eq!(fidelite::utiliser(&mut b.db, &c, &cmd2, 30).unwrap_err().regle_code(), Some("RG-FID-03"), "plus que le solde");
    assert_eq!(fidelite::utiliser(&mut b.db, &c, &cmd2, 20).unwrap(), 1_000);
    assert_eq!(b.total(&cmd2).total, 3_000);
    assert_eq!(fidelite::solde(b.db.conn(), &modibo).unwrap(), 5);
    // Payée : 3 000 FCFA → 3 points de plus.
    let r = b.payer_especes(&cmd2, 3_000);
    assert_eq!(fidelite::solde(b.db.conn(), &modibo).unwrap(), 8);

    // RG-FID-04 : paiement annulé → les points de cette addition sont retirés ; repayée → regagnés, sans doublon.
    let g = gp.clone();
    caisse::annuler_paiement(&mut b.db, &g, &r.paiement_id, "Erreur de table").unwrap();
    assert_eq!(fidelite::solde(b.db.conn(), &modibo).unwrap(), 5);
    b.payer_especes(&cmd2, 3_000);
    assert_eq!(fidelite::solde(b.db.conn(), &modibo).unwrap(), 8);
    // Ajout seul.
    assert!(b.db.conn().execute("DELETE FROM mouvements_fidelite", []).is_err());

    // Fidélité coupée : plus de gain.
    let mut p = parametres::lire(b.db.conn()).unwrap();
    p.fidelite.active = false;
    parametres::ecrire(b.db.conn(), &p).unwrap();
    let cmd3 = b.commande_table("4", &[("Bière blonde", 5)]);
    commandes::definir_client(&mut b.db, &c, &cmd3, Some(&modibo)).unwrap();
    b.payer_especes(&cmd3, 5_000);
    assert_eq!(fidelite::solde(b.db.conn(), &modibo).unwrap(), 8);
}

/// RG-CAD-01 à 05 : carte vendue (argent en caisse, pas de chiffre d'affaires), paiement partiel, annulation, bon d'avoir.
#[test]
fn rg_cad_cartes_cadeaux_et_bons_d_avoir() {
    let mut b = banc();
    let j = b.ouvrir_journee();
    b.ouvrir_caisse(10_000);
    let c = b.caissier();
    let gp = Banc::avec_pin_gerant(b.caissier());
    let vente = cartes::VenteCarte {
        montant: 5_000,
        moyen: "especes".into(),
        compte_id: None,
        reference: String::new(),
        client_id: None,
        beneficiaire: "Awa (anniversaire)".into(),
        expire_le: None,
    };
    let carte = cartes::vendre(&mut b.db, &c, &vente).unwrap();
    assert_eq!(carte.solde, 5_000);
    assert_eq!(carte.code.len(), 9, "code lisible AAAA-BBBB");
    assert_eq!(b.solde("Caisse principale"), 15_000, "l'argent de la carte est dans le tiroir");
    let ch = youma_core::rapports::chiffres(b.db.conn(), &j.date_exploitation, &j.date_exploitation).unwrap();
    assert_eq!(ch.ca, 0, "une carte vendue n'est pas une vente du jour");

    // Paiement de 3 000 FCFA par la carte (code saisi en minuscules, sans tiret) : pas de mouvement de caisse.
    let cmd = b.commande_table("1", &[("Bière blonde", 3)]);
    let saisi = carte.code.replace('-', "").to_lowercase();
    let e = Encaissement { commande_id: cmd.clone(), parts: vec![PartSaisie { reference: Some(saisi.clone()), ..part("carte_cadeau", 3_000) }], especes_recues: None };
    let r = caisse::encaisser(&mut b.db, &c, &e).unwrap();
    assert!(r.commande_payee);
    assert_eq!(cartes::par_code(b.db.conn(), &carte.code).unwrap().solde, 2_000);
    assert_eq!(b.solde("Caisse principale"), 15_000);
    // Solde insuffisant ; code inconnu.
    let cmd2 = b.commande_table("2", &[("Bière blonde", 3)]);
    let trop = Encaissement { commande_id: cmd2.clone(), parts: vec![PartSaisie { reference: Some(saisi.clone()), ..part("carte_cadeau", 3_000) }], especes_recues: None };
    assert_eq!(caisse::encaisser(&mut b.db, &c, &trop).unwrap_err().regle_code(), Some("RG-CAD-02"));
    let inconnu = Encaissement { commande_id: cmd2.clone(), parts: vec![PartSaisie { reference: Some("ZZZZ-ZZZZ".into()), ..part("carte_cadeau", 1_000) }], especes_recues: None };
    assert_eq!(caisse::encaisser(&mut b.db, &c, &inconnu).unwrap_err().regle_code(), Some("RG-CAD-02"));

    // RG-CAD-05 : paiement annulé, la carte retrouve son solde.
    caisse::annuler_paiement(&mut b.db, &gp, &r.paiement_id, "Mauvaise addition").unwrap();
    assert_eq!(cartes::par_code(b.db.conn(), &carte.code).unwrap().solde, 5_000);

    // RG-CAD-04 : bon d'avoir offert au client mécontent : accord du gérant, motif obligatoire, aucun argent.
    let modibo = b.client("Modibo Diallo");
    let avoir = cartes::NouvelAvoir { montant: 2_000, client_id: Some(modibo.clone()), beneficiaire: String::new(), motif: String::new(), expire_le: None };
    assert_eq!(cartes::offrir_avoir(&mut b.db, &gp, &avoir).unwrap_err().regle_code(), Some("RG-CAD-04"));
    let avoir = cartes::NouvelAvoir { motif: "Plat arrivé froid".into(), ..avoir };
    assert_eq!(youma_core::Erreur::code(&cartes::offrir_avoir(&mut b.db, &c, &avoir).unwrap_err()), "AUTORISATION_REQUISE");
    let bon = cartes::offrir_avoir(&mut b.db, &gp, &avoir).unwrap();
    assert_eq!((bon.type_.as_str(), bon.solde), ("avoir", 2_000));
    assert_eq!(b.solde("Caisse principale"), 15_000);
    assert_eq!(cartes::du_client(b.db.conn(), &modibo).unwrap().len(), 1);

    // RG-CAD-03 : carte expirée refusée (date passée refusée à la création).
    let mut expiree = cartes::VenteCarte { expire_le: Some("2020-01-01".into()), ..vente };
    assert_eq!(cartes::vendre(&mut b.db, &c, &expiree).unwrap_err().regle_code(), Some("RG-CAD-01"));
    expiree.expire_le = Some(j.date_exploitation.clone());
    let k = cartes::vendre(&mut b.db, &c, &expiree).unwrap();
    b.horloge.avancer_minutes(2 * 24 * 60);
    let cmd3 = b.commande_table("3", &[("Bière blonde", 1)]);
    let e = Encaissement { commande_id: cmd3, parts: vec![PartSaisie { reference: Some(k.code.clone()), ..part("carte_cadeau", 1_000) }], especes_recues: None };
    assert_eq!(caisse::encaisser(&mut b.db, &c, &e).unwrap_err().regle_code(), Some("RG-CAD-03"));
}

/// RG-SOC-01 à 04 : la société paie sa part (50 %, 2 500 FCFA au plus) sur son compte, l'employé le reste.
#[test]
fn rg_soc_contrat_societe() {
    let mut b = banc();
    let j = b.ouvrir_journee();
    b.ouvrir_caisse(10_000);
    let c = b.caissier();
    let ger = b.gerant();
    let k = contrats::lister(b.db.conn()).unwrap().into_iter().next().unwrap();
    assert_eq!(contrats::part_societe(&k, 4_000), 2_000);
    assert_eq!(contrats::part_societe(&k, 8_000), 2_500, "plafond par repas");

    let cmd = b.commande_table("1", &[("Bière blonde", 4)]);
    let societe = |montant: i64, employe: &str| PartSaisie {
        contrat_id: Some(k.id.clone()),
        reference: Some(employe.into()),
        ..part("credit", montant)
    };
    // Employé non nommé, part trop grosse : refusés.
    let e = Encaissement { commande_id: cmd.clone(), parts: vec![societe(2_000, " ")], especes_recues: None };
    assert_eq!(caisse::encaisser(&mut b.db, &c, &e).unwrap_err().regle_code(), Some("RG-SOC-02"));
    let e = Encaissement { commande_id: cmd.clone(), parts: vec![societe(2_500, "Moussa Traoré")], especes_recues: None };
    assert_eq!(caisse::encaisser(&mut b.db, &c, &e).unwrap_err().regle_code(), Some("RG-SOC-02"));
    // La part d'une société n'est jamais en espèces.
    let e = Encaissement { commande_id: cmd.clone(), parts: vec![PartSaisie { contrat_id: Some(k.id.clone()), ..especes(2_000) }], especes_recues: None };
    assert_eq!(caisse::encaisser(&mut b.db, &c, &e).unwrap_err().regle_code(), Some("RG-SOC-02"));

    let e = Encaissement { commande_id: cmd.clone(), parts: vec![societe(2_000, "Moussa Traoré"), especes(2_000)], especes_recues: Some(2_000) };
    assert!(caisse::encaisser(&mut b.db, &c, &e).unwrap().commande_payee);
    assert_eq!(youma_core::clients::dette(b.db.conn(), &k.client_id).unwrap(), 2_000, "la part est sur le compte de la société");
    let releve = contrats::releve(b.db.conn(), &k.id, &j.date_exploitation, &j.date_exploitation).unwrap();
    assert_eq!(releve.len(), 1);
    assert_eq!((releve[0].employe.as_str(), releve[0].total_repas, releve[0].part_societe), ("Moussa Traoré", 4_000, 2_000));

    // RG-SOC-01 : contrat seulement avec une société autorisée à crédit, par un gérant.
    let salif = b.client("Salif Konaté");
    let nouveau = contrats::Contrat { id: String::new(), client_id: salif, client_nom: String::new(), nom: "X".into(), type_prise: "montant".into(), valeur: 1_000, plafond_repas: 0, actif: true, dette: 0 };
    assert_eq!(youma_core::Erreur::code(&contrats::enregistrer(&mut b.db, &c, &nouveau).unwrap_err()), "AUTORISATION_REQUISE");
    assert_eq!(contrats::enregistrer(&mut b.db, &ger, &nouveau).unwrap_err().regle_code(), Some("RG-SOC-01"));
    let pct = contrats::Contrat { client_id: k.client_id.clone(), type_prise: "pourcentage".into(), valeur: 120, ..nouveau };
    assert_eq!(contrats::enregistrer(&mut b.db, &ger, &pct).unwrap_err().regle_code(), Some("RG-SOC-01"));
}

/// RG-VIP-01 : client privilégié, sans fin ou jusqu'à une date.
#[test]
fn rg_vip_privilege() {
    let mut b = banc();
    b.ouvrir_journee();
    let salif = b.client("Salif Konaté");
    let (caissier, serveur) = (b.caissier(), b.serveur());
    let aujourdhui = "2026-03-14";
    assert!(!fidelite::est_vip(b.db.conn(), &salif, aujourdhui).unwrap());
    let p = fidelite::Privilege { vip: true, jusqu_au: Some("2026-03-20".into()), motif: "Client mécontent, on le garde".into() };
    assert_eq!(youma_core::Erreur::code(&fidelite::definir_privilege(&mut b.db, &serveur, &salif, &p).unwrap_err()), "AUTORISATION_REQUISE");
    fidelite::definir_privilege(&mut b.db, &caissier, &salif, &p).unwrap();
    assert!(fidelite::est_vip(b.db.conn(), &salif, aujourdhui).unwrap());
    assert!(!fidelite::est_vip(b.db.conn(), &salif, "2026-03-21").unwrap(), "privilège terminé");
    let passe = fidelite::Privilege { vip: true, jusqu_au: Some("2026-01-01".into()), motif: String::new() };
    assert_eq!(fidelite::definir_privilege(&mut b.db, &caissier, &salif, &passe).unwrap_err().regle_code(), Some("RG-VIP-01"));
    let client = youma_core::clients::client(b.db.conn(), &salif).unwrap();
    assert!(client.vip && client.vip_jusqu_au.as_deref() == Some("2026-03-20"));
}
