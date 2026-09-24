//! Scénarios d'acceptation du cahier des charges (section 27).

mod commun;

use commun::*;
use youma_core::caisse::{self, ClotureSession, Encaissement, PartSaisie};
use youma_core::commandes::{self, NouvelleCommande};
use youma_core::employes::{self, Avance};
use youma_core::{achats, auth, impression, journee, livraison, paie, rapports, sauvegarde, stock, Acteur, Db};

/// Scénario 1 (a) : une erreur au milieu d'un encaissement mixte n'écrit rien.
#[test]
fn s01_encaissement_mixte_tout_ou_rien() {
    let mut b = banc();
    b.ouvrir_journee();
    b.ouvrir_caisse(10_000);
    let om = b.compte("Orange Money");
    // Première commande payée avec la référence OM-1.
    let c1 = b.commande_table("1", &[("Coca-Cola", 2)]);
    let a = b.caissier();
    caisse::encaisser(&mut b.db, &a, &Encaissement { commande_id: c1, parts: vec![mobile_money(&om, 1_500, "OM-1")], especes_recues: None }).unwrap();
    let caisse_avant = b.solde("Caisse principale");
    let om_avant = b.solde("Orange Money");
    // Deuxième : espèces OK puis Mobile Money avec référence déjà utilisée → échec après la 1re part.
    let c2 = b.commande_table("2", &[("Poulet braisé", 2)]);
    let e = caisse::encaisser(&mut b.db, &a, &Encaissement { commande_id: c2.clone(), parts: vec![especes(3_000), mobile_money(&om, 4_000, "OM-1")], especes_recues: None }).unwrap_err();
    assert_eq!(e.regle_code(), Some("RG-CAI-05"));
    assert_eq!(b.solde("Caisse principale"), caisse_avant);
    assert_eq!(b.solde("Orange Money"), om_avant);
    assert_eq!(b.total(&c2).paye, 0);
    assert_eq!(commandes::detail(b.db.conn(), &c2).unwrap().statut, "ouverte");
}

/// Scénario 1 (b) : coupure réelle (processus tué) pendant un encaissement.
#[test]
fn s01_coupure_processus_tue() {
    let mut b = banc();
    b.ouvrir_journee();
    b.ouvrir_caisse(10_000);
    let c = b.commande_table("3", &[("Coca-Cola", 2)]);
    let chemin = b.dossier.path().join("youma.db");
    let caissier = b.demo.caissier.clone();
    drop(b.db);
    let statut = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "enfant_coupure", "--ignored", "--nocapture"])
        .env("YOUMA_COUPURE_DB", &chemin)
        .env("YOUMA_COUPURE_COMMANDE", &c)
        .env("YOUMA_COUPURE_CAISSIER", &caissier)
        .status()
        .unwrap();
    assert!(!statut.success(), "le processus enfant devait être tué");
    let db = Db::ouvrir(&chemin, b.horloge.clone()).unwrap();
    let r = sauvegarde::verifier_integrite(db.conn(), true, 0).unwrap();
    assert!(r.ok, "{:?}", r.messages);
    let paiements: i64 = db.conn().query_row("SELECT COUNT(*) FROM paiements WHERE commande_id = ?1", [&c], |r| r.get(0)).unwrap();
    assert_eq!(paiements, 0, "aucun paiement à moitié enregistré");
    assert_eq!(commandes::detail(db.conn(), &c).unwrap().statut, "ouverte");
}

/// Processus enfant : écrit un paiement puis meurt avant le commit.
#[test]
#[ignore]
fn enfant_coupure() {
    let Ok(chemin) = std::env::var("YOUMA_COUPURE_DB") else { return };
    let commande = std::env::var("YOUMA_COUPURE_COMMANDE").unwrap();
    let caissier = std::env::var("YOUMA_COUPURE_CAISSIER").unwrap();
    let horloge = std::sync::Arc::new(youma_core::horloge::HorlogeFixe::a("2026-03-14", 10, 5));
    let mut db = Db::ouvrir(std::path::Path::new(&chemin), horloge).unwrap();
    let _ = db.executer(&Acteur::utilisateur(&caissier), |op| {
        op.execute(
            "INSERT INTO paiements(id, numero, commande_id, journee_id, montant, horodatage) SELECT 'x', 999, ?1, journee_id, 1500, 0 FROM commandes WHERE id = ?1",
            [&commande],
        )?;
        std::process::abort();
        #[allow(unreachable_code)]
        Ok(())
    });
}

/// Scénario 2 : paiement mixte 5 000 espèces + 7 500 Orange Money.
#[test]
fn s02_paiement_mixte() {
    let mut b = banc();
    b.ouvrir_journee();
    b.ouvrir_caisse(10_000);
    let c = b.commande_table("4", &[("Poulet braisé", 2), ("Riz sauce arachide", 2), ("Bière blonde", 2), ("Eau minérale", 1)]);
    assert_eq!(b.total(&c).total, 12_500);
    let om = b.compte("Orange Money");
    let a = b.caissier();
    let r = caisse::encaisser(&mut b.db, &a, &Encaissement { commande_id: c.clone(), parts: vec![especes(5_000), mobile_money(&om, 7_500, "PP260314.1234.A1")], especes_recues: Some(10_000) }).unwrap();
    assert_eq!(r.rendu, 5_000);
    assert!(r.commande_payee);
    assert_eq!(b.solde("Caisse principale"), 15_000);
    assert_eq!(b.solde("Orange Money"), 7_500);
    let j = journee::ouverte(b.db.conn()).unwrap().unwrap();
    let rap = rapports::rapport_periode(b.db.conn(), &j.date_exploitation, &j.date_exploitation).unwrap();
    let t = rap.tableaux.iter().find(|t| t.titre == "Encaissements par moyen de paiement").unwrap();
    assert!(t.lignes.iter().any(|l| l[0] == "especes" && l[2] == 5_000));
    assert!(t.lignes.iter().any(|l| l[1] == "Orange Money" && l[2] == 7_500));
    // Paiement « à vérifier » listé pour le responsable.
    assert_eq!(caisse::parts_mobile_money(b.db.conn(), Some("a_verifier")).unwrap().len(), 1);
}

/// Scénario 3 : tournées sur une même addition, deux tickets bar et un ticket cuisine (grill).
#[test]
fn s03_tournees() {
    let mut b = banc();
    b.ouvrir_journee();
    let fichier = b.dossier.path().join("imprimante.txt");
    b.db.conn().execute("UPDATE postes_preparation SET imprimante = ?1", [format!("fichier:{}", fichier.display())]).unwrap();
    let c = b.commande_table("4", &[("Bière blonde", 3)]);
    let a = b.serveur();
    let t = b.table("4");
    let c2 = commandes::ouvrir(&mut b.db, &a, &NouvelleCommande { type_: "sur_place".into(), table_id: Some(t.clone()), client_id: None, employe_id: None, couverts: 0, note: String::new(), livraison: None, canal: None }).unwrap();
    assert_eq!(c, c2, "RG-CMD-01 : même addition");
    let l = b.ligne("Bière blonde", 2);
    commandes::ajouter_lignes(&mut b.db, &a, &c, &[l]).unwrap();
    commandes::envoyer(&mut b.db, &a, &c).unwrap();
    let l = b.ligne("Brochettes (3)", 2);
    commandes::ajouter_lignes(&mut b.db, &a, &c, &[l]).unwrap();
    commandes::envoyer(&mut b.db, &a, &c).unwrap();
    let d = commandes::detail(b.db.conn(), &c).unwrap();
    let postes: Vec<_> = d.envois.iter().map(|e| e.poste_nom.clone().unwrap()).collect();
    assert_eq!(postes.iter().filter(|p| *p == "Bar").count(), 2);
    assert_eq!(postes.iter().filter(|p| *p == "Grill").count(), 1);
    assert_eq!(d.totaux.total, 5 * 1_000 + 2 * 1_500);
    let jobs = impression::jobs(b.db.conn(), &[]).unwrap();
    assert_eq!(jobs.len(), 3);
    assert!(jobs.iter().any(|j| j.contenu.contains("Envoi n°3") && j.contenu.contains("2 × Brochettes (3)")));
    assert_eq!(b.compter("SELECT COUNT(*) FROM commandes WHERE statut = 'ouverte'"), 1);
}

/// Scénario 4 : une vente à 1 h 30 appartient à la journée de la veille.
#[test]
fn s04_apres_minuit() {
    let mut b = banc();
    b.horloge.regler_a("2026-03-14", 18, 0);
    let j = b.ouvrir_journee();
    assert_eq!(j.date_exploitation, "2026-03-14");
    b.horloge.regler_a("2026-03-15", 1, 30);
    let c = b.commande_table("5", &[("Bière blonde", 1)]);
    let d = commandes::detail(b.db.conn(), &c).unwrap();
    assert_eq!(d.journee_id, j.id);
}

/// Scénario 5 : horloge remise au 01/01/2000 → ventes bloquées jusqu'à correction par un responsable.
#[test]
fn s05_horloge_remise_a_zero() {
    let mut b = banc();
    b.ouvrir_journee();
    b.ouvrir_caisse(0);
    b.horloge.regler_a("2000-01-01", 0, 0);
    assert!(!b.db.etat_horloge().unwrap().coherente);
    let a = b.serveur();
    let t = b.table("6");
    let e = commandes::ouvrir(&mut b.db, &a, &NouvelleCommande { type_: "sur_place".into(), table_id: Some(t.clone()), client_id: None, employe_id: None, couverts: 0, note: String::new(), livraison: None, canal: None }).unwrap_err();
    assert_eq!(e.code(), "HORLOGE_INCOHERENTE");
    // Le serveur ne peut pas accepter la nouvelle heure.
    let s = b.serveur();
    assert_eq!(auth::accepter_heure(&mut b.db, &s).unwrap_err().code(), "AUTORISATION_REQUISE");
    // Le PC est corrigé : tout repart, sans intervention.
    b.horloge.regler_a("2026-03-14", 10, 30);
    commandes::ouvrir(&mut b.db, &a, &NouvelleCommande { type_: "sur_place".into(), table_id: Some(t), client_id: None, employe_id: None, couverts: 0, note: String::new(), livraison: None, canal: None }).unwrap();
    // Si la date antérieure est la bonne (PC en avance auparavant), le propriétaire l'accepte et c'est journalisé.
    b.horloge.regler_a("2026-03-13", 9, 0);
    let p = b.proprietaire();
    auth::accepter_heure(&mut b.db, &p).unwrap();
    assert!(b.db.etat_horloge().unwrap().coherente);
    assert_eq!(b.compter("SELECT COUNT(*) FROM journal_audit WHERE action = 'horloge.accepter'"), 1);
}

/// Scénario 6 : annulation d'un article déjà envoyé.
#[test]
fn s06_annulation_apres_envoi() {
    let mut b = banc();
    b.ouvrir_journee();
    let fichier = b.dossier.path().join("grill.txt");
    b.db.conn().execute("UPDATE postes_preparation SET imprimante = ?1 WHERE nom = 'Grill'", [format!("fichier:{}", fichier.display())]).unwrap();
    let c = b.commande_table("7", &[("Poulet braisé", 1)]);
    let ligne = commandes::detail(b.db.conn(), &c).unwrap().lignes[0].id.clone();
    let s = b.serveur();
    // Sans motif : refusé.
    let e = commandes::annuler_ligne(&mut b.db, &s, &ligne, 1, "", false).unwrap_err();
    assert_eq!(e.regle_code(), Some("RG-CMD-04"));
    // Sans PIN du gérant : autorisation requise.
    let e = commandes::annuler_ligne(&mut b.db, &s, &ligne, 1, "Client parti", false).unwrap_err();
    assert_eq!(e.code(), "AUTORISATION_REQUISE");
    // Avec le PIN du gérant.
    commandes::annuler_ligne(&mut b.db, &Banc::avec_pin_gerant(s), &ligne, 1, "Client parti", true).unwrap();
    assert_eq!(b.total(&c).total, 0);
    let (par, autorise): (String, String) = b
        .db
        .conn()
        .query_row("SELECT utilisateur_id, autorise_par FROM annulations", [], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap();
    assert_eq!(par, b.demo.serveur);
    assert_eq!(autorise, b.demo.gerant);
    assert_eq!(b.compter("SELECT COUNT(*) FROM journal_audit WHERE action = 'commande.annuler_article' AND autorise_par IS NOT NULL"), 1);
    let jobs = impression::jobs(b.db.conn(), &[]).unwrap();
    assert!(jobs.iter().any(|j| j.type_ == "annulation" && j.contenu.contains("ANNULATION") && j.contenu.contains("Client parti")));
    let jour = journee::ouverte(b.db.conn()).unwrap().unwrap().date_exploitation;
    let r = rapports::rapport_periode(b.db.conn(), &jour, &jour).unwrap();
    let t = r.tableaux.iter().find(|t| t.titre == "Annulations après envoi").unwrap();
    assert_eq!(t.lignes.len(), 1);
    assert_eq!(t.lignes[0][4], "Client parti");
}

/// Scénario 7 : crédit refusé au-delà de la limite, sauf autorisation du gérant.
#[test]
fn s07_credit_refuse() {
    let mut b = banc();
    b.ouvrir_journee();
    b.ouvrir_caisse(0);
    let modibo = b.client("Modibo Diallo");
    // Dette actuelle 18 000.
    let c1 = b.commande_table("1", &[("Poisson braisé", 4), ("Riz sauce arachide", 1), ("Alloco", 1)]);
    assert_eq!(b.total(&c1).total, 18_000);
    let a = b.caissier();
    caisse::encaisser(&mut b.db, &a, &Encaissement { commande_id: c1, parts: vec![credit(&modibo, 18_000)], especes_recues: None }).unwrap();
    assert_eq!(youma_core::clients::dette(b.db.conn(), &modibo).unwrap(), 18_000);
    // Nouvelle commande de 5 000 → refus.
    let c2 = b.commande_table("2", &[("Poisson braisé", 1), ("Riz sauce arachide", 1)]);
    let e = caisse::encaisser(&mut b.db, &a, &Encaissement { commande_id: c2.clone(), parts: vec![credit(&modibo, 5_500)], especes_recues: None }).unwrap_err();
    assert_eq!(e.code(), "AUTORISATION_REQUISE");
    // Le caissier ne peut pas se l'autoriser : le gérant, si.
    caisse::encaisser(&mut b.db, &Banc::avec_pin_gerant(a), &Encaissement { commande_id: c2, parts: vec![credit(&modibo, 5_500)], especes_recues: None }).unwrap();
    assert_eq!(youma_core::clients::dette(b.db.conn(), &modibo).unwrap(), 23_500);
    assert_eq!(b.compter("SELECT COUNT(*) FROM journal_audit WHERE action = 'client.depassement_limite'"), 1);
    // Client sans crédit autorisé : refus sec (RG-CLI-01).
    let salif = b.client("Salif Konaté");
    let c3 = b.commande_table("3", &[("Alloco", 1)]);
    let a = b.caissier();
    let e = caisse::encaisser(&mut b.db, &a, &Encaissement { commande_id: c3, parts: vec![credit(&salif, 500)], especes_recues: None }).unwrap_err();
    assert_eq!(e.regle_code(), Some("RG-CLI-01"));
}

/// Scénario 8 : avance supérieure au salaire → net négatif reporté.
#[test]
fn s08_avance_superieure_au_salaire() {
    let mut b = banc();
    b.ouvrir_journee();
    b.ouvrir_caisse(100_000);
    let awa = b.employe("Awa Traoré");
    let g = b.gerant();
    // Plafond 50 % de 50 000 = 25 000 : 60 000 exige l'autorisation du propriétaire.
    let caisse_id = b.compte("Caisse principale");
    let e = employes::avance(&mut b.db, &g, &Avance { employe_id: awa.clone(), montant: 60_000, compte_id: Some(caisse_id.clone()), motif: "Maladie".into() }).unwrap_err();
    assert_eq!(e.code(), "AUTORISATION_REQUISE");
    let g_autorise = g.clone().avec_autorisation(Some("1234".into()));
    employes::avance(&mut b.db, &g_autorise, &Avance { employe_id: awa.clone(), montant: 60_000, compte_id: Some(caisse_id), motif: "Maladie".into() }).unwrap();
    assert_eq!(b.solde("Caisse principale"), 40_000);
    let bul = paie::cloturer(&mut b.db, &g, &awa, "2026-03-01", "2026-03-31").unwrap();
    assert_eq!(bul.net_a_payer, -10_000);
    // Mois suivant : le report est déduit, rien n'est perdu.
    let bul2 = paie::cloturer(&mut b.db, &g, &awa, "2026-04-01", "2026-04-30").unwrap();
    assert_eq!(bul2.report_precedent, -10_000);
    assert_eq!(bul2.net_a_payer, 40_000);
}

/// Scénario 9 : consommation d'un employé : sortie de stock, pas de CA, retenue en fin de mois.
#[test]
fn s09_consommation_employe() {
    let mut b = banc();
    let j = b.ouvrir_journee();
    let awa = b.employe("Awa Traoré");
    let coca_avant = b.stock("Coca-Cola 33 cl");
    let g = b.gerant();
    let c = commandes::ouvrir(&mut b.db, &g, &NouvelleCommande { type_: "comptoir".into(), table_id: None, client_id: None, employe_id: Some(awa.clone()), couverts: 0, note: String::new(), livraison: None, canal: None }).unwrap();
    let l = [b.ligne("Coca-Cola", 1), b.ligne("Riz sauce arachide", 1)];
    commandes::ajouter_lignes(&mut b.db, &g, &c, &l).unwrap();
    // L'encaissement classique est refusé : c'est une imputation.
    let a = b.caissier();
    assert!(caisse::encaisser(&mut b.db, &a, &Encaissement { commande_id: c.clone(), parts: vec![especes(2_250)], especes_recues: None }).is_err());
    let montant = employes::imputer_commande(&mut b.db, &g, &c).unwrap();
    assert_eq!(montant, 2_250);
    assert_eq!(b.stock("Coca-Cola 33 cl"), coca_avant - 1);
    let ch = rapports::chiffres(b.db.conn(), &j.date_exploitation, &j.date_exploitation).unwrap();
    assert_eq!(ch.ca, 0);
    let bul = paie::cloturer(&mut b.db, &g, &awa, "2026-03-01", "2026-03-31").unwrap();
    assert!(bul.lignes.iter().any(|l| l.type_ == "consommation" && l.montant == -2_250));
    assert_eq!(bul.net_a_payer, 50_000 - 2_250);
}

/// Scénario 10 : 2 casiers de 24 achetés au marché, payés en espèces.
#[test]
fn s10_reception_boissons() {
    let mut b = banc();
    b.ouvrir_journee();
    b.ouvrir_caisse(30_000);
    let coca = b.article("Coca-Cola 33 cl");
    let casier: String = b.db.conn().query_row("SELECT id FROM conditionnements WHERE article_id = ?1", [&coca], |r| r.get(0)).unwrap();
    let avant = b.stock("Coca-Cola 33 cl");
    let g = b.gerant();
    // Le gérant n'a pas de session : il paie depuis la caisse principale explicitement.
    let caisse_id = b.compte("Caisse principale");
    achats::receptionner(
        &mut b.db,
        &g,
        &achats::NouvelAchat { fournisseur_id: None, mode: "comptant".into(), compte_id: Some(caisse_id), lignes: vec![achats::LigneAchatSaisie { article_id: coca.clone(), conditionnement_id: Some(casier), quantite: 2, prix_total: 26_400 }], consignes: vec![], note: "Marché".into() },
    )
    .unwrap();
    assert_eq!(b.stock("Coca-Cola 33 cl"), avant + 48);
    assert_eq!(b.solde("Caisse principale"), 30_000 - 26_400);
    let hist = achats::historique_prix_achat(b.db.conn(), &coca).unwrap();
    assert_eq!(hist[0].1, 550, "26 400 / 48 = 550 par bouteille");
    assert_eq!(hist[1].1, 500, "prix précédent conservé");
}

/// Scénario 11 : inventaire du soir, 30 théoriques, 27 comptés.
#[test]
fn s11_inventaire_du_soir() {
    let mut b = banc();
    let j = b.ouvrir_journee();
    b.ouvrir_caisse(0);
    let c = b.commande_table("1", &[("Coca-Cola", 18)]);
    b.payer_especes(&c, 13_500);
    assert_eq!(b.stock("Coca-Cola 33 cl"), 30);
    let coca = b.article("Coca-Cola 33 cl");
    let s = b.caissier();
    let g = b.gerant();
    let inv = stock::creer_inventaire(&mut b.db, &g, "Boissons du soir").unwrap();
    stock::saisir_comptage(&mut b.db, &g, &inv, &coca, 27).unwrap();
    // Le caissier ne peut pas valider.
    assert_eq!(stock::valider_inventaire(&mut b.db, &s, &inv).unwrap_err().code(), "AUTORISATION_REQUISE");
    stock::valider_inventaire(&mut b.db, &g, &inv).unwrap();
    assert_eq!(b.stock("Coca-Cola 33 cl"), 27);
    let i = stock::inventaire(b.db.conn(), &inv).unwrap();
    assert_eq!((i.lignes[0].theorique, i.lignes[0].compte, i.lignes[0].ecart), (30, 27, -3));
    let t = rapports::ou_part_le_stock(b.db.conn(), &j.date_exploitation, &j.date_exploitation).unwrap();
    let l = t.lignes.iter().find(|l| l[0] == "Coca-Cola 33 cl").unwrap();
    assert_eq!(l[2], 18);
    assert_eq!(l[6], -3);
    assert_eq!(l[7], -1_500);
}

/// Scénario 12 : retour du livreur, trois livraisons payées en espèces.
#[test]
fn s12_retour_livreur() {
    let mut b = banc();
    b.ouvrir_journee();
    b.ouvrir_caisse(5_000);
    let ibrahim = b.employe("Ibrahim Keïta");
    let a = b.caissier();
    for (i, q) in ["Hamdallaye", "Badalabougou", "Kalaban Coura"].iter().enumerate() {
        let c = commandes::ouvrir(
            &mut b.db,
            &a,
            &NouvelleCommande {
                type_: "livraison".into(),
                table_id: None,
                client_id: None,
                employe_id: None,
                couverts: 0,
                note: String::new(),
                livraison: Some(commandes::InfosLivraison { quartier: q.to_string(), repere: "Près du marché".into(), telephone: format!("7600000{i}"), frais: None, lat: None, lon: None }),
                canal: None,
            },
        )
        .unwrap();
        let l = b.ligne("Poulet braisé", 1);
        commandes::ajouter_lignes(&mut b.db, &a, &c, &[l]).unwrap();
        commandes::envoyer(&mut b.db, &a, &c).unwrap();
        livraison::assigner(&mut b.db, &a, &c, &ibrahim).unwrap();
        let total = b.total(&c).total;
        let mut part = especes(total);
        part.par_livreur = true;
        caisse::encaisser(&mut b.db, &a, &Encaissement { commande_id: c, parts: vec![part], especes_recues: None }).unwrap();
    }
    // 3 × 3 500 + 500 + 1 000 + 1 500 de frais = 13 500 à remettre ; la caisse n'a pas bougé.
    assert_eq!(b.solde("Caisse principale"), 5_000);
    let soldes = livraison::soldes_livreurs(b.db.conn()).unwrap();
    assert_eq!(soldes[0].a_remettre, 13_500);
    let e = livraison::remise_livreur(&mut b.db, &a, &ibrahim, 13_000, "").unwrap_err();
    assert_eq!(e.regle_code(), Some("RG-LIV-03"));
    let r = livraison::remise_livreur(&mut b.db, &a, &ibrahim, 13_000, "Monnaie rendue en trop").unwrap();
    assert_eq!(r.ecart, -500);
    assert_eq!(b.solde("Caisse principale"), 18_000);
    assert_eq!(livraison::soldes_livreurs(b.db.conn()).unwrap()[0].a_remettre, 0);
}

/// Scénario 14 : restauration depuis une clé USB sur un nouveau PC.
#[test]
fn s14_restauration_nouveau_pc() {
    let mut b = banc();
    b.ouvrir_journee();
    b.ouvrir_caisse(7_000);
    let usb = b.dossier.path().join("usb");
    let p = b.proprietaire();
    let s = sauvegarde::exporter(&mut b.db, &p, &usb).unwrap();
    // Nouveau PC : base vide.
    let neuf = tempfile::tempdir().unwrap();
    let mut db = Db::ouvrir(&neuf.path().join("youma.db"), b.horloge.clone()).unwrap();
    assert_eq!(auth::nombre_utilisateurs(db.conn()).unwrap(), 0);
    // Base vide : on restaure en tant que système (assistant d'installation).
    sauvegarde::restaurer(&mut db, &Acteur::systeme(), std::path::Path::new(&s.chemin), &neuf.path().join("sauvegardes")).unwrap();
    assert!(auth::nombre_utilisateurs(db.conn()).unwrap() >= 5);
    assert_eq!(caisse::solde(db.conn(), &b.compte("Caisse principale")).unwrap(), 7_000);
    assert!(journee::ouverte(db.conn()).unwrap().is_some());
}

/// Scénario 15 : imprimante cuisine en panne → envoi en attente, signalé, réimprimable ailleurs.
#[test]
fn s15_imprimante_en_panne() {
    let mut b = banc();
    b.ouvrir_journee();
    b.db.conn().execute("UPDATE postes_preparation SET imprimante = 'tcp:10.0.0.250:9100' WHERE nom = 'Grill'", []).unwrap();
    b.commande_table("8", &[("Poulet braisé", 2)]);
    let jobs = impression::jobs_a_imprimer(b.db.conn()).unwrap();
    assert_eq!(jobs.len(), 1);
    impression::marquer_job(&b.db, &jobs[0].id, Some("Imprimante injoignable")).unwrap();
    let tdb = rapports::tableau_de_bord(b.db.conn()).unwrap();
    assert_eq!(tdb.impressions_en_erreur, 1);
    let a = b.caissier();
    let copie = impression::reimprimer(&mut b.db, &a, &jobs[0].id, Some("fichier:/tmp/caisse.txt")).unwrap();
    let a_faire = impression::jobs_a_imprimer(b.db.conn()).unwrap();
    assert_eq!(a_faire.len(), 1);
    assert_eq!(a_faire[0].id, copie);
    assert!(a_faire[0].contenu.contains("COPIE"));
    assert_eq!(rapports::tableau_de_bord(b.db.conn()).unwrap().impressions_en_erreur, 0);
}

/// Clôture de caisse avec écart et motif (complète le scénario 2).
#[test]
fn cloture_caisse_avec_ecart() {
    let mut b = banc();
    b.ouvrir_journee();
    let s = b.ouvrir_caisse(10_000);
    let c = b.commande_table("1", &[("Bière blonde", 5)]);
    b.payer_especes(&c, 5_000);
    let a = b.caissier();
    let e = caisse::cloturer_session(&mut b.db, &a, &s, &ClotureSession { compte_final: 14_000, billetage: vec![], motif_ecart: String::new() }).unwrap_err();
    assert_eq!(e.regle_code(), Some("RG-CAI-09"));
    let r = caisse::cloturer_session(&mut b.db, &a, &s, &ClotureSession { compte_final: 14_000, billetage: vec![], motif_ecart: "Erreur de rendu".into() }).unwrap();
    assert_eq!(r.ecart, Some(-1_000));
    assert_eq!(b.solde("Caisse principale"), 14_000);
    let z = rapports::rapport_z(b.db.conn(), &s).unwrap();
    assert!(z.contains("RAPPORT Z"));
    assert!(z.contains("-1 000"));
    assert!(z.contains("Erreur de rendu"));
    // Passation : le caissier suivant ouvre avec le fond laissé.
    let p = b.proprietaire();
    caisse::ouvrir_session(&mut b.db, &p, &caisse::OuvertureSession { compte_id: None, fond_compte: 14_000, billetage: vec![], motif_ecart: String::new() }).unwrap();
    // Paiement partiel puis division.
    let _ = PartSaisie { moyen: "especes".into(), montant: 1, compte_id: None, reference: None, numero_payeur: None, client_id: None, par_livreur: false };
}
