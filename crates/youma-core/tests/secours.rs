//! RG-AUT-07 : mot de passe d'administration oublié (fiche 0031).

use std::sync::Arc;

use ed25519_dalek::SigningKey;
use youma_core::auth::{self, NouvelUtilisateur};
use youma_core::horloge::HorlogeFixe;
use youma_core::secours::{self, Preuve};
use youma_core::{Acteur, Db, Erreur};

struct Banc {
    db: Db,
    proprietaire: String,
    code: String,
    cle: SigningKey,
}

fn banc() -> Banc {
    let mut db = Db::en_memoire(Arc::new(HorlogeFixe::a("2026-09-26", 9, 0))).unwrap();
    let (proprietaire, code) = auth::installer_proprietaire(&mut db, "Mariam", "1234", "ancien123", "Chez Mariam").unwrap();
    Banc { db, proprietaire, code, cle: SigningKey::from_bytes(&[7; 32]) }
}

impl Banc {
    fn jeton(&mut self) -> String {
        auth::connexion_pin(&mut self.db, &self.proprietaire, "1234", None).unwrap().jeton
    }
    fn reinitialiser(&mut self, jeton: &str, preuve: Preuve, nouveau: &str) -> Result<String, Erreur> {
        let cle = self.cle.verifying_key();
        secours::reinitialiser_avec_cle(&mut self.db, jeton, preuve, nouveau, &cle)
    }
}

#[test]
fn rg_aut_07_code_de_secours_donne_a_l_installation() {
    let mut b = banc();
    assert!(secours::code_existe(b.db.conn()).unwrap());
    assert_eq!(b.code.len(), 19, "XXXX-XXXX-XXXX-XXXX : {}", b.code);
    let jeton = b.jeton();
    // Minuscules et sans tirets : c'est le même code.
    let saisi = b.code.to_lowercase().replace('-', " ");
    let nouveau_code = b.reinitialiser(&jeton, Preuve::CodeSecours(&saisi), "nouveau123").unwrap();
    assert_ne!(nouveau_code, b.code);
    // Nouveau mot de passe actif, session confirmée, ancien refusé.
    assert!(auth::session_courante(&b.db, &jeton).unwrap().eleve);
    let autre = b.jeton();
    assert!(auth::elever_session(&mut b.db, &autre, "ancien123").is_err());
    let autre = b.jeton();
    assert!(auth::elever_session(&mut b.db, &autre, "nouveau123").unwrap().eleve);
    // Le code a servi : il ne marche plus, le nouveau oui.
    let j = b.jeton();
    assert!(b.reinitialiser(&j, Preuve::CodeSecours(&b.code.clone()), "encore123").is_err());
    assert!(b.reinitialiser(&j, Preuve::CodeSecours(&nouveau_code), "encore123").is_ok());
}

#[test]
fn rg_aut_07_reponse_du_fournisseur_signee() {
    let mut b = banc();
    let jeton = b.jeton();
    let demande = secours::code_demande(b.db.conn()).unwrap();
    assert_eq!(secours::code_demande(b.db.conn()).unwrap(), demande, "même demande tant qu'elle n'a pas servi");
    // Signée par une autre clé, ou pour une autre demande : refusée.
    let fausse = secours::signer_reponse(&demande, &SigningKey::from_bytes(&[9; 32]));
    assert!(b.reinitialiser(&jeton, Preuve::ReponseFournisseur(&fausse), "nouveau123").is_err());
    let autre_demande = secours::signer_reponse("AAAA-BBBB-CCCC", &b.cle);
    assert!(b.reinitialiser(&jeton, Preuve::ReponseFournisseur(&autre_demande), "nouveau123").is_err());
    let reponse = secours::signer_reponse(&demande, &b.cle);
    b.reinitialiser(&jeton, Preuve::ReponseFournisseur(&reponse), "nouveau123").unwrap();
    // Réponse à usage unique : une nouvelle demande est tirée.
    assert_ne!(secours::code_demande(b.db.conn()).unwrap(), demande);
    let j = b.jeton();
    assert!(b.reinitialiser(&j, Preuve::ReponseFournisseur(&reponse), "encore123").is_err());
}

#[test]
fn rg_aut_07_reserve_au_proprietaire_et_verrouille_apres_cinq_echecs() {
    let mut b = banc();
    let proprio = Acteur::utilisateur(&b.proprietaire).avec_eleve(true);
    let gerant = auth::creer_utilisateur(
        &mut b.db,
        &proprio,
        &NouvelUtilisateur { nom: "Adama".into(), role_code: "gerant".into(), pin: "2222".into(), mot_de_passe: None, employe_id: None },
    )
    .unwrap();
    let jg = auth::connexion_pin(&mut b.db, &gerant, "2222", None).unwrap().jeton;
    let code = b.code.clone();
    let e = b.reinitialiser(&jg, Preuve::CodeSecours(&code), "nouveau123").unwrap_err();
    assert!(matches!(e, Erreur::Regle { regle: "RG-AUT-07", .. }), "{e}");

    let j = b.jeton();
    for _ in 0..4 {
        assert!(matches!(b.reinitialiser(&j, Preuve::CodeSecours("FAUX-FAUX-FAUX-FAUX"), "nouveau123"), Err(Erreur::Validation(_))));
    }
    assert!(matches!(b.reinitialiser(&j, Preuve::CodeSecours("FAUX-FAUX-FAUX-FAUX"), "nouveau123"), Err(Erreur::Verrouille)));
    // Verrouillé : même le bon code attend la fin du délai.
    assert!(matches!(b.reinitialiser(&j, Preuve::CodeSecours(&code), "nouveau123"), Err(Erreur::Verrouille)));
}

#[test]
fn rg_aut_07_nouveau_code_depuis_l_administration() {
    let mut b = banc();
    let sans_mot_de_passe = Acteur::utilisateur(&b.proprietaire);
    assert!(secours::renouveler_code(&mut b.db, &sans_mot_de_passe).is_err());
    let proprio = Acteur::utilisateur(&b.proprietaire).avec_eleve(true);
    let nouveau = secours::renouveler_code(&mut b.db, &proprio).unwrap();
    let j = b.jeton();
    let ancien = b.code.clone();
    assert!(b.reinitialiser(&j, Preuve::CodeSecours(&ancien), "nouveau123").is_err());
    assert!(b.reinitialiser(&j, Preuve::CodeSecours(&nouveau), "nouveau123").is_ok());
}
