#![allow(dead_code)]

use std::sync::Arc;

use youma_core::caisse::{self, Encaissement, OuvertureSession, PartSaisie};
use youma_core::commandes::{self, LigneSaisie, NouvelleCommande};
use youma_core::demo::{self, Demo};
use youma_core::horloge::HorlogeFixe;
use youma_core::{journee, Acteur, Db};

pub struct Banc {
    pub db: Db,
    pub horloge: Arc<HorlogeFixe>,
    pub demo: Demo,
    pub dossier: tempfile::TempDir,
}

pub fn banc() -> Banc {
    let horloge = Arc::new(HorlogeFixe::a("2026-03-14", 10, 0));
    let dossier = tempfile::tempdir().unwrap();
    let mut db = Db::ouvrir(&dossier.path().join("youma.db"), horloge.clone()).unwrap();
    let demo = demo::remplir(&mut db).unwrap();
    Banc { db, horloge, demo, dossier }
}

impl Banc {
    /// Session confirmée par mot de passe (RG-AUT-06) : l'administration est accessible.
    pub fn proprietaire(&self) -> Acteur {
        Acteur::utilisateur(&self.demo.proprietaire).avec_eleve(true)
    }
    pub fn gerant(&self) -> Acteur {
        Acteur::utilisateur(&self.demo.gerant)
    }
    pub fn caissier(&self) -> Acteur {
        Acteur::utilisateur(&self.demo.caissier)
    }
    pub fn serveur(&self) -> Acteur {
        Acteur::utilisateur(&self.demo.serveur)
    }
    /// Acteur avec autorisation ponctuelle du gérant (PIN 2222).
    pub fn avec_pin_gerant(a: Acteur) -> Acteur {
        a.avec_autorisation(Some("2222".into()))
    }

    pub fn ouvrir_journee(&mut self) -> journee::Journee {
        let a = self.caissier();
        journee::ouvrir(&mut self.db, &a).unwrap()
    }

    pub fn ouvrir_caisse(&mut self, fond: i64) -> String {
        let a = self.caissier();
        caisse::ouvrir_session(&mut self.db, &a, &OuvertureSession { compte_id: None, fond_compte: fond, billetage: vec![], motif_ecart: "Fond initial".into() }).unwrap()
    }

    pub fn produit(&self, nom: &str) -> String {
        self.db.conn().query_row("SELECT id FROM produits WHERE nom = ?1", [nom], |r| r.get(0)).unwrap()
    }

    pub fn table(&self, nom: &str) -> String {
        self.db.conn().query_row("SELECT id FROM tables_salle WHERE nom = ?1", [nom], |r| r.get(0)).unwrap()
    }

    pub fn compte(&self, nom: &str) -> String {
        self.db.conn().query_row("SELECT id FROM comptes_tresorerie WHERE nom = ?1", [nom], |r| r.get(0)).unwrap()
    }

    pub fn article(&self, nom: &str) -> String {
        self.db.conn().query_row("SELECT id FROM articles_stock WHERE nom = ?1", [nom], |r| r.get(0)).unwrap()
    }

    pub fn employe(&self, nom: &str) -> String {
        self.db.conn().query_row("SELECT id FROM employes WHERE nom = ?1", [nom], |r| r.get(0)).unwrap()
    }

    pub fn client(&self, nom: &str) -> String {
        self.db.conn().query_row("SELECT id FROM clients WHERE nom = ?1", [nom], |r| r.get(0)).unwrap()
    }

    pub fn ligne(&self, nom: &str, quantite: i64) -> LigneSaisie {
        LigneSaisie { produit_id: self.produit(nom), quantite, options: vec![], commentaire: String::new() }
    }

    /// Ouvre une addition à table (par le serveur), ajoute les lignes et envoie.
    pub fn commande_table(&mut self, table: &str, lignes: &[(&str, i64)]) -> String {
        let a = self.serveur();
        let t = self.table(table);
        let id = commandes::ouvrir(&mut self.db, &a, &NouvelleCommande { type_: "sur_place".into(), table_id: Some(t), client_id: None, employe_id: None, couverts: 2, note: String::new(), livraison: None }).unwrap();
        let l: Vec<LigneSaisie> = lignes.iter().map(|(n, q)| self.ligne(n, *q)).collect();
        commandes::ajouter_lignes(&mut self.db, &a, &id, &l).unwrap();
        commandes::envoyer(&mut self.db, &a, &id).unwrap();
        id
    }

    pub fn payer_especes(&mut self, commande_id: &str, montant: i64) -> caisse::ResultatEncaissement {
        let a = self.caissier();
        caisse::encaisser(&mut self.db, &a, &Encaissement { commande_id: commande_id.into(), parts: vec![especes(montant)], especes_recues: None }).unwrap()
    }

    pub fn solde(&self, compte: &str) -> i64 {
        caisse::solde(self.db.conn(), &self.compte(compte)).unwrap()
    }

    pub fn stock(&self, article: &str) -> i64 {
        youma_core::stock::quantite(self.db.conn(), &self.article(article)).unwrap()
    }

    pub fn total(&self, commande_id: &str) -> commandes::Totaux {
        commandes::totaux(self.db.conn(), commande_id).unwrap()
    }

    pub fn compter(&self, sql: &str) -> i64 {
        self.db.conn().query_row(sql, [], |r| r.get(0)).unwrap()
    }
}

pub fn especes(montant: i64) -> PartSaisie {
    PartSaisie { moyen: "especes".into(), montant, compte_id: None, reference: None, numero_payeur: None, client_id: None, par_livreur: false }
}

pub fn mobile_money(compte_id: &str, montant: i64, reference: &str) -> PartSaisie {
    PartSaisie { moyen: "mobile_money".into(), montant, compte_id: Some(compte_id.into()), reference: Some(reference.into()), numero_payeur: Some("70112233".into()), client_id: None, par_livreur: false }
}

pub fn credit(client_id: &str, montant: i64) -> PartSaisie {
    PartSaisie { moyen: "credit".into(), montant, compte_id: None, reference: None, numero_payeur: None, client_id: Some(client_id.into()), par_livreur: false }
}

pub fn code(e: &youma_core::Erreur) -> &'static str {
    e.code()
}
