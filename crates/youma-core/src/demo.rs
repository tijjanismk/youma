//! Base de démonstration (formation, prospection, tests d'interface).
//! Toujours dans un fichier séparé : jamais mélangée aux vraies données.

use rusqlite::params;

use crate::achats::{self, LigneAchatSaisie, NouvelAchat};
use crate::auth::{self, NouvelUtilisateur};
use crate::catalogue::{self, Categorie, GroupeOptions, OptionProduit, Poste, PrixZone, Produit};
use crate::clients::{self, Client};
use crate::db::{Acteur, Db};
use crate::employes::{self, Employe};
use crate::erreur::Resultat;
use crate::parametres::{self, QuartierLivraison};
use crate::salle::{self, Zone};
use crate::recettes;
use crate::stock::{self, Article, Conditionnement};

pub struct Demo {
    pub proprietaire: String,
    pub gerant: String,
    pub caissier: String,
    pub serveur: String,
}

fn produit(categorie: &str, nom: &str, prix: i64, poste: Option<&str>) -> Produit {
    Produit {
        id: String::new(),
        categorie_id: categorie.into(),
        nom: nom.into(),
        nom_court: String::new(),
        description: String::new(),
        photo: String::new(),
        prix,
        poste_id: poste.map(str::to_owned),
        disponible: true,
        actif: true,
        code: String::new(),
        code_barres: String::new(),
        taux_tva_bp: 0,
        suivi_stock: "aucun".into(),
        article_stock_id: None,
        prix_achat_estime: 0,
        ordre: 0,
        prix_zones: vec![],
        groupes_options: vec![],
    }
}

fn employe(nom: &str, fonction: &str, remuneration: &str, montant: i64, contrat: &str) -> Employe {
    Employe {
        id: String::new(),
        nom: nom.into(),
        surnom: String::new(),
        telephone: String::new(),
        fonction: fonction.into(),
        date_embauche: None,
        type_remuneration: remuneration.into(),
        montant_base: montant,
        type_contrat: contrat.into(),
        date_fin_contrat: None,
        piece_identite: String::new(),
        contact_urgence: String::new(),
        quartier: String::new(),
        declare_inps: false,
        numero_inps: String::new(),
        affilie_amo: false,
        numero_amo: String::new(),
        avantages_nature: String::new(),
        plafond_avance: None,
        horaires: String::new(),
        statut: "actif".into(),
        date_depart: None,
        notes: String::new(),
        solde: 0,
    }
}

/// Remplit une base vide. PIN : propriétaire 1234, gérant 2222, caissier 3333, serveuse 4444, cuisinier 5555.
/// Mots de passe d'administration (RG-AUT-06) : propriétaire « baobab123 », gérant « adama123 ».
pub fn remplir(db: &mut Db) -> Resultat<Demo> {
    let proprietaire = auth::installer_proprietaire(db, "Mariam (propriétaire)", "1234", "baobab123", "Maquis Le Baobab")?;
    let sys = Acteur::systeme();
    db.conn().execute("INSERT OR REPLACE INTO systeme(cle, valeur) VALUES ('demo', '1')", [])?;
    db.conn().execute(
        "UPDATE restaurant SET adresse = 'Hamdallaye ACI 2000, Bamako', telephone = '+223 70 00 00 00'",
        [],
    )?;
    let mut p = parametres::lire(db.conn())?;
    p.quartiers = vec![
        QuartierLivraison { nom: "Hamdallaye".into(), frais: 500 },
        QuartierLivraison { nom: "Badalabougou".into(), frais: 1_000 },
        QuartierLivraison { nom: "Kalaban Coura".into(), frais: 1_500 },
    ];
    parametres::ecrire(db.conn(), &p)?;

    let u = |nom: &str, role: &str, pin: &str| NouvelUtilisateur {
        nom: nom.into(),
        role_code: role.into(),
        pin: pin.into(),
        mot_de_passe: None,
        employe_id: None,
    };
    let gerant = auth::creer_utilisateur(db, &sys, &NouvelUtilisateur { mot_de_passe: Some("adama123".into()), ..u("Adama (gérant)", "gerant", "2222") })?;
    let caissier = auth::creer_utilisateur(db, &sys, &u("Kadi (caisse)", "caissier", "3333"))?;
    let serveur = auth::creer_utilisateur(db, &sys, &u("Awa", "serveur", "4444"))?;
    auth::creer_utilisateur(db, &sys, &u("Moussa (grill)", "cuisinier", "5555"))?;

    let poste = |db: &mut Db, nom: &str| {
        catalogue::enregistrer_poste(db, &sys, &Poste { id: String::new(), nom: nom.into(), imprimante: String::new(), ecran: true, actif: true })
    };
    let cuisine = poste(db, "Cuisine")?;
    let grill = poste(db, "Grill")?;
    let bar = poste(db, "Bar")?;

    let zone = |db: &mut Db, nom: &str, ordre: i64| salle::enregistrer_zone(db, &sys, &Zone { id: String::new(), nom: nom.into(), ordre, actif: true });
    let z_salle = zone(db, "Salle", 0)?;
    let z_terrasse = zone(db, "Terrasse", 1)?;
    let z_vip = zone(db, "VIP climatisé", 2)?;
    salle::creer_tables_serie(db, &sys, &z_salle, "", 1, 8)?;
    salle::creer_tables_serie(db, &sys, &z_terrasse, "T", 1, 6)?;
    salle::creer_tables_serie(db, &sys, &z_vip, "V", 1, 3)?;

    let cat = |db: &mut Db, nom: &str, couleur: &str, icone: &str, ordre: i64| {
        catalogue::enregistrer_categorie(db, &sys, &Categorie { id: String::new(), nom: nom.into(), couleur: couleur.into(), icone: icone.into(), ordre, actif: true })
    };
    let c_boissons = cat(db, "Boissons", "#1565c0", "🥤", 0)?;
    let c_bieres = cat(db, "Bières", "#f9a825", "🍺", 1)?;
    let c_grill = cat(db, "Grillades", "#c62828", "🍗", 2)?;
    let c_plats = cat(db, "Plats", "#2e7d32", "🍛", 3)?;
    let c_acc = cat(db, "Accompagnements", "#6d4c41", "🍟", 4)?;

    let article = |db: &mut Db, nom: &str, cond: &str, contenance: i64, seuil: i64| {
        stock::enregistrer_article(
            db,
            &sys,
            &Article {
                id: String::new(),
                nom: nom.into(),
                unite: "bouteille".into(),
                seuil_alerte: seuil,
                cout_unitaire: 0,
                famille: "Boissons".into(),
                actif: true,
                conditionnements: vec![Conditionnement { id: String::new(), nom: cond.into(), contenance }],
            },
        )
    };
    let a_coca = article(db, "Coca-Cola 33 cl", "Casier de 24", 24, 12)?;
    let a_eau = article(db, "Eau minérale 1,5 L", "Pack de 6", 6, 6)?;
    let a_biere = article(db, "Bière blonde 65 cl", "Casier de 12", 12, 12)?;
    let a_jus = article(db, "Jus de bissap (bouteille)", "Carton de 12", 12, 6)?;

    let revendu = |mut p: Produit, article: &str, zones: Vec<PrixZone>| {
        p.suivi_stock = "revendu".into();
        p.article_stock_id = Some(article.into());
        p.prix_zones = zones;
        p
    };
    let vip = |prix: i64| vec![PrixZone { zone_id: z_vip.clone(), prix }];
    catalogue::enregistrer_produit(db, &sys, &revendu(produit(&c_boissons, "Coca-Cola", 750, Some(&bar)), &a_coca, vip(1_000)))?;
    catalogue::enregistrer_produit(db, &sys, &revendu(produit(&c_boissons, "Eau minérale", 500, Some(&bar)), &a_eau, vip(750)))?;
    catalogue::enregistrer_produit(db, &sys, &revendu(produit(&c_boissons, "Bissap", 500, Some(&bar)), &a_jus, vec![]))?;
    catalogue::enregistrer_produit(db, &sys, &revendu(produit(&c_bieres, "Bière blonde", 1_000, Some(&bar)), &a_biere, vip(1_500)))?;
    let mut poulet = produit(&c_grill, "Poulet braisé", 3_500, Some(&grill));
    poulet.groupes_options = vec![GroupeOptions {
        id: String::new(),
        nom: "Cuisson".into(),
        min_choix: 0,
        max_choix: 1,
        options: vec![
            OptionProduit { id: String::new(), nom: "Bien cuit".into(), supplement: 0 },
            OptionProduit { id: String::new(), nom: "Pimenté".into(), supplement: 0 },
        ],
    }];
    poulet.prix_achat_estime = 2_000;
    catalogue::enregistrer_produit(db, &sys, &poulet)?;
    catalogue::enregistrer_produit(db, &sys, &produit(&c_grill, "Brochettes (3)", 1_500, Some(&grill)))?;
    catalogue::enregistrer_produit(db, &sys, &produit(&c_grill, "Poisson braisé", 4_000, Some(&grill)))?;
    catalogue::enregistrer_produit(db, &sys, &produit(&c_plats, "Riz sauce arachide", 1_500, Some(&cuisine)))?;
    catalogue::enregistrer_produit(db, &sys, &produit(&c_plats, "Tô sauce gombo", 1_000, Some(&cuisine)))?;
    catalogue::enregistrer_produit(db, &sys, &produit(&c_plats, "Fakoye", 1_500, Some(&cuisine)))?;
    let mut frites = produit(&c_acc, "Frites", 750, Some(&cuisine));
    frites.groupes_options = vec![GroupeOptions {
        id: String::new(),
        nom: "Taille".into(),
        min_choix: 1,
        max_choix: 1,
        options: vec![
            OptionProduit { id: String::new(), nom: "Normale".into(), supplement: 0 },
            OptionProduit { id: String::new(), nom: "Grande".into(), supplement: 500 },
        ],
    }];
    let frites_id = catalogue::enregistrer_produit(db, &sys, &frites)?;
    // Recette (fiche 0014) : ingrédients en unités fines (g, ml) pour rester en entiers.
    let ingredient = |db: &mut Db, nom: &str, unite: &str, cout: i64, cond: &str, contenance: i64| {
        stock::enregistrer_article(
            db,
            &sys,
            &Article {
                id: String::new(),
                nom: nom.into(),
                unite: unite.into(),
                seuil_alerte: 0,
                cout_unitaire: cout,
                famille: "Cuisine".into(),
                actif: true,
                conditionnements: vec![Conditionnement { id: String::new(), nom: cond.into(), contenance }],
            },
        )
    };
    let a_pdt = ingredient(db, "Pommes de terre", "g", 1, "Sac de 25 kg", 25_000)?;
    let a_huile = ingredient(db, "Huile", "ml", 2, "Bidon de 20 L", 20_000)?;
    let grande = catalogue::produit(db.conn(), &frites_id)?.groupes_options[0].options.iter().find(|o| o.nom == "Grande").map(|o| o.id.clone());
    let ligne = |a: &str, q: i64| recettes::LigneRecette { article_id: a.into(), quantite: q, article_nom: String::new(), unite: String::new(), cout_unitaire: 0 };
    recettes::definir(
        db,
        &sys,
        &recettes::Recette {
            produit_id: frites_id,
            lignes: vec![ligne(&a_pdt, 250), ligne(&a_huile, 30)],
            options: grande
                .map(|g| vec![recettes::RecetteOption { option_id: g, option_nom: String::new(), lignes: vec![ligne(&a_pdt, 150)] }])
                .unwrap_or_default(),
            cout: 0,
        },
    )?;
    catalogue::enregistrer_produit(db, &sys, &produit(&c_acc, "Alloco", 500, Some(&cuisine)))?;
    catalogue::enregistrer_produit(db, &sys, &produit(&c_acc, "Attiéké", 500, Some(&cuisine)))?;

    // Employés : la plupart sans contrat écrit ni INPS (réalité terrain).
    let mut awa = employe("Awa Traoré", "serveur", "mensuel", 50_000, "verbal");
    awa.surnom = "Awa".into();
    awa.avantages_nature = "Nourrie le midi".into();
    let awa_id = employes::enregistrer(db, &sys, &awa)?;
    db.conn().execute("UPDATE utilisateurs SET employe_id = ?1 WHERE id = ?2", params![awa_id, serveur])?;
    employes::enregistrer(db, &sys, &employe("Moussa Coulibaly", "grilleur", "journalier", 3_000, "aucun"))?;
    employes::enregistrer(db, &sys, &employe("Fanta Diarra", "plongeur", "journalier", 2_000, "aucun"))?;
    employes::enregistrer(db, &sys, &employe("Ibrahim Keïta", "livreur", "tache", 500, "aucun"))?;
    let mut adama = employe("Adama Sangaré", "gérant", "mensuel", 120_000, "cdi");
    adama.declare_inps = true;
    adama.affilie_amo = true;
    adama.numero_inps = "À compléter".into();
    employes::enregistrer(db, &sys, &adama)?;
    employes::enregistrer(db, &sys, &employe("Oumou (nièce)", "aide-cuisinier", "aucun", 0, "aucun"))?;

    clients::enregistrer(
        db,
        &sys,
        &Client {
            id: String::new(),
            nom: "Modibo Diallo".into(),
            telephone: Some("76000001".into()),
            adresse: "Hamdallaye".into(),
            reperes: "Près de la mosquée".into(),
            credit_autorise: true,
            limite_credit: 20_000,
            actif: true,
            dette: 0,
        },
    )?;
    clients::enregistrer(
        db,
        &sys,
        &Client {
            id: String::new(),
            nom: "Salif Konaté".into(),
            telephone: Some("66000002".into()),
            adresse: String::new(),
            reperes: String::new(),
            credit_autorise: false,
            limite_credit: 0,
            actif: true,
            dette: 0,
        },
    )?;
    let fournisseur = achats::enregistrer_fournisseur(
        db,
        &sys,
        &achats::Fournisseur { id: String::new(), nom: "Dépôt de boissons du quartier".into(), telephone: "70000003".into(), notes: String::new(), actif: true, dette: 0 },
    )?;
    // Stock initial reçu à crédit.
    let cond = |db: &Db, a: &str| -> Resultat<String> {
        Ok(db.conn().query_row("SELECT id FROM conditionnements WHERE article_id = ?1", params![a], |r| r.get(0))?)
    };
    let lignes = vec![
        LigneAchatSaisie { article_id: a_coca.clone(), conditionnement_id: Some(cond(db, &a_coca)?), quantite: 2, prix_total: 24_000 },
        LigneAchatSaisie { article_id: a_eau.clone(), conditionnement_id: Some(cond(db, &a_eau)?), quantite: 4, prix_total: 7_200 },
        LigneAchatSaisie { article_id: a_biere.clone(), conditionnement_id: Some(cond(db, &a_biere)?), quantite: 3, prix_total: 21_600 },
        LigneAchatSaisie { article_id: a_jus.clone(), conditionnement_id: Some(cond(db, &a_jus)?), quantite: 1, prix_total: 3_600 },
    ];
    achats::receptionner(db, &sys, &NouvelAchat { fournisseur_id: Some(fournisseur), mode: "credit".into(), compte_id: None, lignes, note: "Stock initial".into() })?;
    Ok(Demo { proprietaire, gerant, caissier, serveur })
}

pub fn est_demo(conn: &rusqlite::Connection) -> bool {
    conn.query_row("SELECT valeur FROM systeme WHERE cle = 'demo'", [], |r| r.get::<_, String>(0)).map(|v| v == "1").unwrap_or(false)
}
