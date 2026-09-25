//! Recettes et consommation théorique (fiche 0014, RG-REC-01 à 04).
//! Facultatives : un plat sans recette se vend normalement, sans sortie de stock.

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::db::{trouver, Acteur, Db};
use crate::erreur::{Erreur, Resultat};
use crate::permissions as perm;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LigneRecette {
    pub article_id: String,
    /// Entier, dans l'unité de base de l'article (g, ml, pièce).
    pub quantite: i64,
    #[serde(default)]
    pub article_nom: String,
    #[serde(default)]
    pub unite: String,
    #[serde(default)]
    pub cout_unitaire: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecetteOption {
    pub option_id: String,
    #[serde(default)]
    pub option_nom: String,
    pub lignes: Vec<LigneRecette>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Recette {
    pub produit_id: String,
    pub lignes: Vec<LigneRecette>,
    #[serde(default)]
    pub options: Vec<RecetteOption>,
    /// RG-REC-04 : coût matière du plat de base, Σ quantité × coût unitaire.
    #[serde(default)]
    pub cout: i64,
}

fn lignes(conn: &Connection, colonne: &str, id: &str) -> Resultat<Vec<LigneRecette>> {
    let mut s = conn.prepare(&format!(
        "SELECT r.article_id, r.quantite, a.nom, a.unite, a.cout_unitaire FROM recettes r JOIN articles_stock a ON a.id = r.article_id
         WHERE r.{colonne} = ?1 ORDER BY a.nom"
    ))?;
    let v = s
        .query_map(params![id], |r| {
            Ok(LigneRecette { article_id: r.get(0)?, quantite: r.get(1)?, article_nom: r.get(2)?, unite: r.get(3)?, cout_unitaire: r.get(4)? })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(v)
}

fn cout(l: &[LigneRecette]) -> i64 {
    l.iter().map(|x| x.quantite * x.cout_unitaire).sum()
}

pub fn lire(conn: &Connection, produit_id: &str) -> Resultat<Recette> {
    let base = lignes(conn, "produit_id", produit_id)?;
    let mut s = conn.prepare(
        "SELECT o.id, o.nom FROM options o JOIN groupes_options g ON g.id = o.groupe_id WHERE g.produit_id = ?1 ORDER BY g.ordre, o.nom",
    )?;
    let opts: Vec<(String, String)> = s.query_map(params![produit_id], |r| Ok((r.get(0)?, r.get(1)?)))?.collect::<Result<_, _>>()?;
    let mut options = Vec::new();
    for (id, nom) in opts {
        let l = lignes(conn, "option_id", &id)?;
        if !l.is_empty() {
            options.push(RecetteOption { option_id: id, option_nom: nom, lignes: l });
        }
    }
    Ok(Recette { produit_id: produit_id.into(), cout: cout(&base), lignes: base, options })
}

/// RG-REC-01 : la recette remplace la précédente ; quantités entières > 0, articles actifs, pas de doublon.
/// Un produit avec recette passe en suivi « recette » ; sans recette, il repasse en « aucun ».
pub fn definir(db: &mut Db, acteur: &Acteur, r: &Recette) -> Resultat<()> {
    db.executer(acteur, |op| {
        op.exiger(perm::CATALOGUE_GERER)?;
        let suivi: String = trouver(op.query_row("SELECT suivi_stock FROM produits WHERE id = ?1", params![r.produit_id], |x| x.get(0)), "Produit")?;
        let verifier = |l: &[LigneRecette]| -> Resultat<()> {
            let mut vus = std::collections::HashSet::new();
            for x in l {
                if x.quantite <= 0 {
                    return Err(Erreur::regle("RG-REC-01", "Quantité de recette : nombre entier positif (dans l'unité de base)"));
                }
                if !vus.insert(&x.article_id) {
                    return Err(Erreur::regle("RG-REC-01", "Un ingrédient apparaît deux fois"));
                }
                let actif: bool = trouver(op.query_row("SELECT actif FROM articles_stock WHERE id = ?1", params![x.article_id], |r| r.get(0)), "Article de stock")?;
                if !actif {
                    return Err(Erreur::regle("RG-REC-01", "Ingrédient inactif"));
                }
            }
            Ok(())
        };
        verifier(&r.lignes)?;
        for o in &r.options {
            verifier(&o.lignes)?;
            let du_produit: i64 = op.query_row(
                "SELECT COUNT(*) FROM options o JOIN groupes_options g ON g.id = o.groupe_id WHERE o.id = ?1 AND g.produit_id = ?2",
                params![o.option_id, r.produit_id],
                |x| x.get(0),
            )?;
            if du_produit == 0 {
                return Err(Erreur::validation("Option d'un autre produit"));
            }
        }
        let avant = serde_json::to_value(lire(op, &r.produit_id)?)?;
        op.execute("DELETE FROM recettes WHERE produit_id = ?1", params![r.produit_id])?;
        op.execute(
            "DELETE FROM recettes WHERE option_id IN (SELECT o.id FROM options o JOIN groupes_options g ON g.id = o.groupe_id WHERE g.produit_id = ?1)",
            params![r.produit_id],
        )?;
        let inserer = |colonne: &str, id: &str, l: &LigneRecette| -> Resultat<()> {
            op.execute(
                &format!("INSERT INTO recettes(id, {colonne}, article_id, quantite, modifie_le) VALUES (?1, ?2, ?3, ?4, ?5)"),
                params![op.nouvel_id(), id, l.article_id, l.quantite, op.maintenant],
            )?;
            Ok(())
        };
        for l in &r.lignes {
            inserer("produit_id", &r.produit_id, l)?;
        }
        for o in &r.options {
            for l in &o.lignes {
                inserer("option_id", &o.option_id, l)?;
            }
        }
        let vide = r.lignes.is_empty() && r.options.iter().all(|o| o.lignes.is_empty());
        let nouveau = match (vide, suivi.as_str()) {
            (false, _) => "recette",
            (true, "recette") => "aucun",
            (true, s) => s,
        };
        op.execute(
            "UPDATE produits SET suivi_stock = ?1, modifie_le = ?2, version = version + 1 WHERE id = ?3",
            params![nouveau, op.maintenant, r.produit_id],
        )?;
        let apres = serde_json::to_value(lire(op, &r.produit_id)?)?;
        op.audit("recette.definir", "produit", Some(&r.produit_id), Some(avant), Some(apres), None, None)?;
        op.outbox("produit", &r.produit_id, "recette")?;
        op.evenement("catalogue", Some(&r.produit_id));
        Ok(())
    })
}

/// RG-REC-02 : consommation théorique d'une unité vendue (plat + options choisies), par article.
pub(crate) fn consommation_unitaire(conn: &Connection, produit_id: &str, options_json: &str) -> Resultat<Vec<(String, i64, i64)>> {
    let mut total: Vec<(String, i64, i64)> = Vec::new();
    let mut ajouter = |l: Vec<LigneRecette>| {
        for x in l {
            match total.iter_mut().find(|t| t.0 == x.article_id) {
                Some(t) => t.1 += x.quantite,
                None => total.push((x.article_id, x.quantite, x.cout_unitaire)),
            }
        }
    };
    ajouter(lignes(conn, "produit_id", produit_id)?);
    let options: Vec<serde_json::Value> = serde_json::from_str(options_json).unwrap_or_default();
    for o in options {
        if let Some(id) = o["id"].as_str() {
            ajouter(lignes(conn, "option_id", id)?);
        }
    }
    Ok(total)
}

#[derive(Debug, Serialize)]
pub struct CoutMatiere {
    pub produit_id: String,
    pub nom: String,
    pub prix: i64,
    pub cout: i64,
    /// Coût matière en points de base du prix (3 000 = 30 %).
    pub part_bp: i64,
}

/// RG-REC-04 : coût matière de chaque plat avec recette ; formule « Σ quantité × coût unitaire de l'article ».
pub fn couts_matiere(conn: &Connection) -> Resultat<Vec<CoutMatiere>> {
    let produits: Vec<(String, String, i64)> = conn
        .prepare("SELECT id, nom, prix FROM produits WHERE suivi_stock = 'recette' AND actif = 1 ORDER BY nom")?
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<Result<_, _>>()?;
    let mut v = Vec::new();
    for (id, nom, prix) in produits {
        let c = cout(&lignes(conn, "produit_id", &id)?);
        v.push(CoutMatiere { produit_id: id, nom, prix, cout: c, part_bp: if prix > 0 { c * 10_000 / prix } else { 0 } });
    }
    Ok(v)
}
