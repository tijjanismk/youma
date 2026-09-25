use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::db::{trouver, Acteur, Db, Op};
use crate::erreur::{Erreur, Resultat};
use crate::permissions as perm;

// ───────────── Postes de préparation ─────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Poste {
    #[serde(default)]
    pub id: String,
    pub nom: String,
    #[serde(default)]
    pub imprimante: String,
    #[serde(default)]
    pub ecran: bool,
    #[serde(default = "vrai")]
    pub actif: bool,
}

fn vrai() -> bool {
    true
}

pub fn lister_postes(conn: &Connection) -> Resultat<Vec<Poste>> {
    let mut s = conn.prepare("SELECT id, nom, imprimante, ecran, actif FROM postes_preparation ORDER BY nom")?;
    let v = s
        .query_map([], |r| {
            Ok(Poste { id: r.get(0)?, nom: r.get(1)?, imprimante: r.get(2)?, ecran: r.get(3)?, actif: r.get(4)? })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(v)
}

pub fn enregistrer_poste(db: &mut Db, acteur: &Acteur, p: &Poste) -> Resultat<String> {
    db.executer(acteur, |op| {
        op.exiger(perm::CATALOGUE_GERER)?;
        non_vide(&p.nom, "nom du poste")?;
        let id = if p.id.is_empty() { op.nouvel_id() } else { p.id.clone() };
        op.execute(
            "INSERT INTO postes_preparation(id, nom, imprimante, ecran, actif, modifie_le) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(id) DO UPDATE SET nom = excluded.nom, imprimante = excluded.imprimante,
               ecran = excluded.ecran, actif = excluded.actif, modifie_le = excluded.modifie_le",
            params![id, p.nom.trim(), p.imprimante.trim(), p.ecran, p.actif, op.maintenant],
        )?;
        op.audit("poste.enregistrer", "poste", Some(&id), None, Some(json!(p)), None, None)?;
        Ok(id)
    })
}

// ───────────── Catégories ─────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Categorie {
    #[serde(default)]
    pub id: String,
    pub nom: String,
    #[serde(default = "couleur_defaut")]
    pub couleur: String,
    #[serde(default)]
    pub icone: String,
    #[serde(default)]
    pub ordre: i64,
    #[serde(default = "vrai")]
    pub actif: bool,
}

fn couleur_defaut() -> String {
    "#2e7d32".into()
}

pub fn lister_categories(conn: &Connection) -> Resultat<Vec<Categorie>> {
    let mut s = conn.prepare("SELECT id, nom, couleur, icone, ordre, actif FROM categories ORDER BY ordre, nom")?;
    let v = s
        .query_map([], |r| {
            Ok(Categorie {
                id: r.get(0)?,
                nom: r.get(1)?,
                couleur: r.get(2)?,
                icone: r.get(3)?,
                ordre: r.get(4)?,
                actif: r.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(v)
}

pub fn enregistrer_categorie(db: &mut Db, acteur: &Acteur, c: &Categorie) -> Resultat<String> {
    db.executer(acteur, |op| enregistrer_categorie_op(op, c))
}

pub(crate) fn enregistrer_categorie_op(op: &Op, c: &Categorie) -> Resultat<String> {
    op.exiger(perm::CATALOGUE_GERER)?;
    non_vide(&c.nom, "nom de la catégorie")?;
    let id = if c.id.is_empty() { op.nouvel_id() } else { c.id.clone() };
    op.execute(
        "INSERT INTO categories(id, nom, couleur, icone, ordre, actif, modifie_le) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(id) DO UPDATE SET nom = excluded.nom, couleur = excluded.couleur, icone = excluded.icone,
           ordre = excluded.ordre, actif = excluded.actif, modifie_le = excluded.modifie_le",
        params![id, c.nom.trim(), c.couleur, c.icone, c.ordre, c.actif, op.maintenant],
    )?;
    op.outbox("categorie", &id, "enregistrer")?;
    Ok(id)
}

// ───────────── Produits ─────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Produit {
    #[serde(default)]
    pub id: String,
    pub categorie_id: String,
    pub nom: String,
    #[serde(default)]
    pub nom_court: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub photo: String,
    pub prix: i64,
    #[serde(default)]
    pub poste_id: Option<String>,
    #[serde(default = "vrai")]
    pub disponible: bool,
    #[serde(default = "vrai")]
    pub actif: bool,
    #[serde(default)]
    pub code: String,
    #[serde(default)]
    pub code_barres: String,
    #[serde(default)]
    pub taux_tva_bp: i64,
    #[serde(default = "suivi_defaut")]
    pub suivi_stock: String,
    #[serde(default)]
    pub article_stock_id: Option<String>,
    #[serde(default)]
    pub prix_achat_estime: i64,
    #[serde(default)]
    pub ordre: i64,
    #[serde(default)]
    pub prix_zones: Vec<PrixZone>,
    #[serde(default)]
    pub groupes_options: Vec<GroupeOptions>,
}

fn suivi_defaut() -> String {
    "aucun".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrixZone {
    pub zone_id: String,
    pub prix: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupeOptions {
    #[serde(default)]
    pub id: String,
    pub nom: String,
    #[serde(default)]
    pub min_choix: i64,
    #[serde(default = "un")]
    pub max_choix: i64,
    #[serde(default)]
    pub options: Vec<OptionProduit>,
}

fn un() -> i64 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptionProduit {
    #[serde(default)]
    pub id: String,
    pub nom: String,
    #[serde(default)]
    pub supplement: i64,
}

const COLS_PRODUIT: &str = "id, categorie_id, nom, nom_court, description, photo, prix, poste_id, disponible, actif,
    code, code_barres, taux_tva_bp, suivi_stock, article_stock_id, prix_achat_estime, ordre";

fn produit_depuis(r: &rusqlite::Row) -> rusqlite::Result<Produit> {
    Ok(Produit {
        id: r.get(0)?,
        categorie_id: r.get(1)?,
        nom: r.get(2)?,
        nom_court: r.get(3)?,
        description: r.get(4)?,
        photo: r.get(5)?,
        prix: r.get(6)?,
        poste_id: r.get(7)?,
        disponible: r.get(8)?,
        actif: r.get(9)?,
        code: r.get(10)?,
        code_barres: r.get(11)?,
        taux_tva_bp: r.get(12)?,
        suivi_stock: r.get(13)?,
        article_stock_id: r.get(14)?,
        prix_achat_estime: r.get(15)?,
        ordre: r.get(16)?,
        prix_zones: vec![],
        groupes_options: vec![],
    })
}

fn completer(conn: &Connection, p: &mut Produit) -> Resultat<()> {
    let mut s = conn.prepare_cached("SELECT zone_id, prix FROM prix_zone WHERE produit_id = ?1")?;
    p.prix_zones = s
        .query_map(params![p.id], |r| Ok(PrixZone { zone_id: r.get(0)?, prix: r.get(1)? }))?
        .collect::<Result<Vec<_>, _>>()?;
    let mut s = conn.prepare_cached(
        "SELECT id, nom, min_choix, max_choix FROM groupes_options WHERE produit_id = ?1 ORDER BY ordre",
    )?;
    let mut groupes = s
        .query_map(params![p.id], |r| {
            Ok(GroupeOptions { id: r.get(0)?, nom: r.get(1)?, min_choix: r.get(2)?, max_choix: r.get(3)?, options: vec![] })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    for g in &mut groupes {
        let mut s = conn.prepare_cached("SELECT id, nom, supplement FROM options WHERE groupe_id = ?1 AND actif = 1")?;
        g.options = s
            .query_map(params![g.id], |r| Ok(OptionProduit { id: r.get(0)?, nom: r.get(1)?, supplement: r.get(2)? }))?
            .collect::<Result<Vec<_>, _>>()?;
    }
    p.groupes_options = groupes;
    Ok(())
}

pub fn lister_produits(conn: &Connection, actifs_seulement: bool) -> Resultat<Vec<Produit>> {
    let mut s = conn.prepare(&format!(
        "SELECT {COLS_PRODUIT} FROM produits WHERE (?1 = 0 OR actif = 1) ORDER BY ordre, nom"
    ))?;
    let mut v = s.query_map(params![actifs_seulement], produit_depuis)?.collect::<Result<Vec<_>, _>>()?;
    for p in &mut v {
        completer(conn, p)?;
    }
    Ok(v)
}

pub fn produit(conn: &Connection, id: &str) -> Resultat<Produit> {
    let mut p = trouver(
        conn.query_row(&format!("SELECT {COLS_PRODUIT} FROM produits WHERE id = ?1"), params![id], produit_depuis),
        "Produit",
    )?;
    completer(conn, &mut p)?;
    Ok(p)
}

/// RG-CAT-03.
pub fn prix_effectif(conn: &Connection, produit_id: &str, zone_id: Option<&str>) -> Resultat<i64> {
    if let Some(z) = zone_id {
        let p: Option<i64> = conn
            .query_row(
                "SELECT prix FROM prix_zone WHERE produit_id = ?1 AND zone_id = ?2",
                params![produit_id, z],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(p) = p {
            return Ok(p);
        }
    }
    trouver(conn.query_row("SELECT prix FROM produits WHERE id = ?1", params![produit_id], |r| r.get(0)), "Produit")
}

pub fn enregistrer_produit(db: &mut Db, acteur: &Acteur, p: &Produit) -> Resultat<String> {
    db.executer(acteur, |op| enregistrer_produit_op(op, p))
}

pub(crate) fn enregistrer_produit_op(op: &Op, p: &Produit) -> Resultat<String> {
    op.exiger(perm::CATALOGUE_GERER)?;
    non_vide(&p.nom, "nom du produit")?;
    if p.prix < 0 {
        return Err(Erreur::validation("Le prix ne peut pas être négatif"));
    }
    if !["aucun", "revendu", "recette"].contains(&p.suivi_stock.as_str()) {
        return Err(Erreur::validation("Mode de suivi du stock inconnu"));
    }
    if p.suivi_stock == "revendu" && p.article_stock_id.is_none() {
        return Err(Erreur::regle("RG-CAT-06", "Un produit revendu doit être lié à un article de stock"));
    }
    let nouveau = p.id.is_empty();
    let id = if nouveau { op.nouvel_id() } else { p.id.clone() };
    let ancien_prix: Option<i64> =
        op.query_row("SELECT prix FROM produits WHERE id = ?1", params![id], |r| r.get(0)).optional()?;
    op.execute(
        "INSERT INTO produits(id, categorie_id, nom, nom_court, description, photo, prix, poste_id, disponible, actif,
            code, code_barres, taux_tva_bp, suivi_stock, article_stock_id, prix_achat_estime, ordre, modifie_le)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)
         ON CONFLICT(id) DO UPDATE SET categorie_id = excluded.categorie_id, nom = excluded.nom,
            nom_court = excluded.nom_court, description = excluded.description, photo = excluded.photo,
            prix = excluded.prix, poste_id = excluded.poste_id, disponible = excluded.disponible,
            actif = excluded.actif, code = excluded.code, code_barres = excluded.code_barres,
            taux_tva_bp = excluded.taux_tva_bp, suivi_stock = excluded.suivi_stock,
            article_stock_id = excluded.article_stock_id, prix_achat_estime = excluded.prix_achat_estime,
            ordre = excluded.ordre, modifie_le = excluded.modifie_le, version = version + 1",
        params![
            id,
            p.categorie_id,
            p.nom.trim(),
            p.nom_court.trim(),
            p.description,
            p.photo,
            p.prix,
            p.poste_id,
            p.disponible,
            p.actif,
            p.code,
            p.code_barres,
            p.taux_tva_bp,
            p.suivi_stock,
            p.article_stock_id,
            p.prix_achat_estime,
            p.ordre,
            op.maintenant
        ],
    )?;
    // RG-CAT-02 : historique des prix.
    if ancien_prix != Some(p.prix) {
        op.execute(
            "INSERT INTO historique_prix(id, produit_id, zone_id, ancien, nouveau, horodatage, utilisateur_id)
             VALUES (?1, ?2, NULL, ?3, ?4, ?5, ?6)",
            params![op.nouvel_id(), id, ancien_prix, p.prix, op.maintenant, op.utilisateur()],
        )?;
        if !nouveau {
            op.audit(
                "produit.prix",
                "produit",
                Some(&id),
                Some(json!({ "prix": ancien_prix })),
                Some(json!({ "prix": p.prix })),
                None,
                None,
            )?;
        }
    }
    // Prix par zone.
    let anciens: Vec<(String, i64)> = op
        .prepare("SELECT zone_id, prix FROM prix_zone WHERE produit_id = ?1")?
        .query_map(params![id], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<_, _>>()?;
    op.execute("DELETE FROM prix_zone WHERE produit_id = ?1", params![id])?;
    for pz in &p.prix_zones {
        if pz.prix < 0 {
            return Err(Erreur::validation("Prix de zone négatif"));
        }
        op.execute(
            "INSERT INTO prix_zone(produit_id, zone_id, prix) VALUES (?1, ?2, ?3)",
            params![id, pz.zone_id, pz.prix],
        )?;
        let avant = anciens.iter().find(|(z, _)| z == &pz.zone_id).map(|(_, p)| *p);
        if avant != Some(pz.prix) {
            op.execute(
                "INSERT INTO historique_prix(id, produit_id, zone_id, ancien, nouveau, horodatage, utilisateur_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![op.nouvel_id(), id, pz.zone_id, avant, pz.prix, op.maintenant, op.utilisateur()],
            )?;
        }
    }
    // Options : remplacées (les lignes de commande en gardent une copie JSON).
    let groupes: Vec<String> = op
        .prepare("SELECT id FROM groupes_options WHERE produit_id = ?1")?
        .query_map(params![id], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    // Recettes des options (fiche 0014) : mises de côté, remises si l'option existe toujours.
    let recettes_options: Vec<(String, String, String, i64, i64)> = op
        .prepare(
            "SELECT r.id, r.option_id, r.article_id, r.quantite, r.modifie_le FROM recettes r
             JOIN options o ON o.id = r.option_id JOIN groupes_options g ON g.id = o.groupe_id WHERE g.produit_id = ?1",
        )?
        .query_map(params![id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)))?
        .collect::<Result<_, _>>()?;
    for r in &recettes_options {
        op.execute("DELETE FROM recettes WHERE id = ?1", params![r.0])?;
    }
    for g in &groupes {
        op.execute("DELETE FROM options WHERE groupe_id = ?1", params![g])?;
    }
    op.execute("DELETE FROM groupes_options WHERE produit_id = ?1", params![id])?;
    for (i, g) in p.groupes_options.iter().enumerate() {
        if g.min_choix < 0 || g.max_choix < g.min_choix.max(1) {
            return Err(Erreur::validation(format!("Groupe « {} » : min/max incohérents", g.nom)));
        }
        let gid = if g.id.is_empty() { op.nouvel_id() } else { g.id.clone() };
        op.execute(
            "INSERT INTO groupes_options(id, produit_id, nom, min_choix, max_choix, ordre) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![gid, id, g.nom, g.min_choix, g.max_choix, i as i64],
        )?;
        for o in &g.options {
            let oid = if o.id.is_empty() { op.nouvel_id() } else { o.id.clone() };
            op.execute(
                "INSERT INTO options(id, groupe_id, nom, supplement) VALUES (?1, ?2, ?3, ?4)",
                params![oid, gid, o.nom, o.supplement],
            )?;
        }
    }
    for (rid, option, article, quantite, modifie) in recettes_options {
        op.execute(
            "INSERT INTO recettes(id, option_id, article_id, quantite, modifie_le)
             SELECT ?1, ?2, ?3, ?4, ?5 WHERE EXISTS (SELECT 1 FROM options WHERE id = ?2)",
            params![rid, option, article, quantite, modifie],
        )?;
    }
    if nouveau {
        op.audit("produit.creer", "produit", Some(&id), None, Some(json!({ "nom": p.nom, "prix": p.prix })), None, None)?;
    }
    op.outbox("produit", &id, "enregistrer")?;
    Ok(id)
}

/// Rupture du jour en un clic (RG-CAT-05).
pub fn definir_disponibilite(db: &mut Db, acteur: &Acteur, produit_id: &str, disponible: bool) -> Resultat<()> {
    db.executer(acteur, |op| {
        // Le caissier doit pouvoir signaler une rupture sans droits de catalogue.
        if !op.a_permission(perm::CAISSE_ENCAISSER) {
            op.exiger(perm::CATALOGUE_GERER)?;
        }
        let n = op.execute(
            "UPDATE produits SET disponible = ?1, modifie_le = ?2 WHERE id = ?3",
            params![disponible, op.maintenant, produit_id],
        )?;
        if n == 0 {
            return Err(Erreur::NonTrouve("Produit".into()));
        }
        op.audit("produit.disponibilite", "produit", Some(produit_id), None, Some(json!({ "disponible": disponible })), None, None)?;
        op.evenement("catalogue", Some(produit_id));
        Ok(())
    })
}

#[derive(Debug, Serialize)]
pub struct LigneHistoriquePrix {
    pub horodatage: i64,
    pub zone_id: Option<String>,
    pub ancien: Option<i64>,
    pub nouveau: i64,
}

pub fn historique_prix(conn: &Connection, produit_id: &str) -> Resultat<Vec<LigneHistoriquePrix>> {
    let mut s = conn.prepare(
        "SELECT horodatage, zone_id, ancien, nouveau FROM historique_prix WHERE produit_id = ?1 ORDER BY horodatage DESC",
    )?;
    let v = s
        .query_map(params![produit_id], |r| {
            Ok(LigneHistoriquePrix { horodatage: r.get(0)?, zone_id: r.get(1)?, ancien: r.get(2)?, nouveau: r.get(3)? })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(v)
}

/// Import depuis un tableur enregistré en CSV (séparateur `;` ou `,`) :
/// `categorie;nom;prix[;poste]`. Les catégories et postes manquants sont créés.
pub fn importer_csv(db: &mut Db, acteur: &Acteur, texte: &str) -> Resultat<usize> {
    db.executer(acteur, |op| {
        op.exiger(perm::CATALOGUE_GERER)?;
        let mut n = 0;
        for (i, ligne) in texte.lines().enumerate() {
            let ligne = ligne.trim().trim_start_matches('\u{feff}');
            if ligne.is_empty() {
                continue;
            }
            let sep = if ligne.contains(';') { ';' } else { ',' };
            let cols: Vec<&str> = ligne.split(sep).map(|c| c.trim().trim_matches('"')).collect();
            if cols.len() < 3 {
                return Err(Erreur::validation(format!("Ligne {} : il faut au moins catégorie, nom, prix", i + 1)));
            }
            let prix_txt: String = cols[2].chars().filter(|c| c.is_ascii_digit()).collect();
            let Ok(prix) = prix_txt.parse::<i64>() else {
                if i == 0 {
                    continue; // ligne d'en-tête
                }
                return Err(Erreur::validation(format!("Ligne {} : prix invalide « {} »", i + 1, cols[2])));
            };
            let cat_id = match op
                .query_row("SELECT id FROM categories WHERE nom = ?1", params![cols[0]], |r| r.get::<_, String>(0))
                .optional()?
            {
                Some(id) => id,
                None => enregistrer_categorie_op(
                    op,
                    &Categorie {
                        id: String::new(),
                        nom: cols[0].into(),
                        couleur: couleur_defaut(),
                        icone: String::new(),
                        ordre: 0,
                        actif: true,
                    },
                )?,
            };
            let poste_id = match cols.get(3).filter(|s| !s.is_empty()) {
                None => None,
                Some(nom) => Some(
                    match op
                        .query_row("SELECT id FROM postes_preparation WHERE nom = ?1", params![nom], |r| {
                            r.get::<_, String>(0)
                        })
                        .optional()?
                    {
                        Some(id) => id,
                        None => {
                            let id = op.nouvel_id();
                            op.execute(
                                "INSERT INTO postes_preparation(id, nom, modifie_le) VALUES (?1, ?2, ?3)",
                                params![id, nom, op.maintenant],
                            )?;
                            id
                        }
                    },
                ),
            };
            enregistrer_produit_op(
                op,
                &Produit {
                    id: String::new(),
                    categorie_id: cat_id,
                    nom: cols[1].into(),
                    nom_court: String::new(),
                    description: String::new(),
                    photo: String::new(),
                    prix,
                    poste_id,
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
                },
            )?;
            n += 1;
        }
        op.audit("produit.import", "produit", None, None, Some(json!({ "nombre": n })), None, None)?;
        Ok(n)
    })
}

pub(crate) fn non_vide(s: &str, quoi: &str) -> Resultat<()> {
    if s.trim().is_empty() {
        return Err(Erreur::validation(format!("Le champ « {quoi} » est obligatoire")));
    }
    Ok(())
}

/// Quantité disponible par produit suivi en stock (affichage « 12 disponibles » à la prise de commande) :
/// article revendu → stock de l'article ; recette → portions possibles avec le stock des ingrédients
/// (le plus petit stock ÷ quantité par portion). Produits sans suivi : absents. Jamais bloquant (RG-STK-07).
pub fn disponibles(conn: &Connection) -> Resultat<std::collections::HashMap<String, i64>> {
    let mut v = std::collections::HashMap::new();
    let mut s = conn.prepare(
        "SELECT p.id, COALESCE(SUM(m.quantite), 0) FROM produits p JOIN mouvements_stock m ON m.article_id = p.article_stock_id
         WHERE p.actif = 1 AND p.suivi_stock = 'revendu' GROUP BY p.id",
    )?;
    for r in s.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))? {
        let (id, q) = r?;
        v.insert(id, q.max(0));
    }
    let mut s = conn.prepare(
        "SELECT r.produit_id, MIN(COALESCE((SELECT SUM(m.quantite) FROM mouvements_stock m WHERE m.article_id = r.article_id), 0) / r.quantite)
         FROM recettes r JOIN produits p ON p.id = r.produit_id WHERE p.actif = 1 AND p.suivi_stock = 'recette' GROUP BY r.produit_id",
    )?;
    for r in s.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))? {
        let (id, q) = r?;
        v.insert(id, q.max(0));
    }
    Ok(v)
}
