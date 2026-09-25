//! Cloud facultatif : résumé de journée, SMS, mot de passe distant, secrets masqués (fiche 0018, RG-CLO-01 à 05).

mod commun;

use commun::*;
use youma_core::{cloud, parametres};

#[test]
fn rg_clo_01_04_resume_de_journee_et_sms() {
    let mut b = banc();
    assert!(cloud::resume_journee(b.db.conn(), "2026-03-14", 0).unwrap().is_none(), "pas de journée");
    b.ouvrir_journee();
    b.ouvrir_caisse(0);
    let c = b.commande_table("1", &[("Poulet braisé", 2)]);
    b.payer_especes(&c, 7_000);
    let om = b.compte("Orange Money");
    let c2 = b.commande_table("2", &[("Coca-Cola", 2)]);
    let ca = b.caissier();
    youma_core::caisse::encaisser(&mut b.db, &ca, &youma_core::caisse::Encaissement { commande_id: c2, parts: vec![mobile_money(&om, 1_500, "REF1")], especes_recues: None }).unwrap();
    let r = cloud::resume_journee(b.db.conn(), "2026-03-14", 42).unwrap().unwrap();
    assert_eq!((r.chiffre_affaires, r.commandes, r.cloturee), (8_500, 2, false));
    assert_eq!(r.encaissements, vec![("especes".to_string(), 7_000), ("mobile_money".to_string(), 1_500)]);
    assert_eq!(r.mobile_money_a_verifier, (1, 1_500));
    let t = cloud::texte_sms("Maquis Le Baobab", &r);
    assert_eq!(t, "Maquis Le Baobab 14/03 : CA 8 500 F, 2 cmd (espèces 7 000, MM 1 500). MM à vérifier : 1 (1 500 F)");
    assert!(t.chars().count() <= 160 * 2);
    assert_eq!(cloud::resumes_recents(b.db.conn(), 7, 42).unwrap().len(), 1);
}

#[test]
fn rg_clo_03_05_mot_de_passe_distant_et_secrets() {
    let mut b = banc();
    let g = b.gerant();
    assert!(cloud::definir_mot_de_passe(&mut b.db, &g, "baobab-distant").is_err(), "administration : mot de passe exigé");
    let p = b.proprietaire();
    assert_eq!(cloud::definir_mot_de_passe(&mut b.db, &p, "court").unwrap_err().regle_code(), Some("RG-CLO-03"));
    cloud::definir_mot_de_passe(&mut b.db, &p, "baobab-distant").unwrap();
    let hash = parametres::lire(b.db.conn()).unwrap().cloud.mdp_hash;
    assert!(hash.starts_with("$argon2") && youma_core::auth::verifier("baobab-distant", &hash));
    // Clé et phrase enregistrées, jamais visibles sans connexion ; le masque renvoyé les conserve.
    let mut x = parametres::lire(b.db.conn()).unwrap();
    x.cloud.cle = "cle-du-restaurant".into();
    x.cloud.phrase_chiffrement = "phrase secrète du maquis".into();
    parametres::modifier(&mut b.db, &p, &x).unwrap();
    let publics = parametres::publics(b.db.conn()).unwrap();
    assert_eq!((publics.cloud.cle.as_str(), publics.cloud.phrase_chiffrement.as_str(), publics.cloud.mdp_hash.as_str()), ("********", "********", "********"));
    parametres::modifier(&mut b.db, &p, &publics).unwrap();
    let apres = parametres::lire(b.db.conn()).unwrap().cloud;
    assert_eq!((apres.cle.as_str(), apres.phrase_chiffrement.as_str()), ("cle-du-restaurant", "phrase secrète du maquis"));
    assert_eq!(apres.mdp_hash, hash, "l'empreinte ne change que par definir_mot_de_passe");
    // Rien de secret dans le journal d'audit.
    let journal: String = b.db.conn().query_row("SELECT group_concat(COALESCE(apres, ''), ' ') FROM journal_audit", [], |r| r.get(0)).unwrap();
    assert!(!journal.contains("cle-du-restaurant") && !journal.contains("phrase secrète") && !journal.contains(&hash));
}
