//! Règles métier numérotées (docs/conception/phase-2-donnees-regles.md).

mod commun;

use commun::*;
use youma_core::auth::{self, NouvelUtilisateur};
use youma_core::caisse::{self, Encaissement, MouvementCaisse, NouvelleDepense, OuvertureSession};
use youma_core::catalogue;
use youma_core::commandes::{self, LigneSaisie, NouvelleCommande};
use youma_core::employes::{self, Employe, Evenement, SaisiePresence};
use youma_core::horloge::HorlogeFixe;
use youma_core::licence;
use youma_core::paie::{self, PaiementSalaire};
use youma_core::parametres;
use youma_core::{journee, Acteur, Db};

fn employe_minimal(nom: &str, remuneration: &str, montant: i64) -> Employe {
    serde_json::from_value(serde_json::json!({ "nom": nom, "type_remuneration": remuneration, "montant_base": montant })).unwrap()
}

// ───────────── Système ─────────────

#[test]
fn rg_sys_03_tables_financieres_en_ajout_seul() {
    let mut b = banc();
    b.ouvrir_journee();
    b.ouvrir_caisse(1_000);
    let c = b.commande_table("1", &[("Coca-Cola", 1)]);
    b.payer_especes(&c, 750);
    for sql in [
        "UPDATE paiements SET montant = 1",
        "DELETE FROM paiements",
        "UPDATE mouvements_tresorerie SET montant = 0",
        "DELETE FROM mouvements_stock",
        "DELETE FROM journal_audit",
        "DELETE FROM commandes",
    ] {
        let e = b.db.conn().execute(sql, []).unwrap_err();
        assert!(e.to_string().contains("AJOUT_SEUL"), "{sql} : {e}");
    }
    // Une ligne envoyée ne se supprime pas non plus.
    assert!(b.db.conn().execute("DELETE FROM lignes_commande", []).is_err());
}

#[test]
fn rg_sys_04_numeros_sans_trou_meme_apres_echec() {
    let mut b = banc();
    b.ouvrir_journee();
    b.ouvrir_caisse(0);
    let c1 = b.commande_table("1", &[("Coca-Cola", 1)]);
    let r1 = b.payer_especes(&c1, 750);
    let c2 = b.commande_table("2", &[("Coca-Cola", 1)]);
    // Échec (paiement supérieur au dû) : la séquence ne doit pas avancer.
    let a = b.caissier();
    assert!(caisse::encaisser(&mut b.db, &a, &Encaissement { commande_id: c2.clone(), parts: vec![especes(5_000)], especes_recues: None }).is_err());
    let r2 = b.payer_especes(&c2, 750);
    assert_eq!(r2.numero, r1.numero + 1);
}

#[test]
fn rg_sys_06_licence_expiree_ne_bloque_pas_les_ventes() {
    use ed25519_dalek::SigningKey;
    let mut b = banc();
    let cle = SigningKey::from_bytes(&[7u8; 32]);
    let code = licence::code_machine(b.db.conn()).unwrap();
    let l = licence::Licence {
        numero: "L-1".into(),
        restaurant: "Maquis Le Baobab".into(),
        code_machine: code,
        modules: vec!["reseau".into(), "cloud".into()],
        emise_le: "2025-01-01".into(),
        maintenance_jusqua: Some("2025-12-31".into()),
    };
    let texte = licence::signer(&l, &cle).unwrap();
    let p = b.proprietaire();
    licence::installer_avec_cle(&mut b.db, &p, &texte, &cle.verifying_key()).unwrap();
    let e = licence::etat_avec_cle(b.db.conn(), "2026-03-14", &cle.verifying_key()).unwrap();
    assert!(e.valide);
    assert!(!e.maintenance_active);
    assert_eq!(e.modules, vec!["reseau".to_string()], "cloud suspendu, réseau local conservé");
    // Les ventes continuent.
    b.ouvrir_journee();
    b.ouvrir_caisse(0);
    let c = b.commande_table("1", &[("Coca-Cola", 1)]);
    b.payer_especes(&c, 750);
    // Une licence falsifiée est refusée.
    let autre = SigningKey::from_bytes(&[9u8; 32]);
    assert!(licence::verifier(&texte, &autre.verifying_key()).is_err());
    let mut l2 = l.clone();
    l2.code_machine = "AAAA-BBBB-CCCC-DDDD".into();
    let t2 = licence::signer(&l2, &cle).unwrap();
    assert!(licence::installer_avec_cle(&mut b.db, &p, &t2, &cle.verifying_key()).is_err());
}

// ───────────── Accès ─────────────

#[test]
fn rg_aut_01_pin_unique_et_format() {
    let mut b = banc();
    let p = b.proprietaire();
    let u = |pin: &str| NouvelUtilisateur { nom: "Test".into(), role_code: "serveur".into(), pin: pin.into(), mot_de_passe: None, employe_id: None };
    assert_eq!(auth::creer_utilisateur(&mut b.db, &p, &u("12")).unwrap_err().regle_code(), Some("RG-AUT-01"));
    assert_eq!(auth::creer_utilisateur(&mut b.db, &p, &u("12a4")).unwrap_err().regle_code(), Some("RG-AUT-01"));
    assert_eq!(auth::creer_utilisateur(&mut b.db, &p, &u("4444")).unwrap_err().regle_code(), Some("RG-AUT-01"));
    auth::creer_utilisateur(&mut b.db, &p, &u("987654")).unwrap();
    // Un serveur ne crée pas d'utilisateur.
    let s = b.serveur();
    assert_eq!(auth::creer_utilisateur(&mut b.db, &s, &u("1111")).unwrap_err().code(), "AUTORISATION_REQUISE");
}

#[test]
fn rg_aut_02_verrouillage_apres_cinq_echecs() {
    let mut b = banc();
    let s = b.demo.serveur.clone();
    for _ in 0..4 {
        assert_eq!(auth::connexion_pin(&mut b.db, &s, "0000", None).unwrap_err().code(), "PIN_INCORRECT");
    }
    assert_eq!(auth::connexion_pin(&mut b.db, &s, "0000", None).unwrap_err().code(), "VERROUILLE");
    // Même le bon PIN est refusé pendant 5 minutes.
    assert_eq!(auth::connexion_pin(&mut b.db, &s, "4444", None).unwrap_err().code(), "VERROUILLE");
    b.horloge.avancer_minutes(6);
    let session = auth::connexion_pin(&mut b.db, &s, "4444", None).unwrap();
    assert!(session.permissions.contains(&"commande.creer".to_string()));
}

#[test]
fn rg_aut_04_session_expire_apres_inactivite() {
    let mut b = banc();
    let s = b.demo.serveur.clone();
    let session = auth::connexion_pin(&mut b.db, &s, "4444", None).unwrap();
    assert!(auth::verifier_session(&b.db, &session.jeton).is_ok());
    b.horloge.avancer_minutes(16);
    assert_eq!(auth::verifier_session(&b.db, &session.jeton).unwrap_err().code(), "NON_AUTHENTIFIE");
}

#[test]
fn rg_aut_05_proprietaire_intouchable() {
    let mut b = banc();
    let p = b.proprietaire();
    let e = auth::modifier_role(&mut b.db, &p, &auth::ModifRole { code: "proprietaire".into(), plafond_remise_pct: 0, permissions: vec![] }).unwrap_err();
    assert_eq!(e.regle_code(), Some("RG-AUT-05"));
    let id = b.demo.proprietaire.clone();
    let e = auth::modifier_utilisateur(&mut b.db, &p, &auth::ModifUtilisateur { id, nom: None, role_code: None, pin: None, actif: Some(false), employe_id: None }).unwrap_err();
    assert_eq!(e.regle_code(), Some("RG-AUT-05"));
}

// ───────────── Journée ─────────────

#[test]
fn rg_jou_01_02_04_journee() {
    let mut b = banc();
    let a = b.serveur();
    let t = b.table("1");
    let n = NouvelleCommande { type_: "sur_place".into(), table_id: Some(t), client_id: None, employe_id: None, couverts: 0, note: String::new(), livraison: None };
    assert_eq!(commandes::ouvrir(&mut b.db, &a, &n).unwrap_err().regle_code(), Some("RG-JOU-01"));
    b.ouvrir_journee();
    let c = b.caissier();
    assert_eq!(journee::ouvrir(&mut b.db, &c).unwrap_err().regle_code(), Some("RG-JOU-02"));
    commandes::ouvrir(&mut b.db, &a, &n).unwrap();
    assert_eq!(journee::cloturer(&mut b.db, &c).unwrap_err().regle_code(), Some("RG-JOU-04"));
}

// ───────────── Catalogue ─────────────

#[test]
fn rg_cat_01_02_03_prix_copie_historise_et_par_zone() {
    let mut b = banc();
    b.ouvrir_journee();
    let c_vip = b.commande_table("V1", &[("Bière blonde", 1)]);
    let c_salle = b.commande_table("1", &[("Bière blonde", 1)]);
    assert_eq!(b.total(&c_vip).total, 1_500, "grille VIP");
    assert_eq!(b.total(&c_salle).total, 1_000);
    let mut p = catalogue::produit(b.db.conn(), &b.produit("Bière blonde")).unwrap();
    p.prix = 1_250;
    let g = b.gerant();
    catalogue::enregistrer_produit(&mut b.db, &g, &p).unwrap();
    assert_eq!(b.total(&c_salle).total, 1_000, "RG-CAT-01 : la commande garde son prix");
    let h = catalogue::historique_prix(b.db.conn(), &p.id).unwrap();
    assert!(h.iter().any(|l| l.ancien == Some(1_000) && l.nouveau == 1_250));
}

#[test]
fn rg_cat_04_05_options_et_rupture() {
    let mut b = banc();
    b.ouvrir_journee();
    let a = b.serveur();
    let t = b.table("2");
    let c = commandes::ouvrir(&mut b.db, &a, &NouvelleCommande { type_: "sur_place".into(), table_id: Some(t), client_id: None, employe_id: None, couverts: 0, note: String::new(), livraison: None }).unwrap();
    let frites = catalogue::produit(b.db.conn(), &b.produit("Frites")).unwrap();
    // Taille obligatoire (min 1).
    let sans = LigneSaisie { produit_id: frites.id.clone(), quantite: 1, options: vec![], commentaire: String::new() };
    assert_eq!(commandes::ajouter_lignes(&mut b.db, &a, &c, &[sans]).unwrap_err().regle_code(), Some("RG-CAT-04"));
    let grande = frites.groupes_options[0].options.iter().find(|o| o.nom == "Grande").unwrap().id.clone();
    let avec = LigneSaisie { produit_id: frites.id.clone(), quantite: 2, options: vec![grande], commentaire: String::new() };
    commandes::ajouter_lignes(&mut b.db, &a, &c, &[avec]).unwrap();
    assert_eq!(b.total(&c).total, 2 * (750 + 500));
    // Rupture du jour.
    let caissier = b.caissier();
    catalogue::definir_disponibilite(&mut b.db, &caissier, &frites.id, false).unwrap();
    let l = LigneSaisie { produit_id: b.produit("Poulet braisé"), quantite: 1, options: vec![], commentaire: String::new() };
    commandes::ajouter_lignes(&mut b.db, &a, &c, &[l]).unwrap();
    let l = LigneSaisie { produit_id: frites.id, quantite: 1, options: vec![], commentaire: String::new() };
    assert_eq!(commandes::ajouter_lignes(&mut b.db, &a, &c, &[l]).unwrap_err().regle_code(), Some("RG-CAT-05"));
}

// ───────────── Commandes ─────────────

#[test]
fn rg_cmd_05_remise_plafonnee_par_role() {
    let mut b = banc();
    b.ouvrir_journee();
    let c = b.commande_table("3", &[("Poulet braisé", 2)]);
    let caissier = b.caissier();
    // Caissier : plafond 10 %.
    assert_eq!(commandes::remise(&mut b.db, &caissier, &c, None, Some(700), None, "").unwrap_err().regle_code(), Some("RG-CMD-05"));
    commandes::remise(&mut b.db, &caissier, &c, None, Some(700), None, "Client fidèle").unwrap();
    let e = commandes::remise(&mut b.db, &caissier, &c, None, None, Some(30), "Client fidèle").unwrap_err();
    assert_eq!(e.code(), "AUTORISATION_REQUISE");
    commandes::remise(&mut b.db, &Banc::avec_pin_gerant(caissier), &c, None, None, Some(20), "Anniversaire").unwrap();
    let t = b.total(&c);
    // 7 000 − 700 = 6 300 ; 20 % de 6 300 = 1 260 arrondi à 1 250.
    assert_eq!(t.remises, 700 + 1_250);
    assert_eq!(t.total, 7_000 - 1_950);
    // Un serveur n'a pas le droit de remise.
    let s = b.serveur();
    assert_eq!(commandes::remise(&mut b.db, &s, &c, None, Some(100), None, "x").unwrap_err().code(), "AUTORISATION_REQUISE");
}

#[test]
fn rg_cmd_06_offert_sort_du_stock_sans_ca() {
    let mut b = banc();
    let j = b.ouvrir_journee();
    b.ouvrir_caisse(0);
    let avant = b.stock("Bière blonde 65 cl");
    let c = b.commande_table("4", &[("Bière blonde", 2), ("Coca-Cola", 1)]);
    let biere = commandes::detail(b.db.conn(), &c).unwrap().lignes[0].id.clone();
    let s = b.serveur();
    assert_eq!(commandes::offrir(&mut b.db, &s, &biere, "Habitué").unwrap_err().code(), "AUTORISATION_REQUISE");
    let g = b.gerant();
    commandes::offrir(&mut b.db, &g, &biere, "Habitué").unwrap();
    assert_eq!(b.total(&c).total, 750);
    assert_eq!(b.stock("Bière blonde 65 cl"), avant - 2);
    b.payer_especes(&c, 750);
    let ch = youma_core::rapports::chiffres(b.db.conn(), &j.date_exploitation, &j.date_exploitation).unwrap();
    assert_eq!(ch.ca, 750);
    assert_eq!(ch.offerts, 2_000);
}

#[test]
fn rg_cmd_08_commande_payee_figee_et_contrepassation() {
    let mut b = banc();
    b.ouvrir_journee();
    b.ouvrir_caisse(0);
    let c = b.commande_table("5", &[("Coca-Cola", 2)]);
    let r = b.payer_especes(&c, 1_500);
    let s = b.serveur();
    let l = b.ligne("Coca-Cola", 1);
    assert_eq!(commandes::ajouter_lignes(&mut b.db, &s, &c, &[l]).unwrap_err().regle_code(), Some("RG-CMD-08"));
    // Annulation du paiement : caissier sans droit → gérant.
    let ca = b.caissier();
    assert_eq!(caisse::annuler_paiement(&mut b.db, &ca, &r.paiement_id, "Erreur de table").unwrap_err().code(), "AUTORISATION_REQUISE");
    caisse::annuler_paiement(&mut b.db, &Banc::avec_pin_gerant(ca), &r.paiement_id, "Erreur de table").unwrap();
    assert_eq!(b.solde("Caisse principale"), 0);
    let d = commandes::detail(b.db.conn(), &c).unwrap();
    assert_eq!(d.statut, "ouverte");
    assert_eq!(d.totaux.paye, 0);
    assert_eq!(b.compter("SELECT COUNT(*) FROM paiements"), 2, "original + contre-passation");
}

#[test]
fn rg_cmd_09_transfert_et_fusion() {
    let mut b = banc();
    b.ouvrir_journee();
    let c1 = b.commande_table("1", &[("Coca-Cola", 1)]);
    let c2 = b.commande_table("2", &[("Bière blonde", 1)]);
    let a = b.caissier();
    let t2 = b.table("2");
    assert_eq!(commandes::transferer(&mut b.db, &a, &c1, &t2).unwrap_err().regle_code(), Some("RG-CMD-09"));
    let t9 = b.table("T1");
    commandes::transferer(&mut b.db, &a, &c1, &t9).unwrap();
    commandes::fusionner(&mut b.db, &a, &c1, &c2).unwrap();
    assert_eq!(b.total(&c2).total, 1_750);
    assert_eq!(commandes::detail(b.db.conn(), &c1).unwrap().statut, "annulee");
}

#[test]
fn rg_cmd_11_payer_d_abord_au_comptoir() {
    let mut b = banc();
    b.ouvrir_journee();
    b.ouvrir_caisse(0);
    let a = b.caissier();
    let c = commandes::ouvrir(&mut b.db, &a, &NouvelleCommande { type_: "comptoir".into(), table_id: None, client_id: None, employe_id: None, couverts: 0, note: String::new(), livraison: None }).unwrap();
    let l = b.ligne("Brochettes (3)", 1);
    commandes::ajouter_lignes(&mut b.db, &a, &c, &[l]).unwrap();
    assert_eq!(commandes::envoyer(&mut b.db, &a, &c).unwrap_err().regle_code(), Some("RG-CMD-11"));
    // Le paiement déclenche l'envoi.
    b.payer_especes(&c, 1_500);
    let d = commandes::detail(b.db.conn(), &c).unwrap();
    assert_eq!(d.statut, "payee");
    assert_eq!(d.envois.len(), 1);
}

// ───────────── Caisse ─────────────

#[test]
fn rg_cai_01_session_obligatoire() {
    let mut b = banc();
    b.ouvrir_journee();
    let c = b.commande_table("1", &[("Coca-Cola", 1)]);
    let a = b.caissier();
    let e = caisse::encaisser(&mut b.db, &a, &Encaissement { commande_id: c, parts: vec![especes(750)], especes_recues: None }).unwrap_err();
    assert_eq!(e.regle_code(), Some("RG-CAI-01"));
}

#[test]
fn rg_cai_02_04_paiement_partiel_et_reference_mm() {
    let mut b = banc();
    b.ouvrir_journee();
    b.ouvrir_caisse(0);
    let c = b.commande_table("1", &[("Poulet braisé", 2)]);
    let om = b.compte("Orange Money");
    let a = b.caissier();
    let sans_ref = youma_core::caisse::PartSaisie { reference: None, ..mobile_money(&om, 3_500, "") };
    assert_eq!(caisse::encaisser(&mut b.db, &a, &Encaissement { commande_id: c.clone(), parts: vec![sans_ref], especes_recues: None }).unwrap_err().regle_code(), Some("RG-CAI-04"));
    let r = caisse::encaisser(&mut b.db, &a, &Encaissement { commande_id: c.clone(), parts: vec![mobile_money(&om, 3_500, "REF-1")], especes_recues: None }).unwrap();
    assert!(!r.commande_payee);
    assert_eq!(r.reste, 3_500);
    assert_eq!(caisse::encaisser(&mut b.db, &a, &Encaissement { commande_id: c.clone(), parts: vec![especes(4_000)], especes_recues: None }).unwrap_err().regle_code(), Some("RG-CAI-02"));
    assert!(b.payer_especes(&c, 3_500).commande_payee);
    // Vérification par le gérant.
    let part = caisse::parts_mobile_money(b.db.conn(), None).unwrap()[0].part_id.clone();
    assert_eq!(caisse::verifier_mobile_money(&mut b.db, &a, &part, "verifie", "").unwrap_err().code(), "AUTORISATION_REQUISE");
    let g = b.gerant();
    caisse::verifier_mobile_money(&mut b.db, &g, &part, "verifie", "Vu sur le téléphone").unwrap();
    assert!(caisse::parts_mobile_money(b.db.conn(), Some("a_verifier")).unwrap().is_empty());
}

#[test]
fn rg_cai_08_10_11_13_mouvements_de_caisse() {
    let mut b = banc();
    let j = b.ouvrir_journee();
    let s = b.ouvrir_caisse(20_000);
    let a = b.caissier();
    // RG-CAI-11 : pas de sortie au-delà des espèces présentes.
    let e = caisse::mouvement_caisse(&mut b.db, &a, &MouvementCaisse { type_: "retrait".into(), montant: 25_000, libelle: "Banque".into() }).unwrap_err();
    assert_eq!(e.regle_code(), Some("RG-CAI-11"));
    // RG-CAI-10 : le caissier ne fait pas de retrait propriétaire.
    let e = caisse::mouvement_caisse(&mut b.db, &a, &MouvementCaisse { type_: "retrait_proprietaire".into(), montant: 5_000, libelle: String::new() }).unwrap_err();
    assert_eq!(e.code(), "AUTORISATION_REQUISE");
    caisse::mouvement_caisse(&mut b.db, &a.clone().avec_autorisation(Some("1234".into())), &MouvementCaisse { type_: "retrait_proprietaire".into(), montant: 5_000, libelle: String::new() }).unwrap();
    let cat = caisse::categories_depense(b.db.conn()).unwrap().into_iter().find(|(_, n)| n == "Gaz / charbon").unwrap().0;
    caisse::depenser(&mut b.db, &a, &NouvelleDepense { categorie_id: cat, montant: 3_000, compte_id: None, beneficiaire: "Vendeuse de charbon".into(), libelle: "Sac de charbon".into(), justificatif: String::new() }).unwrap();
    assert_eq!(b.solde("Caisse principale"), 12_000);
    assert_eq!(b.solde("Coffre / propriétaire"), 5_000);
    let ch = youma_core::rapports::chiffres(b.db.conn(), &j.date_exploitation, &j.date_exploitation).unwrap();
    assert_eq!(ch.depenses, 3_000, "le retrait propriétaire n'est pas une dépense");
    // RG-CAI-08 : fond compté différent du théorique → motif.
    let fin = caisse::ClotureSession { compte_final: 12_000, billetage: vec![caisse::LigneBilletage { coupure: 10_000, nombre: 1 }, caisse::LigneBilletage { coupure: 1_000, nombre: 2 }], motif_ecart: String::new() };
    caisse::cloturer_session(&mut b.db, &a, &s, &fin).unwrap();
    let e = caisse::ouvrir_session(&mut b.db, &a, &OuvertureSession { compte_id: None, fond_compte: 2_000, billetage: vec![], motif_ecart: String::new() }).unwrap_err();
    assert_eq!(e.regle_code(), Some("RG-CAI-08"));
}

// ───────────── Employés et paie : réalités maliennes ─────────────

#[test]
fn rg_emp_01_02_employe_sans_contrat_ni_inps() {
    let mut b = banc();
    let g = b.gerant();
    // Seulement un nom et un taux journalier : accepté.
    let id = employes::enregistrer(&mut b.db, &g, &employe_minimal("Bakary", "journalier", 2_500)).unwrap();
    let e = employes::employe(b.db.conn(), &id).unwrap();
    assert_eq!(e.type_contrat, "aucun");
    assert!(!e.declare_inps && !e.affilie_amo);
    assert!(e.telephone.is_empty() && e.date_embauche.is_none());
    // Aide familiale non rémunérée : suivi des avances et des repas seulement.
    employes::enregistrer(&mut b.db, &g, &employe_minimal("Petite sœur", "aucun", 0)).unwrap();
    // Un salarié mensuel sans montant est refusé (RG-EMP-04).
    assert_eq!(employes::enregistrer(&mut b.db, &g, &employe_minimal("X", "mensuel", 0)).unwrap_err().regle_code(), Some("RG-EMP-04"));
    // Contrat inconnu refusé.
    let mut e = employe_minimal("Y", "mensuel", 40_000);
    e.type_contrat = "cdd-bidon".into();
    assert_eq!(employes::enregistrer(&mut b.db, &g, &e).unwrap_err().regle_code(), Some("RG-EMP-02"));
    // Un serveur ne gère pas le personnel.
    let s = b.serveur();
    assert!(employes::enregistrer(&mut b.db, &s, &employe_minimal("Z", "journalier", 1_000)).is_err());
}

#[test]
fn rg_emp_07_derniere_presence_fait_foi() {
    let mut b = banc();
    b.ouvrir_journee();
    let moussa = b.employe("Moussa Coulibaly");
    let g = b.gerant();
    employes::pointer(&mut b.db, &g, &[SaisiePresence { employe_id: moussa.clone(), date: None, statut: "absent_non_justifie".into(), minutes_retard: 0, note: String::new() }]).unwrap();
    b.horloge.avancer_minutes(60);
    employes::pointer(&mut b.db, &g, &[SaisiePresence { employe_id: moussa.clone(), date: None, statut: "retard".into(), minutes_retard: 45, note: "Transport".into() }]).unwrap();
    let p = employes::presences(b.db.conn(), Some(&moussa), "2026-03-14", "2026-03-14").unwrap();
    assert_eq!(p.len(), 1);
    assert_eq!(p[0].statut, "retard");
    assert_eq!(b.compter("SELECT COUNT(*) FROM presences"), 2, "la première saisie reste dans l'historique");
}

fn pointer_jours(b: &mut Banc, employe: &str, jours: &[(&str, &str)]) {
    let g = b.gerant();
    let s: Vec<SaisiePresence> = jours
        .iter()
        .map(|(d, st)| SaisiePresence { employe_id: employe.into(), date: Some(d.to_string()), statut: st.to_string(), minutes_retard: 0, note: String::new() })
        .collect();
    employes::pointer(&mut b.db, &g, &s).unwrap();
}

#[test]
fn rg_pai_03_journalier_paye_aux_jours_de_presence() {
    let mut b = banc();
    let moussa = b.employe("Moussa Coulibaly");
    pointer_jours(&mut b, &moussa, &[("2026-03-02", "present"), ("2026-03-03", "retard"), ("2026-03-04", "absent_non_justifie"), ("2026-03-05", "present"), ("2026-03-06", "repos")]);
    let g = b.gerant();
    let apercu = paie::apercu(b.db.conn(), &moussa, "2026-03-01", "2026-03-07").unwrap();
    assert_eq!(apercu.jours_presents, 3);
    assert_eq!(apercu.net_a_payer, 9_000);
    assert_eq!(b.compter("SELECT COUNT(*) FROM mouvements_employe"), 0, "l'aperçu n'écrit rien");
    let bul = paie::cloturer(&mut b.db, &g, &moussa, "2026-03-01", "2026-03-07").unwrap();
    assert_eq!(bul.net_a_payer, 9_000);
    assert_eq!(bul.type_contrat, "aucun");
}

#[test]
fn rg_pai_05_06_paiement_partiel_et_periode_figee() {
    let mut b = banc();
    b.ouvrir_journee();
    b.ouvrir_caisse(100_000);
    let awa = b.employe("Awa Traoré");
    let g = b.gerant();
    employes::evenement(&mut b.db, &g, &Evenement { employe_id: awa.clone(), type_: "prime".into(), montant: 10_000, quantite: None, motif: "Bon mois".into() }).unwrap();
    employes::evenement(&mut b.db, &g, &Evenement { employe_id: awa.clone(), type_: "retenue".into(), montant: 5_000, quantite: None, motif: "Casse verre".into() }).unwrap();
    let caisse_id = b.compte("Caisse principale");
    employes::avance(&mut b.db, &g, &employes::Avance { employe_id: awa.clone(), montant: 20_000, compte_id: Some(caisse_id.clone()), motif: String::new() }).unwrap();
    let bul = paie::cloturer(&mut b.db, &g, &awa, "2026-03-01", "2026-03-31").unwrap();
    // Exemple du cahier : 50 000 + 10 000 − 20 000 − 5 000 = 35 000.
    assert_eq!(bul.net_a_payer, 35_000);
    assert_eq!(bul.total_gains, 60_000);
    assert_eq!(bul.total_retenues, 25_000);
    let bid = bul.id.clone().unwrap();
    // Paiement partiel puis solde.
    paie::payer(&mut b.db, &g, &PaiementSalaire { employe_id: awa.clone(), montant: 20_000, bulletin_id: Some(bid.clone()), compte_id: Some(caisse_id.clone()), note: String::new() }).unwrap();
    assert_eq!(paie::bulletin(b.db.conn(), &bid).unwrap().reste_a_payer, 15_000);
    let e = paie::payer(&mut b.db, &g, &PaiementSalaire { employe_id: awa.clone(), montant: 20_000, bulletin_id: Some(bid.clone()), compte_id: Some(caisse_id.clone()), note: String::new() }).unwrap_err();
    assert_eq!(e.regle_code(), Some("RG-PAI-05"));
    paie::payer(&mut b.db, &g, &PaiementSalaire { employe_id: awa.clone(), montant: 15_000, bulletin_id: Some(bid.clone()), compte_id: Some(caisse_id), note: String::new() }).unwrap();
    assert_eq!(b.solde("Caisse principale"), 100_000 - 20_000 - 35_000);
    // Période chevauchante refusée ; correction par régularisation sur la suivante.
    assert_eq!(paie::cloturer(&mut b.db, &g, &awa, "2026-03-15", "2026-04-14").unwrap_err().regle_code(), Some("RG-PAI-06"));
    employes::evenement(&mut b.db, &g, &Evenement { employe_id: awa.clone(), type_: "regularisation".into(), montant: 2_000, quantite: None, motif: "Oubli prime mars".into() }).unwrap();
    let avril = paie::cloturer(&mut b.db, &g, &awa, "2026-04-01", "2026-04-30").unwrap();
    assert_eq!(avril.report_precedent, 0, "mars soldé");
    assert_eq!(avril.net_a_payer, 52_000);
    // Le bulletin figé ne change pas.
    assert!(b.db.conn().execute("UPDATE bulletins SET net_a_payer = 0", []).is_err());
}

#[test]
fn rg_pai_07_cotisations_facultatives() {
    let mut b = banc();
    let g = b.gerant();
    let adama = b.employe("Adama Sangaré"); // déclaré INPS et AMO
    let awa = b.employe("Awa Traoré"); // ni l'un ni l'autre
    // Désactivées par défaut : aucun prélèvement, même pour un employé déclaré.
    let a0 = paie::apercu(b.db.conn(), &adama, "2026-03-01", "2026-03-31").unwrap();
    assert_eq!(a0.cotisations_salarie, 0);
    assert_eq!(a0.net_a_payer, 120_000);
    // Activées : seulement pour les employés déclarés.
    let mut p = parametres::lire(b.db.conn()).unwrap();
    p.cotisations.inps_active = true;
    p.cotisations.amo_active = true;
    parametres::ecrire(b.db.conn(), &p).unwrap();
    let a1 = paie::apercu(b.db.conn(), &adama, "2026-03-01", "2026-03-31").unwrap();
    assert_eq!(a1.cotisations_salarie, 4_320 + 3_672);
    assert_eq!(a1.net_a_payer, 120_000 - 7_992);
    assert_eq!(a1.charges_employeur, 19_680 + 4_200, "informatif, jamais retenu");
    let w = paie::apercu(b.db.conn(), &awa, "2026-03-01", "2026-03-31").unwrap();
    assert_eq!(w.cotisations_salarie, 0);
    assert_eq!(w.net_a_payer, 50_000);
    let bul = paie::cloturer(&mut b.db, &g, &adama, "2026-03-01", "2026-03-31").unwrap();
    assert!(bul.lignes.iter().any(|l| l.type_ == "cotisation_inps" && l.montant == -4_320));
}

#[test]
fn rg_pai_08_deduction_absences_activable() {
    let mut b = banc();
    let awa = b.employe("Awa Traoré");
    pointer_jours(&mut b, &awa, &[("2026-03-10", "absent_non_justifie"), ("2026-03-11", "absent_non_justifie"), ("2026-03-12", "absent_justifie")]);
    assert_eq!(paie::apercu(b.db.conn(), &awa, "2026-03-01", "2026-03-31").unwrap().net_a_payer, 50_000);
    let mut p = parametres::lire(b.db.conn()).unwrap();
    p.paie.deduire_absences = true;
    parametres::ecrire(b.db.conn(), &p).unwrap();
    // 50 000 / 26 × 2 = 3 846.
    assert_eq!(paie::apercu(b.db.conn(), &awa, "2026-03-01", "2026-03-31").unwrap().net_a_payer, 50_000 - 3_846);
}

#[test]
fn livreur_paye_a_la_course() {
    let mut b = banc();
    let ibrahim = b.employe("Ibrahim Keïta");
    let g = b.gerant();
    employes::evenement(&mut b.db, &g, &Evenement { employe_id: ibrahim.clone(), type_: "tache".into(), montant: 0, quantite: Some(12), motif: String::new() }).unwrap();
    let bul = paie::cloturer(&mut b.db, &g, &ibrahim, "2026-03-01", "2026-03-15").unwrap();
    assert_eq!(bul.net_a_payer, 6_000);
    assert!(bul.lignes.iter().any(|l| l.type_ == "tache" && l.quantite == Some(12)));
}

#[test]
fn employe_parti_garde_son_historique() {
    let mut b = banc();
    let g = b.gerant();
    let fanta = b.employe("Fanta Diarra");
    let mut e = employes::employe(b.db.conn(), &fanta).unwrap();
    e.statut = "parti".into();
    employes::enregistrer(&mut b.db, &g, &e).unwrap();
    let e = employes::employe(b.db.conn(), &fanta).unwrap();
    assert!(e.date_depart.is_some());
    assert!(!employes::lister(b.db.conn(), false).unwrap().iter().any(|x| x.id == fanta));
    assert!(employes::lister(b.db.conn(), true).unwrap().iter().any(|x| x.id == fanta));
}

// ───────────── Horloge et base ─────────────

#[test]
fn base_neuve_initialisee_sans_utilisateur() {
    let h = std::sync::Arc::new(HorlogeFixe::a("2026-01-01", 8, 0));
    let mut db = Db::en_memoire(h).unwrap();
    assert_eq!(auth::nombre_utilisateurs(db.conn()).unwrap(), 0);
    assert_eq!(caisse::lister_comptes(db.conn()).unwrap().len(), 4);
    let id = auth::installer_proprietaire(&mut db, "Moi", "1234", "Chez Moi").unwrap();
    assert!(auth::installer_proprietaire(&mut db, "Autre", "9999", "").is_err());
    let s = auth::connexion_pin(&mut db, &id, "1234", None).unwrap();
    assert!(s.permissions.len() > 30);
    let _ = Acteur::systeme();
}

#[test]
fn rg_rap_03_salaires_rattaches_a_leur_periode() {
    let mut b = banc();
    // Paie de mars clôturée le 2 avril : les salaires comptent dans le rapport de mars, pas d'avril.
    b.horloge.regler_a("2026-04-02", 10, 0);
    let awa = b.employe("Awa Traoré");
    let g = b.gerant();
    paie::cloturer(&mut b.db, &g, &awa, "2026-03-01", "2026-03-31").unwrap();
    let mars = youma_core::rapports::chiffres(b.db.conn(), "2026-03-01", "2026-03-31").unwrap();
    let avril = youma_core::rapports::chiffres(b.db.conn(), "2026-04-01", "2026-04-30").unwrap();
    assert_eq!(mars.salaires, 50_000);
    assert_eq!(avril.salaires, 0);
}

#[test]
fn rg_cai_14_especes_recues_et_monnaie_rendue() {
    let mut b = banc();
    let j = b.ouvrir_journee();
    let s = b.ouvrir_caisse(10_000);
    let a = b.caissier();
    // 4 × Coca = 3 000 ; le client donne un billet de 5 000.
    let c = b.commande_table("1", &[("Coca-Cola", 4)]);
    let r = caisse::encaisser(&mut b.db, &a, &Encaissement { commande_id: c.clone(), parts: vec![especes(3_000)], especes_recues: Some(5_000) }).unwrap();
    assert_eq!((r.especes_recues, r.rendu), (5_000, 2_000));
    // La caisse ne garde que le prix payé.
    assert_eq!(b.solde("Caisse principale"), 13_000);
    let p = &caisse::paiements_commande(b.db.conn(), &c).unwrap()[0];
    assert_eq!((p.recu, p.rendu), (5_000, 2_000));
    let ticket = youma_core::impression::ticket_client(b.db.conn(), &c).unwrap();
    assert!(ticket.contains("Espèces reçues") && ticket.contains("5 000"), "{ticket}");
    assert!(ticket.contains("Monnaie rendue") && ticket.contains("2 000"), "{ticket}");

    // Paiement 100 % Mobile Money : une saisie « espèces reçues » ne crée pas de monnaie fictive.
    let c2 = b.commande_table("2", &[("Coca-Cola", 2)]);
    let om = b.compte("Orange Money");
    let r2 = caisse::encaisser(&mut b.db, &a, &Encaissement { commande_id: c2, parts: vec![mobile_money(&om, 1_500, "REF-14")], especes_recues: Some(5_000) }).unwrap();
    assert_eq!((r2.especes_recues, r2.rendu), (0, 0));

    // Rapport Z et rapport d'activité.
    let z = youma_core::rapports::rapport_z(b.db.conn(), &s).unwrap();
    assert!(z.contains("Reçues des clients (1)"), "{z}");
    assert!(z.contains("Monnaie rendue"));
    let rap = youma_core::rapports::rapport_periode(b.db.conn(), &j.date_exploitation, &j.date_exploitation).unwrap();
    let t = rap.tableaux.iter().find(|t| t.titre == "Espèces reçues et monnaie rendue").unwrap();
    assert_eq!(t.lignes.len(), 1);
    assert_eq!(t.lignes[0][4..], [serde_json::json!(3_000), serde_json::json!(5_000), serde_json::json!(2_000)]);
}
