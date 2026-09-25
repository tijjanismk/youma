//! Canaux de commande, zones à risque, liste noire et suivi en direct (fiche 0013).

mod commun;

use commun::*;
use youma_core::commandes::{self, InfosLivraison, LigneSaisie, NouvelleCommande};
use youma_core::entrantes::{self, CommandeEntrante};
use youma_core::zones_risque::{self, ZoneRisque};
use youma_core::{journee, livraison, parametres, salle};

fn activer(b: &mut Banc, f: impl FnOnce(&mut parametres::Canaux)) {
    let mut p = parametres::lire(b.db.conn()).unwrap();
    f(&mut p.canaux);
    let a = b.proprietaire();
    parametres::modifier(&mut b.db, &a, &p).unwrap();
}

fn zone(nom: &str, quartier: Option<&str>, debut_h: i64, fin_h: i64, action: &str) -> ZoneRisque {
    ZoneRisque {
        id: String::new(),
        nom: nom.into(),
        quartier: quartier.map(Into::into),
        lat: None,
        lon: None,
        rayon_m: None,
        debut_min: debut_h * 60,
        fin_min: fin_h * 60,
        jours: 127,
        action: action.into(),
        message: String::new(),
        actif: true,
    }
}

fn en_ligne(b: &Banc, quartier: &str, telephone: &str, mode: &str) -> CommandeEntrante {
    CommandeEntrante {
        origine_id: None,
        code_suivi: None,
        canal: "en_ligne".into(),
        code_table: None,
        type_: Some("livraison".into()),
        client_nom: "Awa".into(),
        telephone: telephone.into(),
        telephone_verifie: false,
        livraison: Some(livraison_a(quartier, telephone)),
        paiement_mode: Some(mode.into()),
        paiement_operateur: None,
        paiement_reference: None,
        lignes: vec![b.ligne("Brochettes (3)", 2)],
        note: String::new(),
    }
}

fn livraison_a(quartier: &str, telephone: &str) -> InfosLivraison {
    InfosLivraison { quartier: quartier.into(), repere: "Près de la pharmacie".into(), telephone: telephone.into(), frais: None, lat: None, lon: None }
}

fn qr(b: &Banc, code: &str, lignes: Vec<LigneSaisie>) -> CommandeEntrante {
    CommandeEntrante {
        origine_id: None,
        code_suivi: None,
        canal: "qr_table".into(),
        code_table: Some(code.into()),
        type_: None,
        client_nom: String::new(),
        telephone: String::new(),
        telephone_verifie: false,
        livraison: None,
        paiement_mode: None,
        paiement_operateur: None,
        paiement_reference: None,
        lignes: if lignes.is_empty() { vec![b.ligne("Brochettes (3)", 1)] } else { lignes },
        note: String::new(),
    }
}

fn recu(b: &mut Banc, quartier: &str, telephone: &str, mode: &str) -> entrantes::Reponse {
    let e = en_ligne(b, quartier, telephone, mode);
    entrantes::recevoir(&mut b.db, &e).unwrap()
}

fn recu_qr(b: &mut Banc, code: &str, lignes: Vec<LigneSaisie>) -> entrantes::Reponse {
    let e = qr(b, code, lignes);
    entrantes::recevoir(&mut b.db, &e).unwrap()
}

fn id_par_numero(b: &Banc, numero: i64) -> String {
    b.db.conn().query_row("SELECT id FROM commandes WHERE numero = ?1", [numero], |r| r.get(0)).unwrap()
}

fn nouvelle_livraison(quartier: &str, telephone: &str) -> NouvelleCommande {
    NouvelleCommande {
        type_: "livraison".into(),
        table_id: None,
        client_id: None,
        employe_id: None,
        couverts: 0,
        note: String::new(),
        livraison: Some(livraison_a(quartier, telephone)),
        canal: Some("telephone".into()),
    }
}

#[test]
fn rg_can_01_papier_toujours_telephone_activable() {
    let mut b = banc();
    b.ouvrir_journee();
    let a = b.serveur();
    let mut n = nouvelle_livraison("Hamdallaye", "76000001");
    assert!(commandes::ouvrir(&mut b.db, &a, &n).is_ok());
    activer(&mut b, |c| c.telephone = false);
    let e = commandes::ouvrir(&mut b.db, &a, &n).unwrap_err();
    assert_eq!(e.regle_code(), Some("RG-CAN-01"));
    // Le menu papier (commande saisie par le serveur) reste toujours possible.
    n.canal = None;
    assert!(commandes::ouvrir(&mut b.db, &a, &n).is_ok());
    // Les canaux « QR » et « en ligne » ne passent jamais par la saisie du personnel.
    n.canal = Some("en_ligne".into());
    assert!(commandes::ouvrir(&mut b.db, &a, &n).is_err());
}

#[test]
fn rg_can_02_qr_table_file_de_validation_et_addition_unique() {
    let mut b = banc();
    b.ouvrir_journee();
    // QR désactivé par défaut.
    let r = recu_qr(&mut b, "XXXXXX", vec![]);
    assert_eq!(r.statut, "refusee");
    activer(&mut b, |c| c.qr_table = true);
    let a = b.proprietaire();
    assert_eq!(entrantes::generer_codes_qr(&mut b.db, &a, false).unwrap(), 17);
    assert_eq!(entrantes::generer_codes_qr(&mut b.db, &a, false).unwrap(), 0);
    let t3 = b.table("3");
    let code: String = b.db.conn().query_row("SELECT code_qr FROM tables_salle WHERE id = ?1", [&t3], |r| r.get(0)).unwrap();

    let menu = entrantes::menu_public(b.db.conn(), Some(&code.to_lowercase()), b.db.maintenant()).unwrap();
    assert_eq!(menu.table.as_deref(), Some("3"));
    assert!(menu.ouvert && menu.qr_table && !menu.en_ligne);

    assert_eq!(recu_qr(&mut b, "ZZZZZZ", vec![]).statut, "refusee");
    let r = recu_qr(&mut b, &code, vec![]);
    assert_eq!(r.statut, "en_attente");
    let id = id_par_numero(&b, r.numero.unwrap());
    // En attente : ni en cuisine, ni sur le plan de salle, ni dans la liste des additions.
    assert_eq!(b.compter("SELECT COUNT(*) FROM envois"), 0);
    assert!(salle::plan(b.db.conn()).unwrap().iter().all(|t| t.statut != "occupee"));
    let j = journee::ouverte(b.db.conn()).unwrap().unwrap();
    assert!(commandes::lister(b.db.conn(), &j.id, None).unwrap().is_empty());
    assert_eq!(entrantes::file(b.db.conn()).unwrap().len(), 1);
    // RG-JOU-04 : pas de clôture avec des commandes en attente.
    let c = b.caissier();
    assert_eq!(journee::cloturer(&mut b.db, &c).unwrap_err().regle_code(), Some("RG-JOU-04"));

    // Le serveur ne valide pas ; le refus exige un motif.
    let s = b.serveur();
    assert!(entrantes::valider(&mut b.db, &s, &id, true, "").is_err());
    assert_eq!(entrantes::valider(&mut b.db, &c, &id, false, "").unwrap_err().regle_code(), Some("RG-CAN-02"));
    entrantes::valider(&mut b.db, &c, &id, true, "").unwrap();
    let cmd = commandes::detail(b.db.conn(), &id).unwrap();
    assert_eq!(cmd.table_id.as_deref(), Some(t3.as_str()));
    assert_eq!(cmd.envois.len(), 1);
    assert!(entrantes::valider(&mut b.db, &c, &id, true, "").is_err(), "déjà traitée");

    // Deuxième commande depuis la même table : ajoutée à l'addition ouverte (une addition par table).
    let deux = vec![b.ligne("Brochettes (3)", 2)];
    let r2 = recu_qr(&mut b, &code, deux);
    let id2 = id_par_numero(&b, r2.numero.unwrap());
    entrantes::valider(&mut b.db, &c, &id2, true, "").unwrap();
    let cmd = commandes::detail(b.db.conn(), &id).unwrap();
    assert_eq!(cmd.envois.len(), 2);
    assert_eq!(cmd.lignes.iter().map(|l| l.quantite).sum::<i64>(), 3);
    assert_eq!(b.compter("SELECT COUNT(*) FROM commandes WHERE table_id IS NOT NULL AND statut = 'ouverte'"), 1);
    assert_eq!(entrantes::suivi(b.db.conn(), r2.code_suivi.as_deref().unwrap()).unwrap().etape, "acceptee");
}

#[test]
fn rg_can_02_prix_recalcules_et_refus_journalise() {
    let mut b = banc();
    activer(&mut b, |c| c.en_ligne = true);
    // Journée fermée : refus.
    let e = en_ligne(&b, "Hamdallaye", "76000001", "a_la_livraison");
    assert_eq!(entrantes::recevoir(&mut b.db, &e).unwrap().statut, "refusee");
    b.ouvrir_journee();
    let r = entrantes::recevoir(&mut b.db, &e).unwrap();
    assert_eq!(r.statut, "en_attente");
    let prix: i64 = b.db.conn().query_row("SELECT prix FROM produits WHERE nom = 'Brochettes (3)'", [], |r| r.get(0)).unwrap();
    assert_eq!(r.total, 2 * prix + 500, "prix du catalogue + frais du quartier");
    assert!(b.compter("SELECT COUNT(*) FROM journal_audit WHERE action = 'commande_entrante.refusee'") >= 1);
    // Refus par le personnel : la commande est annulée, le client voit le motif.
    let c = b.caissier();
    let id = id_par_numero(&b, r.numero.unwrap());
    entrantes::valider(&mut b.db, &c, &id, false, "Plus de brochettes").unwrap();
    let s = entrantes::suivi(b.db.conn(), r.code_suivi.as_deref().unwrap()).unwrap();
    assert_eq!((s.etape.as_str(), s.motif.as_deref()), ("refusee", Some("Plus de brochettes")));
}

#[test]
fn rg_can_02_idempotence_par_origine() {
    let mut b = banc();
    b.ouvrir_journee();
    activer(&mut b, |c| c.en_ligne = true);
    let mut e = en_ligne(&b, "Hamdallaye", "76000001", "a_la_livraison");
    e.origine_id = Some("relais-42".into());
    let r1 = entrantes::recevoir(&mut b.db, &e).unwrap();
    let r2 = entrantes::recevoir(&mut b.db, &e).unwrap();
    assert_eq!(r1.numero, r2.numero);
    assert_eq!(r1.code_suivi, r2.code_suivi);
    assert_eq!(b.compter("SELECT COUNT(*) FROM commandes"), 1);
}

#[test]
fn rg_can_03_liste_noire() {
    let mut b = banc();
    b.ouvrir_journee();
    activer(&mut b, |c| c.en_ligne = true);
    let s = b.serveur();
    assert!(zones_risque::bloquer_numero(&mut b.db, &s, "76000001", "Fausses commandes").is_err());
    let g = b.gerant();
    zones_risque::bloquer_numero(&mut b.db, &g, "+223 76 00 00 01", "Fausses commandes").unwrap();
    assert_eq!(zones_risque::numeros_bloques(b.db.conn()).unwrap()[0].0, "76000001");

    let r = recu(&mut b, "Hamdallaye", "0022376000001", "a_la_livraison");
    assert_eq!(r.statut, "refusee");
    // Saisie par le personnel (téléphone) : accord d'un responsable.
    let e = commandes::ouvrir(&mut b.db, &s, &nouvelle_livraison("Hamdallaye", "76 00 00 01")).unwrap_err();
    assert_eq!(code(&e), "AUTORISATION_REQUISE");
    let s_pin = Banc::avec_pin_gerant(s.clone());
    commandes::ouvrir(&mut b.db, &s_pin, &nouvelle_livraison("Hamdallaye", "76 00 00 01")).unwrap();
    assert_eq!(b.compter("SELECT COUNT(*) FROM journal_audit WHERE action = 'numero.outrepasser'"), 1);

    zones_risque::debloquer_numero(&mut b.db, &g, "76000001").unwrap();
    assert_eq!(recu(&mut b, "Hamdallaye", "76000001", "a_la_livraison").statut, "en_attente");
}

#[test]
fn rg_can_04_verification_du_numero_par_sms() {
    let mut b = banc();
    b.ouvrir_journee();
    activer(&mut b, |c| {
        c.en_ligne = true;
        c.verification_numero = "sms".into();
    });
    let mut e = en_ligne(&b, "Hamdallaye", "76000001", "a_la_livraison");
    assert_eq!(entrantes::recevoir(&mut b.db, &e).unwrap().statut, "refusee");
    e.telephone_verifie = true;
    assert_eq!(entrantes::recevoir(&mut b.db, &e).unwrap().statut, "en_attente");
}

#[test]
fn rg_can_05_paiement_avance_ou_a_la_livraison() {
    let mut b = banc();
    b.ouvrir_journee();
    b.ouvrir_caisse(10_000);
    activer(&mut b, |c| {
        c.en_ligne = true;
        c.avance_nouveau_client = true;
    });
    // Nouveau client : Mobile Money d'avance obligatoire.
    let r = recu(&mut b, "Hamdallaye", "76000001", "a_la_livraison");
    assert_eq!(r.statut, "refusee");
    let mut e = en_ligne(&b, "Hamdallaye", "76000001", "avance");
    assert_eq!(entrantes::recevoir(&mut b.db, &e).unwrap().statut, "refusee", "référence obligatoire");
    e.paiement_operateur = Some("Orange Money".into());
    e.paiement_reference = Some("OM-778899".into());
    let r = entrantes::recevoir(&mut b.db, &e).unwrap();
    assert_eq!(r.statut, "en_attente");
    let id = id_par_numero(&b, r.numero.unwrap());
    // Acceptation : le paiement Mobile Money est encaissé, la commande part en cuisine.
    let c = b.caissier();
    entrantes::valider(&mut b.db, &c, &id, true, "").unwrap();
    let cmd = commandes::detail(b.db.conn(), &id).unwrap();
    assert_eq!(cmd.statut, "payee");
    assert_eq!(cmd.envois.len(), 1);
    assert_eq!(b.solde("Orange Money"), r.total);
    // RG-CAI-05 : la même référence ne peut pas servir deux fois.
    e.origine_id = None;
    let r2 = entrantes::recevoir(&mut b.db, &e).unwrap();
    let id2 = id_par_numero(&b, r2.numero.unwrap());
    assert!(entrantes::valider(&mut b.db, &c, &id2, true, "").is_err());

    // Plafond du paiement à la livraison.
    activer(&mut b, |c| {
        c.avance_nouveau_client = false;
        c.plafond_paiement_livraison = 1_000;
    });
    assert_eq!(recu(&mut b, "Hamdallaye", "76000002", "a_la_livraison").statut, "refusee");
    activer(&mut b, |c| c.plafond_paiement_livraison = 0);
    assert_eq!(recu(&mut b, "Hamdallaye", "76000002", "a_la_livraison").statut, "en_attente");
    // Paiement à la livraison désactivé.
    activer(&mut b, |c| c.paiement_a_la_livraison = false);
    assert_eq!(recu(&mut b, "Hamdallaye", "76000003", "a_la_livraison").statut, "refusee");
}

#[test]
fn rg_zon_01_zone_invalide() {
    let mut b = banc();
    let g = b.gerant();
    let e = zones_risque::enregistrer(&mut b.db, &g, &zone("Nulle part", None, 21, 6, "bloquer")).unwrap_err();
    assert_eq!(e.regle_code(), Some("RG-ZON-01"));
    let e = zones_risque::enregistrer(&mut b.db, &g, &zone("X", Some("Hamdallaye"), 8, 8, "bloquer")).unwrap_err();
    assert_eq!(e.regle_code(), Some("RG-ZON-01"));
    let c = b.caissier();
    assert!(zones_risque::enregistrer(&mut b.db, &c, &zone("X", Some("Hamdallaye"), 21, 6, "bloquer")).is_err());
}

#[test]
fn rg_zon_02_blocage_la_nuit_dans_un_quartier() {
    let mut b = banc();
    b.ouvrir_journee();
    activer(&mut b, |c| c.en_ligne = true);
    let g = b.gerant();
    zones_risque::enregistrer(&mut b.db, &g, &zone("Kalaban la nuit", Some("kalaban coura"), 21, 6, "bloquer")).unwrap();
    // 10 h : la zone ne s'applique pas.
    assert_eq!(recu(&mut b, "Kalaban Coura", "76000001", "a_la_livraison").statut, "en_attente");
    // 22 h : bloquée, avec un message clair.
    b.horloge.regler_a("2026-03-14", 22, 0);
    let r = recu(&mut b, "Kalaban Coura", "76000001", "a_la_livraison");
    assert_eq!(r.statut, "refusee");
    assert!(r.message.contains("Kalaban la nuit"), "{}", r.message);
    // Autre quartier : pas concerné.
    assert_eq!(recu(&mut b, "Hamdallaye", "76000001", "a_la_livraison").statut, "en_attente");
    // À emporter : pas concerné.
    let mut e = en_ligne(&b, "Kalaban Coura", "76000001", "a_la_livraison");
    e.type_ = Some("emporter".into());
    e.livraison = None;
    assert_eq!(entrantes::recevoir(&mut b.db, &e).unwrap().statut, "en_attente");
    // Après minuit, toujours bloqué (plage 21 h → 6 h).
    b.horloge.regler_a("2026-03-15", 1, 30);
    assert_eq!(recu(&mut b, "Kalaban Coura", "76000001", "a_la_livraison").statut, "refusee");
    // RG-ZON-03 : saisie par le personnel → accord d'un responsable.
    let s = b.serveur();
    let e = commandes::ouvrir(&mut b.db, &s, &nouvelle_livraison("Kalaban Coura", "76000009")).unwrap_err();
    assert_eq!(code(&e), "AUTORISATION_REQUISE");
    commandes::ouvrir(&mut b.db, &Banc::avec_pin_gerant(s), &nouvelle_livraison("Kalaban Coura", "76000009")).unwrap();
    assert_eq!(b.compter("SELECT COUNT(*) FROM journal_audit WHERE action = 'zone.outrepasser'"), 1);
}

#[test]
fn rg_zon_02_jours_et_zone_la_plus_stricte() {
    let mut b = banc();
    b.ouvrir_journee();
    activer(&mut b, |c| c.en_ligne = true);
    let g = b.gerant();
    // Samedi 14 mars 2026 : bit 32. Zone active le vendredi seulement.
    let mut vendredi = zone("Vendredi soir", Some("Badalabougou"), 0, 24, "bloquer");
    vendredi.jours = 16;
    zones_risque::enregistrer(&mut b.db, &g, &vendredi).unwrap();
    assert!(zones_risque::evaluer(b.db.conn(), Some("Badalabougou"), None, b.db.maintenant()).unwrap().is_none());
    zones_risque::enregistrer(&mut b.db, &g, &zone("Contrôle", Some("Badalabougou"), 8, 20, "validation_manuelle")).unwrap();
    zones_risque::enregistrer(&mut b.db, &g, &zone("Avance", Some("Badalabougou"), 9, 12, "paiement_avance")).unwrap();
    let d = zones_risque::evaluer(b.db.conn(), Some("Badalabougou"), None, b.db.maintenant()).unwrap().unwrap();
    assert_eq!(d.action, "paiement_avance");
    // Saisie par le personnel : l'addition doit être payée avant l'envoi.
    let s = b.serveur();
    let id = commandes::ouvrir(&mut b.db, &s, &nouvelle_livraison("Badalabougou", "76000009")).unwrap();
    assert_eq!(commandes::detail(b.db.conn(), &id).unwrap().ordre_paiement, "avant");
}

#[test]
fn rg_zon_02_cercle_gps_et_validation_par_responsable() {
    let mut b = banc();
    b.ouvrir_journee();
    activer(&mut b, |c| c.en_ligne = true);
    let g = b.gerant();
    let mut z = zone("Autour du marché", None, 0, 24, "validation_manuelle");
    (z.lat, z.lon, z.rayon_m) = (Some(12_639_000), Some(-8_002_000), Some(500));
    zones_risque::enregistrer(&mut b.db, &g, &z).unwrap();
    let mut e = en_ligne(&b, "Hamdallaye", "76000001", "a_la_livraison");
    e.livraison.as_mut().unwrap().lat = Some(12_640_000);
    e.livraison.as_mut().unwrap().lon = Some(-8_002_000);
    let r = entrantes::recevoir(&mut b.db, &e).unwrap();
    assert_eq!(r.statut, "en_attente");
    let id = id_par_numero(&b, r.numero.unwrap());
    let f = entrantes::file(b.db.conn()).unwrap();
    assert!(f[0].validation_responsable);
    // La caissière accepte d'ordinaire, mais ici il faut un responsable.
    let c = b.caissier();
    assert_eq!(code(&entrantes::valider(&mut b.db, &c, &id, true, "").unwrap_err()), "AUTORISATION_REQUISE");
    entrantes::valider(&mut b.db, &Banc::avec_pin_gerant(c), &id, true, "").unwrap();
    // Loin du cercle : pas de contrôle.
    let mut e = en_ligne(&b, "Hamdallaye", "76000002", "a_la_livraison");
    e.livraison.as_mut().unwrap().lat = Some(12_660_000);
    e.livraison.as_mut().unwrap().lon = Some(-8_002_000);
    entrantes::recevoir(&mut b.db, &e).unwrap();
    assert!(!entrantes::file(b.db.conn()).unwrap()[0].validation_responsable);
}

#[test]
fn rg_liv_04_suivi_en_direct_et_position_du_livreur() {
    let mut b = banc();
    b.ouvrir_journee();
    activer(&mut b, |c| c.en_ligne = true);
    let r = recu(&mut b, "Hamdallaye", "76000001", "a_la_livraison");
    let code_suivi = r.code_suivi.clone().unwrap();
    assert_eq!(entrantes::suivi(b.db.conn(), &code_suivi).unwrap().etape, "recue");
    let id = id_par_numero(&b, r.numero.unwrap());
    let c = b.caissier();
    entrantes::valider(&mut b.db, &c, &id, true, "").unwrap();
    assert_eq!(entrantes::suivi(b.db.conn(), &code_suivi).unwrap().etape, "en_preparation");

    let g = b.gerant();
    let liens = entrantes::liens(&mut b.db, &g, &id).unwrap();
    assert_eq!(liens.code_suivi, code_suivi);
    let code_livreur = liens.code_livreur.clone().unwrap();
    assert_ne!(code_livreur, code_suivi);
    assert_eq!(entrantes::liens(&mut b.db, &g, &id).unwrap(), liens, "codes stables");

    // Pas de position avant la course, ni avec le code du client.
    assert_eq!(entrantes::ajouter_position(&mut b.db, &code_livreur, 12_640_000, -8_000_000).unwrap_err().regle_code(), Some("RG-LIV-04"));
    let livreur = b.employe("Ibrahim Keïta");
    livraison::assigner(&mut b.db, &g, &id, &livreur).unwrap();
    livraison::changer_statut(&mut b.db, &g, &id, "en_route", "").unwrap();
    assert!(entrantes::ajouter_position(&mut b.db, &code_suivi, 12_640_000, -8_000_000).is_err());
    assert!(entrantes::ajouter_position(&mut b.db, &code_livreur, 99_000_000, 0).is_err());
    entrantes::ajouter_position(&mut b.db, &code_livreur, 12_640_000, -8_000_000).unwrap();
    b.horloge.avancer_minutes(1);
    entrantes::ajouter_position(&mut b.db, &code_livreur, 12_641_000, -8_000_500).unwrap();
    let s = entrantes::suivi(b.db.conn(), &code_suivi).unwrap();
    assert_eq!(s.etape, "en_route");
    assert_eq!(s.livreur.map(|l| (l.0, l.1)), Some((12_641_000, -8_000_500)));
    // Ajout seul.
    assert!(b.db.conn().execute("UPDATE positions_livreur SET lat = 0", []).is_err());

    livraison::changer_statut(&mut b.db, &g, &id, "livree", "").unwrap();
    let s = entrantes::suivi(b.db.conn(), &code_suivi).unwrap();
    assert_eq!(s.etape, "livree");
    assert!(s.livreur.is_none(), "la position n'est plus visible après la course");
    assert!(entrantes::ajouter_position(&mut b.db, &code_livreur, 12_640_000, -8_000_000).is_err());
    assert!(entrantes::suivi(b.db.conn(), "INCONNU").is_err());
}
