use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::catalogue::non_vide;
use crate::db::{trouver, Acteur, Db, Op};
use crate::erreur::{Erreur, Resultat};
use crate::permissions as perm;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Article {
    #[serde(default)]
    pub id: String,
    pub nom: String,
    #[serde(default = "unite_defaut")]
    pub unite: String,
    #[serde(default)]
    pub seuil_alerte: i64,
    #[serde(default)]
    pub cout_unitaire: i64,
    #[serde(default)]
    pub famille: String,
    #[serde(default = "vrai")]
    pub actif: bool,
    #[serde(default)]
    pub conditionnements: Vec<Conditionnement>,
}

fn unite_defaut() -> String {
    "bouteille".into()
}
fn vrai() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conditionnement {
    #[serde(default)]
    pub id: String,
    pub nom: String,
    pub contenance: i64,
}

#[derive(Debug, Serialize)]
pub struct NiveauStock {
    pub article_id: String,
    pub nom: String,
    pub unite: String,
    pub famille: String,
    pub quantite: i64,
    pub seuil_alerte: i64,
    pub cout_unitaire: i64,
    pub valeur: i64,
    pub alerte: bool,
    pub conditionnements: Vec<Conditionnement>,
}

pub fn enregistrer_article(db: &mut Db, acteur: &Acteur, a: &Article) -> Resultat<String> {
    db.executer(acteur, |op| enregistrer_article_op(op, a))
}

pub(crate) fn enregistrer_article_op(op: &Op, a: &Article) -> Resultat<String> {
    op.exiger(perm::STOCK_MOUVEMENT)?;
    non_vide(&a.nom, "nom de l'article")?;
    let id = if a.id.is_empty() { op.nouvel_id() } else { a.id.clone() };
    op.execute(
        "INSERT INTO articles_stock(id, nom, unite, seuil_alerte, cout_unitaire, famille, actif, modifie_le)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
         ON CONFLICT(id) DO UPDATE SET nom = excluded.nom, unite = excluded.unite, seuil_alerte = excluded.seuil_alerte,
           famille = excluded.famille, actif = excluded.actif, modifie_le = excluded.modifie_le",
        params![id, a.nom.trim(), a.unite, a.seuil_alerte, a.cout_unitaire, a.famille, a.actif, op.maintenant],
    )?;
    op.execute("UPDATE conditionnements SET actif = 0 WHERE article_id = ?1", params![id])?;
    for c in &a.conditionnements {
        if c.contenance <= 0 {
            return Err(Erreur::validation("La contenance d'un conditionnement doit être positive"));
        }
        let cid = if c.id.is_empty() { op.nouvel_id() } else { c.id.clone() };
        op.execute(
            "INSERT INTO conditionnements(id, article_id, nom, contenance, actif) VALUES (?1, ?2, ?3, ?4, 1)
             ON CONFLICT(id) DO UPDATE SET nom = excluded.nom, contenance = excluded.contenance, actif = 1",
            params![cid, id, c.nom, c.contenance],
        )?;
    }
    op.outbox("article_stock", &id, "enregistrer")?;
    Ok(id)
}

pub fn conditionnements(conn: &Connection, article_id: &str) -> Resultat<Vec<Conditionnement>> {
    let mut s = conn.prepare_cached(
        "SELECT id, nom, contenance FROM conditionnements WHERE article_id = ?1 AND actif = 1 ORDER BY contenance",
    )?;
    let v = s
        .query_map(params![article_id], |r| Ok(Conditionnement { id: r.get(0)?, nom: r.get(1)?, contenance: r.get(2)? }))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(v)
}

/// Solde = Σ mouvements (cahier §4).
pub fn quantite(conn: &Connection, article_id: &str) -> Resultat<i64> {
    Ok(conn.query_row(
        "SELECT COALESCE(SUM(quantite), 0) FROM mouvements_stock WHERE article_id = ?1",
        params![article_id],
        |r| r.get(0),
    )?)
}

pub fn niveaux(conn: &Connection) -> Resultat<Vec<NiveauStock>> {
    let mut s = conn.prepare(
        "SELECT a.id, a.nom, a.unite, a.famille, COALESCE(SUM(m.quantite), 0), a.seuil_alerte, a.cout_unitaire
         FROM articles_stock a LEFT JOIN mouvements_stock m ON m.article_id = a.id
         WHERE a.actif = 1 GROUP BY a.id ORDER BY a.famille, a.nom",
    )?;
    let base = s
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, i64>(4)?,
                r.get::<_, i64>(5)?,
                r.get::<_, i64>(6)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let mut v = Vec::new();
    for (id, nom, unite, famille, q, seuil, cout) in base {
        v.push(NiveauStock {
            conditionnements: conditionnements(conn, &id)?,
            article_id: id,
            nom,
            unite,
            famille,
            quantite: q,
            seuil_alerte: seuil,
            cout_unitaire: cout,
            valeur: q.max(0) * cout,
            // RG-STK-07 : stock négatif = alerte, jamais blocage.
            alerte: q <= seuil,
        });
    }
    Ok(v)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn inserer_mouvement(
    op: &Op,
    article_id: &str,
    type_: &str,
    quantite: i64,
    cout_unitaire: i64,
    motif: &str,
    reference: Option<(&str, &str)>,
) -> Resultat<String> {
    let journee = crate::journee::ouverte(op)?.map(|j| j.id);
    let id = op.nouvel_id();
    op.execute(
        "INSERT INTO mouvements_stock(id, article_id, type, quantite, cout_unitaire, motif, reference_type, reference_id,
            journee_id, horodatage, utilisateur_id)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            id,
            article_id,
            type_,
            quantite,
            cout_unitaire,
            motif,
            reference.map(|r| r.0),
            reference.map(|r| r.1),
            journee,
            op.maintenant,
            op.utilisateur()
        ],
    )?;
    op.outbox("mouvement_stock", &id, "creer")?;
    Ok(id)
}

/// RG-STK-01 : sortie d'un article revendu à l'envoi (ou au paiement en comptoir).
pub(crate) fn sortie_vente(op: &Op, ligne_id: &str) -> Resultat<()> {
    let r: Option<(i64, i64, bool, bool, String, Option<String>, Option<String>)> = op
        .query_row(
            "SELECT l.quantite, l.quantite_annulee, l.offert, l.stock_sorti, p.suivi_stock, p.article_stock_id, c.employe_id
             FROM lignes_commande l JOIN produits p ON p.id = l.produit_id JOIN commandes c ON c.id = l.commande_id
             WHERE l.id = ?1",
            params![ligne_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?)),
        )
        .optional()?;
    let Some((q, qa, offert, deja, suivi, article, employe)) = r else { return Ok(()) };
    if deja || suivi != "revendu" {
        return Ok(());
    }
    let Some(article) = article else { return Ok(()) };
    let cout: i64 = op.query_row("SELECT cout_unitaire FROM articles_stock WHERE id = ?1", params![article], |r| r.get(0))?;
    let type_ = if employe.is_some() {
        "consommation_employe"
    } else if offert {
        "offert"
    } else {
        "vente"
    };
    inserer_mouvement(op, &article, type_, -(q - qa), cout, "", Some(("ligne_commande", ligne_id)))?;
    op.execute(
        "UPDATE lignes_commande SET stock_sorti = 1, cout_unitaire = ?1 WHERE id = ?2",
        params![cout, ligne_id],
    )?;
    Ok(())
}

/// RG-STK-02 : retour en stock d'un article annulé, sauf perte.
pub(crate) fn retour_annulation(op: &Op, ligne_id: &str, quantite: i64, perte: bool) -> Resultat<()> {
    if perte {
        return Ok(());
    }
    let r: Option<(bool, Option<String>, i64)> = op
        .query_row(
            "SELECT l.stock_sorti, p.article_stock_id, l.cout_unitaire FROM lignes_commande l JOIN produits p ON p.id = l.produit_id
             WHERE l.id = ?1 AND p.suivi_stock = 'revendu'",
            params![ligne_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    if let Some((true, Some(article), cout)) = r {
        inserer_mouvement(op, &article, "annulation_vente", quantite, cout, "", Some(("ligne_commande", ligne_id)))?;
    }
    Ok(())
}

pub const TYPES_MANUELS: &[&str] = &[
    "perte", "perime", "casse", "repas_personnel", "offert", "consommation_interne", "vol", "regularisation",
    "retour_fournisseur",
];

#[derive(Debug, Deserialize)]
pub struct MouvementManuel {
    pub article_id: String,
    #[serde(rename = "type")]
    pub type_: String,
    /// Quantité positive ; le signe dépend du type (régularisation : signée).
    pub quantite: i64,
    pub motif: String,
    /// Conditionnement optionnel (quantité exprimée en casiers…).
    #[serde(default)]
    pub conditionnement_id: Option<String>,
}

/// RG-STK-04.
pub fn mouvement_manuel(db: &mut Db, acteur: &Acteur, m: &MouvementManuel) -> Resultat<String> {
    db.executer(acteur, |op| {
        let autorise_par = op.exiger(perm::STOCK_MOUVEMENT)?;
        if !TYPES_MANUELS.contains(&m.type_.as_str()) {
            return Err(Erreur::regle("RG-STK-04", "Type de mouvement non autorisé"));
        }
        if m.motif.trim().is_empty() {
            return Err(Erreur::regle("RG-STK-04", "Motif obligatoire"));
        }
        if m.quantite == 0 || (m.type_ != "regularisation" && m.quantite < 0) {
            return Err(Erreur::validation("Quantité invalide"));
        }
        let facteur = contenance(op, m.conditionnement_id.as_deref())?;
        let q = m.quantite * facteur;
        let signe = if m.type_ == "regularisation" { q } else { -q };
        let cout: i64 = trouver(
            op.query_row("SELECT cout_unitaire FROM articles_stock WHERE id = ?1", params![m.article_id], |r| r.get(0)),
            "Article",
        )?;
        let id = inserer_mouvement(op, &m.article_id, &m.type_, signe, cout, m.motif.trim(), None)?;
        op.audit(
            "stock.mouvement",
            "article_stock",
            Some(&m.article_id),
            None,
            Some(json!({ "type": m.type_, "quantite": signe })),
            Some(m.motif.trim()),
            autorise_par.as_deref(),
        )?;
        op.evenement("stock", Some(&m.article_id));
        Ok(id)
    })
}

pub(crate) fn contenance(conn: &Connection, conditionnement_id: Option<&str>) -> Resultat<i64> {
    match conditionnement_id {
        None => Ok(1),
        Some(c) => trouver(
            conn.query_row("SELECT contenance FROM conditionnements WHERE id = ?1", params![c], |r| r.get(0)),
            "Conditionnement",
        ),
    }
}

// ───────────── Inventaire ─────────────

#[derive(Debug, Serialize)]
pub struct Inventaire {
    pub id: String,
    pub libelle: String,
    pub statut: String,
    pub cree_le: i64,
    pub valide_le: Option<i64>,
    pub lignes: Vec<LigneInventaire>,
}

#[derive(Debug, Serialize)]
pub struct LigneInventaire {
    pub article_id: String,
    pub nom: String,
    pub unite: String,
    pub compte: i64,
    /// Figé à la validation ; sinon stock théorique actuel.
    pub theorique: i64,
    pub ecart: i64,
    pub valeur_ecart: i64,
}

pub fn creer_inventaire(db: &mut Db, acteur: &Acteur, libelle: &str) -> Resultat<String> {
    db.executer(acteur, |op| {
        op.exiger(perm::STOCK_INVENTAIRE)?;
        let en_cours: i64 =
            op.query_row("SELECT COUNT(*) FROM inventaires WHERE statut = 'en_cours'", [], |r| r.get(0))?;
        if en_cours > 0 {
            return Err(Erreur::validation("Un inventaire est déjà en cours"));
        }
        let id = op.nouvel_id();
        let journee = crate::journee::ouverte(op)?.map(|j| j.id);
        let libelle = if libelle.trim().is_empty() { "Inventaire" } else { libelle.trim() };
        op.execute(
            "INSERT INTO inventaires(id, journee_id, libelle, statut, cree_le, cree_par) VALUES (?1, ?2, ?3, 'en_cours', ?4, ?5)",
            params![id, journee, libelle, op.maintenant, op.utilisateur()],
        )?;
        Ok(id)
    })
}

/// Saisie (ou correction) d'une quantité comptée. Inventaire partiel : seuls les articles saisis comptent.
pub fn saisir_comptage(db: &mut Db, acteur: &Acteur, inventaire_id: &str, article_id: &str, compte: i64) -> Resultat<()> {
    db.executer(acteur, |op| {
        op.exiger(perm::STOCK_INVENTAIRE)?;
        exiger_en_cours(op, inventaire_id)?;
        if compte < 0 {
            return Err(Erreur::validation("Quantité comptée négative"));
        }
        op.execute(
            "INSERT INTO lignes_inventaire(inventaire_id, article_id, compte) VALUES (?1, ?2, ?3)
             ON CONFLICT(inventaire_id, article_id) DO UPDATE SET compte = excluded.compte",
            params![inventaire_id, article_id, compte],
        )?;
        Ok(())
    })
}

fn exiger_en_cours(conn: &Connection, inventaire_id: &str) -> Resultat<()> {
    let statut: String = trouver(
        conn.query_row("SELECT statut FROM inventaires WHERE id = ?1", params![inventaire_id], |r| r.get(0)),
        "Inventaire",
    )?;
    if statut != "en_cours" {
        return Err(Erreur::validation("Cet inventaire est clos"));
    }
    Ok(())
}

/// RG-STK-05 : validation par un responsable, un mouvement d'écart par article.
pub fn valider_inventaire(db: &mut Db, acteur: &Acteur, inventaire_id: &str) -> Resultat<()> {
    db.executer(acteur, |op| {
        let autorise_par = op.exiger(perm::STOCK_VALIDER_INVENTAIRE)?;
        exiger_en_cours(op, inventaire_id)?;
        let lignes: Vec<(String, i64)> = op
            .prepare("SELECT article_id, compte FROM lignes_inventaire WHERE inventaire_id = ?1")?
            .query_map(params![inventaire_id], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<Result<_, _>>()?;
        if lignes.is_empty() {
            return Err(Erreur::validation("Aucune quantité saisie"));
        }
        let mut ecarts = Vec::new();
        for (article, compte) in lignes {
            let theorique = quantite(op, &article)?;
            op.execute(
                "UPDATE lignes_inventaire SET theorique = ?1 WHERE inventaire_id = ?2 AND article_id = ?3",
                params![theorique, inventaire_id, article],
            )?;
            let ecart = compte - theorique;
            if ecart != 0 {
                let cout: i64 =
                    op.query_row("SELECT cout_unitaire FROM articles_stock WHERE id = ?1", params![article], |r| r.get(0))?;
                inserer_mouvement(op, &article, "inventaire", ecart, cout, "Écart d'inventaire", Some(("inventaire", inventaire_id)))?;
                ecarts.push(json!({ "article": article, "theorique": theorique, "compte": compte, "ecart": ecart }));
            }
        }
        op.execute(
            "UPDATE inventaires SET statut = 'valide', valide_le = ?1, valide_par = ?2 WHERE id = ?3",
            params![op.maintenant, autorise_par.as_deref().or(op.utilisateur()), inventaire_id],
        )?;
        op.audit("stock.inventaire", "inventaire", Some(inventaire_id), None, Some(json!(ecarts)), None, autorise_par.as_deref())?;
        op.evenement("stock", None);
        Ok(())
    })
}

pub fn abandonner_inventaire(db: &mut Db, acteur: &Acteur, inventaire_id: &str) -> Resultat<()> {
    db.executer(acteur, |op| {
        op.exiger(perm::STOCK_INVENTAIRE)?;
        exiger_en_cours(op, inventaire_id)?;
        op.execute("UPDATE inventaires SET statut = 'abandonne' WHERE id = ?1", params![inventaire_id])?;
        Ok(())
    })
}

pub fn inventaire(conn: &Connection, id: &str) -> Resultat<Inventaire> {
    let (libelle, statut, cree_le, valide_le): (String, String, i64, Option<i64>) = trouver(
        conn.query_row("SELECT libelle, statut, cree_le, valide_le FROM inventaires WHERE id = ?1", params![id], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
        }),
        "Inventaire",
    )?;
    let mut s = conn.prepare(
        "SELECT l.article_id, a.nom, a.unite, l.compte, l.theorique, a.cout_unitaire
         FROM lignes_inventaire l JOIN articles_stock a ON a.id = l.article_id
         WHERE l.inventaire_id = ?1 ORDER BY a.nom",
    )?;
    let base = s
        .query_map(params![id], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, i64>(3)?,
                r.get::<_, Option<i64>>(4)?,
                r.get::<_, i64>(5)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let mut lignes = Vec::new();
    for (article_id, nom, unite, compte, theo, cout) in base {
        let theorique = match theo {
            Some(t) => t,
            None => quantite(conn, &article_id)?,
        };
        let ecart = compte - theorique;
        lignes.push(LigneInventaire { article_id, nom, unite, compte, theorique, ecart, valeur_ecart: ecart * cout });
    }
    Ok(Inventaire { id: id.into(), libelle, statut, cree_le, valide_le, lignes })
}

pub fn lister_inventaires(conn: &Connection) -> Resultat<Vec<(String, String, String, i64)>> {
    let mut s = conn.prepare("SELECT id, libelle, statut, cree_le FROM inventaires ORDER BY cree_le DESC LIMIT 50")?;
    let v = s.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))?.collect::<Result<Vec<_>, _>>()?;
    Ok(v)
}

#[derive(Debug, Serialize)]
pub struct MouvementLu {
    pub id: String,
    pub type_: String,
    pub quantite: i64,
    pub motif: String,
    pub horodatage: i64,
    pub utilisateur: Option<String>,
}

pub fn mouvements_article(conn: &Connection, article_id: &str, limite: i64) -> Resultat<Vec<MouvementLu>> {
    let mut s = conn.prepare(
        "SELECT m.id, m.type, m.quantite, m.motif, m.horodatage, u.nom FROM mouvements_stock m
         LEFT JOIN utilisateurs u ON u.id = m.utilisateur_id
         WHERE m.article_id = ?1 ORDER BY m.horodatage DESC, m.rowid DESC LIMIT ?2",
    )?;
    let v = s
        .query_map(params![article_id, limite], |r| {
            Ok(MouvementLu {
                id: r.get(0)?,
                type_: r.get(1)?,
                quantite: r.get(2)?,
                motif: r.get(3)?,
                horodatage: r.get(4)?,
                utilisateur: r.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(v)
}
