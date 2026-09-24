//! Rapports. RG-RAP-01 : chaque indicateur porte sa formule écrite.

use rusqlite::{params, Connection};
use serde::Serialize;

use crate::erreur::Resultat;
use crate::impression::{fcfa, ligne_montant};
use crate::{caisse, employes, horloge, journee, paie, stock};

#[derive(Debug, Serialize, Clone)]
pub struct Indicateur {
    pub cle: String,
    pub libelle: String,
    pub valeur: i64,
    pub formule: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct Tableau {
    pub titre: String,
    pub colonnes: Vec<String>,
    /// Cellules : texte ou nombre (FCFA entiers).
    pub lignes: Vec<Vec<serde_json::Value>>,
    pub formule: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct Rapport {
    pub titre: String,
    pub debut: String,
    pub fin: String,
    pub indicateurs: Vec<Indicateur>,
    pub tableaux: Vec<Tableau>,
}

fn ind(cle: &str, libelle: &str, valeur: i64, formule: &str) -> Indicateur {
    Indicateur { cle: cle.into(), libelle: libelle.into(), valeur, formule: formule.into() }
}

/// Commandes vendues de la période (journées d'exploitation `debut`..=`fin`).
const CMD_VENDUES: &str = "SELECT c.id FROM commandes c JOIN journees j ON j.id = c.journee_id
    WHERE j.date_exploitation BETWEEN ?1 AND ?2 AND c.statut IN ('payee','cloturee') AND c.employe_id IS NULL";

const F_CA: &str = "CA = Σ (quantité servie × prix) des articles non offerts + frais de livraison − remises, \
                    sur les additions payées de la période (hors consommations des employés)";
const F_COUT: &str = "Coût estimé = Σ quantité servie × coût unitaire (dernier prix d'achat, sinon prix d'achat estimé du produit), \
                      articles offerts compris";
const F_DEP: &str = "Dépenses = Σ dépenses de la période − dépenses annulées (les achats de stock et les retraits propriétaire n'y sont pas)";
const F_SAL: &str = "Salaires = Σ salaires, primes et tâches dus sur la période − absences déduites (compte employé)";
const F_BEN: &str = "Bénéfice estimé = CA − coût estimé des produits vendus − dépenses − salaires (RG-RAP-03)";

pub struct Chiffres {
    pub ca: i64,
    pub brut: i64,
    pub remises: i64,
    pub frais_livraison: i64,
    pub offerts: i64,
    pub cout: i64,
    pub depenses: i64,
    pub salaires: i64,
    pub nb_commandes: i64,
}

pub fn chiffres(conn: &Connection, debut: &str, fin: &str) -> Resultat<Chiffres> {
    let (brut, offerts, cout): (i64, i64, i64) = conn.query_row(
        &format!(
            "SELECT COALESCE(SUM(CASE WHEN l.offert = 0 THEN (l.quantite - l.quantite_annulee) * (l.prix_unitaire + l.montant_options) END), 0),
                    COALESCE(SUM(CASE WHEN l.offert = 1 THEN (l.quantite - l.quantite_annulee) * (l.prix_unitaire + l.montant_options) END), 0),
                    COALESCE(SUM((l.quantite - l.quantite_annulee) *
                        CASE WHEN l.cout_unitaire > 0 THEN l.cout_unitaire ELSE p.prix_achat_estime END), 0)
             FROM lignes_commande l JOIN produits p ON p.id = l.produit_id
             WHERE l.commande_id IN ({CMD_VENDUES})"
        ),
        params![debut, fin],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )?;
    let remises: i64 = conn.query_row(
        &format!("SELECT COALESCE(SUM(montant), 0) FROM remises WHERE commande_id IN ({CMD_VENDUES})"),
        params![debut, fin],
        |r| r.get(0),
    )?;
    let (frais, nb): (i64, i64) = conn.query_row(
        &format!("SELECT COALESCE(SUM(livraison_frais), 0), COUNT(*) FROM commandes WHERE id IN ({CMD_VENDUES})"),
        params![debut, fin],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    let depenses: i64 = conn.query_row(
        "SELECT COALESCE(SUM(CASE WHEN d.annule_depense_id IS NULL THEN d.montant ELSE -d.montant END), 0)
         FROM depenses d JOIN journees j ON j.id = d.journee_id WHERE j.date_exploitation BETWEEN ?1 AND ?2",
        params![debut, fin],
        |r| r.get(0),
    )?;
    let salaires: i64 = conn.query_row(
        "SELECT COALESCE(SUM(montant), 0) FROM mouvements_employe
         WHERE type IN ('salaire','prime','tache','deduction_absence') AND date BETWEEN ?1 AND ?2",
        params![debut, fin],
        |r| r.get(0),
    )?;
    Ok(Chiffres {
        ca: (brut - remises).max(0) + frais,
        brut,
        remises,
        frais_livraison: frais,
        offerts,
        cout,
        depenses,
        salaires,
        nb_commandes: nb,
    })
}

fn indicateurs_financiers(c: &Chiffres) -> Vec<Indicateur> {
    vec![
        ind("ca", "Chiffre d'affaires", c.ca, F_CA),
        ind("cout", "Coût estimé des produits vendus", c.cout, F_COUT),
        ind("depenses", "Dépenses", c.depenses, F_DEP),
        ind("salaires", "Salaires de la période", c.salaires, F_SAL),
        ind("benefice", "Bénéfice estimé", c.ca - c.cout - c.depenses - c.salaires, F_BEN),
    ]
}

// ───────────── Tableau de bord ─────────────

#[derive(Debug, Serialize)]
pub struct TableauDeBord {
    pub journee: Option<journee::Journee>,
    pub indicateurs: Vec<Indicateur>,
    pub ventes_par_type: Vec<(String, i64, i64)>,
    pub encaissements: Vec<(String, i64)>,
    pub mobile_money_a_verifier: (i64, i64),
    pub stock_critique: Vec<(String, i64, String)>,
    pub commandes_en_attente: i64,
    pub annulations: (i64, i64),
    pub remises: (i64, i64),
    pub offerts: (i64, i64),
    pub employes_presents: i64,
    pub employes_actifs: i64,
    pub salaires_a_payer: i64,
    pub avances_en_cours: i64,
    pub sessions_ouvertes: i64,
    pub impressions_en_erreur: i64,
}

/// Le propriétaire comprend sa journée en 10 secondes (cahier §5).
pub fn tableau_de_bord(conn: &Connection) -> Resultat<TableauDeBord> {
    let j = journee::ouverte(conn)?.or(journee::lister(conn, 1)?.into_iter().next());
    let Some(j) = j else {
        return Ok(TableauDeBord {
            journee: None,
            indicateurs: vec![],
            ventes_par_type: vec![],
            encaissements: vec![],
            mobile_money_a_verifier: (0, 0),
            stock_critique: vec![],
            commandes_en_attente: 0,
            annulations: (0, 0),
            remises: (0, 0),
            offerts: (0, 0),
            employes_presents: 0,
            employes_actifs: 0,
            salaires_a_payer: 0,
            avances_en_cours: 0,
            sessions_ouvertes: 0,
            impressions_en_erreur: 0,
        });
    };
    let d = &j.date_exploitation;
    let c = chiffres(conn, d, d)?;
    let mut indicateurs = indicateurs_financiers(&c);
    indicateurs.insert(1, ind("nb_commandes", "Commandes payées", c.nb_commandes, "Nombre d'additions payées de la journée"));
    let credit: i64 = conn.query_row(
        "SELECT COALESCE(SUM(pp.montant), 0) FROM parts_paiement pp JOIN paiements p ON p.id = pp.paiement_id
         WHERE pp.moyen = 'credit' AND p.journee_id = ?1",
        params![j.id],
        |r| r.get(0),
    )?;
    indicateurs.push(ind("credit", "Ventes à crédit", credit, "Σ parts « crédit » des paiements de la journée"));
    let mut s = conn.prepare(&format!(
        "SELECT c.type, COUNT(*), 0 FROM commandes c WHERE c.id IN ({CMD_VENDUES}) GROUP BY c.type"
    ))?;
    let types: Vec<(String, i64, i64)> = s.query_map(params![d, d], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?.collect::<Result<_, _>>()?;
    let mut ventes_par_type = Vec::new();
    for (t, n, _) in types {
        let montant: i64 = conn.query_row(
            &format!(
                "SELECT COALESCE(SUM((l.quantite - l.quantite_annulee) * (l.prix_unitaire + l.montant_options)), 0)
                 FROM lignes_commande l JOIN commandes c ON c.id = l.commande_id
                 WHERE l.offert = 0 AND c.type = ?3 AND c.id IN ({CMD_VENDUES})"
            ),
            params![d, d, t],
            |r| r.get(0),
        )?;
        ventes_par_type.push((t, n, montant));
    }
    let mut s = conn.prepare(
        "SELECT CASE WHEN pp.moyen = 'mobile_money' THEN c.nom WHEN pp.moyen = 'especes' THEN 'Espèces'
                     WHEN pp.moyen = 'credit' THEN 'Crédit' ELSE pp.moyen END, SUM(pp.montant)
         FROM parts_paiement pp JOIN paiements p ON p.id = pp.paiement_id LEFT JOIN comptes_tresorerie c ON c.id = pp.compte_id
         WHERE p.journee_id = ?1 GROUP BY 1 ORDER BY 2 DESC",
    )?;
    let encaissements = s.query_map(params![j.id], |r| Ok((r.get(0)?, r.get(1)?)))?.collect::<Result<Vec<_>, _>>()?;
    let a_verifier = caisse::parts_mobile_money(conn, Some("a_verifier"))?;
    let mm = (a_verifier.len() as i64, a_verifier.iter().map(|p| p.montant).sum());
    let stock_critique = stock::niveaux(conn)?
        .into_iter()
        .filter(|n| n.alerte)
        .map(|n| (n.nom, n.quantite, n.unite))
        .collect();
    let commandes_en_attente: i64 =
        conn.query_row("SELECT COUNT(*) FROM commandes WHERE statut = 'ouverte'", [], |r| r.get(0))?;
    let annulations: (i64, i64) = conn.query_row(
        "SELECT COUNT(*), COALESCE(SUM(a.montant), 0) FROM annulations a JOIN commandes c ON c.id = a.commande_id WHERE c.journee_id = ?1",
        params![j.id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    let remises: (i64, i64) = conn.query_row(
        "SELECT COUNT(*), COALESCE(SUM(r.montant), 0) FROM remises r JOIN commandes c ON c.id = r.commande_id
         WHERE c.journee_id = ?1 AND r.montant > 0 AND r.motif <> 'Fusion'",
        params![j.id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    let offerts: (i64, i64) = conn.query_row(
        "SELECT COUNT(*), COALESCE(SUM((l.quantite - l.quantite_annulee) * (l.prix_unitaire + l.montant_options)), 0)
         FROM lignes_commande l JOIN commandes c ON c.id = l.commande_id WHERE c.journee_id = ?1 AND l.offert = 1",
        params![j.id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    let presents = employes::presences(conn, None, d, d)?
        .iter()
        .filter(|p| p.statut == "present" || p.statut == "retard")
        .count() as i64;
    let actifs: i64 = conn.query_row("SELECT COUNT(*) FROM employes WHERE statut = 'actif'", [], |r| r.get(0))?;
    let (salaires_a_payer, avances_en_cours) = paie::a_payer(conn)?;
    let sessions_ouvertes: i64 =
        conn.query_row("SELECT COUNT(*) FROM sessions_caisse WHERE statut = 'ouverte'", [], |r| r.get(0))?;
    let impressions_en_erreur: i64 =
        conn.query_row("SELECT COUNT(*) FROM impressions WHERE statut = 'erreur' AND tentatives < 99", [], |r| r.get(0))?;
    Ok(TableauDeBord {
        journee: Some(j),
        indicateurs,
        ventes_par_type,
        encaissements,
        mobile_money_a_verifier: mm,
        stock_critique,
        commandes_en_attente,
        annulations,
        remises,
        offerts,
        employes_presents: presents,
        employes_actifs: actifs,
        salaires_a_payer,
        avances_en_cours,
        sessions_ouvertes,
        impressions_en_erreur,
    })
}

// ───────────── Rapport de période ─────────────

fn requete_tableau(
    conn: &Connection,
    titre: &str,
    colonnes: &[&str],
    sql: &str,
    p: &[&dyn rusqlite::ToSql],
    formule: Option<&str>,
) -> Resultat<Tableau> {
    let mut s = conn.prepare(sql)?;
    let n = colonnes.len();
    let lignes = s
        .query_map(p, |r| {
            let mut v = Vec::with_capacity(n);
            for i in 0..n {
                let val: rusqlite::types::Value = r.get(i)?;
                v.push(match val {
                    rusqlite::types::Value::Integer(i) => serde_json::json!(i),
                    rusqlite::types::Value::Text(t) => serde_json::json!(t),
                    rusqlite::types::Value::Null => serde_json::Value::Null,
                    rusqlite::types::Value::Real(f) => serde_json::json!(f),
                    rusqlite::types::Value::Blob(_) => serde_json::Value::Null,
                });
            }
            Ok(v)
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Tableau {
        titre: titre.into(),
        colonnes: colonnes.iter().map(|c| c.to_string()).collect(),
        lignes,
        formule: formule.map(str::to_owned),
    })
}

/// Rapport complet sur des journées d'exploitation (`debut`..=`fin`, AAAA-MM-JJ).
pub fn rapport_periode(conn: &Connection, debut: &str, fin: &str) -> Resultat<Rapport> {
    let c = chiffres(conn, debut, fin)?;
    let mut indicateurs = indicateurs_financiers(&c);
    indicateurs.insert(1, ind("nb_commandes", "Commandes payées", c.nb_commandes, "Nombre d'additions payées"));
    indicateurs.insert(2, ind("remises", "Remises", c.remises, "Σ remises accordées sur les additions payées"));
    indicateurs.insert(3, ind("offerts", "Articles offerts (valeur de vente)", c.offerts, "Σ quantité × prix des articles offerts"));
    let p: &[&dyn rusqlite::ToSql] = &[&debut, &fin];
    let lignes_vendues = format!("FROM lignes_commande l JOIN produits p ON p.id = l.produit_id JOIN categories k ON k.id = p.categorie_id
         JOIN commandes c ON c.id = l.commande_id WHERE l.offert = 0 AND l.commande_id IN ({CMD_VENDUES})");
    let mut tableaux = vec![
        requete_tableau(
            conn,
            "Ventes par journée",
            &["Journée", "Commandes", "Chiffre d'affaires (brut)"],
            &format!(
                "SELECT j.date_exploitation, COUNT(DISTINCT c.id), COALESCE(SUM((l.quantite - l.quantite_annulee) * (l.prix_unitaire + l.montant_options)), 0)
                 FROM commandes c JOIN journees j ON j.id = c.journee_id
                 LEFT JOIN lignes_commande l ON l.commande_id = c.id AND l.offert = 0
                 WHERE c.id IN ({CMD_VENDUES}) GROUP BY j.date_exploitation ORDER BY j.date_exploitation"
            ),
            p,
            Some("Brut = avant remises et frais de livraison"),
        )?,
        requete_tableau(
            conn,
            "Produits les plus vendus",
            &["Produit", "Quantité", "Montant"],
            &format!(
                "SELECT p.nom, SUM(l.quantite - l.quantite_annulee), SUM((l.quantite - l.quantite_annulee) * (l.prix_unitaire + l.montant_options))
                 {lignes_vendues} GROUP BY p.id ORDER BY 2 DESC, 3 DESC"
            ),
            p,
            None,
        )?,
        requete_tableau(
            conn,
            "Ventes par catégorie",
            &["Catégorie", "Quantité", "Montant"],
            &format!(
                "SELECT k.nom, SUM(l.quantite - l.quantite_annulee), SUM((l.quantite - l.quantite_annulee) * (l.prix_unitaire + l.montant_options))
                 {lignes_vendues} GROUP BY k.id ORDER BY 3 DESC"
            ),
            p,
            None,
        )?,
        requete_tableau(
            conn,
            "Ventes par serveur",
            &["Serveur", "Commandes", "Montant"],
            &format!(
                "SELECT COALESCE(u.nom, '—'), COUNT(DISTINCT c.id), SUM((l.quantite - l.quantite_annulee) * (l.prix_unitaire + l.montant_options))
                 {lignes_vendues} AND 1 = 1 GROUP BY c.serveur_id ORDER BY 3 DESC"
            )
            .replace("JOIN commandes c ON c.id = l.commande_id", "JOIN commandes c ON c.id = l.commande_id LEFT JOIN utilisateurs u ON u.id = c.serveur_id"),
            p,
            None,
        )?,
        requete_tableau(
            conn,
            "Ventes par type de commande",
            &["Type", "Commandes", "Montant"],
            &format!(
                "SELECT c.type, COUNT(DISTINCT c.id), SUM((l.quantite - l.quantite_annulee) * (l.prix_unitaire + l.montant_options))
                 {lignes_vendues} GROUP BY c.type ORDER BY 3 DESC"
            ),
            p,
            None,
        )?,
        requete_tableau(
            conn,
            "Encaissements par moyen de paiement",
            &["Moyen", "Compte", "Montant"],
            "SELECT pp.moyen, COALESCE(t.nom, '—'), SUM(pp.montant) FROM parts_paiement pp JOIN paiements p ON p.id = pp.paiement_id
             JOIN journees j ON j.id = p.journee_id LEFT JOIN comptes_tresorerie t ON t.id = pp.compte_id
             WHERE j.date_exploitation BETWEEN ?1 AND ?2 GROUP BY pp.moyen, pp.compte_id ORDER BY 3 DESC",
            p,
            Some("Paiements moins annulations de paiement (contre-passations)"),
        )?,
        requete_tableau(
            conn,
            "Annulations après envoi",
            &["Heure", "Article", "Qté", "Montant", "Motif", "Par", "Autorisé par", "Perte"],
            "SELECT strftime('%d/%m %H:%M', a.horodatage / 1000, 'unixepoch'), l.libelle, a.quantite, a.montant, a.motif,
                    u.nom, v.nom, CASE WHEN a.perte = 1 THEN 'oui' ELSE 'non' END
             FROM annulations a JOIN commandes c ON c.id = a.commande_id JOIN journees j ON j.id = c.journee_id
             JOIN lignes_commande l ON l.id = a.ligne_id LEFT JOIN utilisateurs u ON u.id = a.utilisateur_id
             LEFT JOIN utilisateurs v ON v.id = a.autorise_par
             WHERE j.date_exploitation BETWEEN ?1 AND ?2 ORDER BY a.horodatage",
            p,
            None,
        )?,
        requete_tableau(
            conn,
            "Remises et articles offerts",
            &["Type", "Commande", "Montant", "Motif", "Par", "Autorisé par"],
            "SELECT 'Remise', c.numero, r.montant, r.motif, u.nom, v.nom FROM remises r JOIN commandes c ON c.id = r.commande_id
             JOIN journees j ON j.id = c.journee_id LEFT JOIN utilisateurs u ON u.id = r.utilisateur_id
             LEFT JOIN utilisateurs v ON v.id = r.autorise_par
             WHERE j.date_exploitation BETWEEN ?1 AND ?2 AND r.motif <> 'Fusion'
             UNION ALL
             SELECT 'Offert', c.numero, (l.quantite - l.quantite_annulee) * (l.prix_unitaire + l.montant_options), l.offert_motif, u.nom, NULL
             FROM lignes_commande l JOIN commandes c ON c.id = l.commande_id JOIN journees j ON j.id = c.journee_id
             LEFT JOIN utilisateurs u ON u.id = l.cree_par WHERE l.offert = 1 AND j.date_exploitation BETWEEN ?1 AND ?2",
            p,
            None,
        )?,
        requete_tableau(
            conn,
            "Écarts de caisse par caissier",
            &["Caissier", "Sessions", "Écart total", "Plus gros manquant"],
            "SELECT u.nom, COUNT(*), COALESCE(SUM(s.ecart), 0), COALESCE(MIN(s.ecart), 0) FROM sessions_caisse s
             JOIN utilisateurs u ON u.id = s.caissier_id JOIN journees j ON j.id = s.journee_id
             WHERE s.statut = 'fermee' AND j.date_exploitation BETWEEN ?1 AND ?2 GROUP BY s.caissier_id ORDER BY 3",
            p,
            Some("Écart = compté − théorique à la clôture ; négatif = manquant"),
        )?,
        requete_tableau(
            conn,
            "Dépenses par catégorie",
            &["Catégorie", "Nombre", "Montant"],
            "SELECT k.nom, SUM(CASE WHEN d.annule_depense_id IS NULL THEN 1 ELSE -1 END),
                    SUM(CASE WHEN d.annule_depense_id IS NULL THEN d.montant ELSE -d.montant END)
             FROM depenses d JOIN categories_depense k ON k.id = d.categorie_id JOIN journees j ON j.id = d.journee_id
             WHERE j.date_exploitation BETWEEN ?1 AND ?2 GROUP BY k.id ORDER BY 3 DESC",
            p,
            Some(F_DEP),
        )?,
        requete_tableau(
            conn,
            "Retraits propriétaire",
            &["Date", "Montant", "Libellé"],
            "SELECT strftime('%d/%m %H:%M', m.horodatage / 1000, 'unixepoch'), -m.montant, m.libelle FROM mouvements_tresorerie m
             JOIN journees j ON j.id = m.journee_id JOIN comptes_tresorerie c ON c.id = m.compte_id
             WHERE m.type = 'retrait_proprietaire' AND c.type = 'especes' AND j.date_exploitation BETWEEN ?1 AND ?2 ORDER BY m.horodatage",
            p,
            Some("RG-CAI-10 : les retraits propriétaire ne sont pas des dépenses"),
        )?,
        requete_tableau(
            conn,
            "Achats",
            &["N°", "Date", "Fournisseur", "Mode", "Total"],
            "SELECT a.numero, strftime('%d/%m', a.horodatage / 1000, 'unixepoch'), COALESCE(f.nom, 'Marché'), a.mode, a.total
             FROM achats a LEFT JOIN fournisseurs f ON f.id = a.fournisseur_id LEFT JOIN journees j ON j.id = a.journee_id
             WHERE j.date_exploitation BETWEEN ?1 AND ?2 ORDER BY a.horodatage",
            p,
            None,
        )?,
    ];
    tableaux.push(ou_part_le_stock(conn, debut, fin)?);
    tableaux.push(requete_tableau(
        conn,
        "Salaires et avances",
        &["Employé", "Dû (salaire, primes, tâches)", "Avances", "Retenues et consommations", "Payé"],
        "SELECT e.nom,
                COALESCE(SUM(CASE WHEN m.type IN ('salaire','prime','tache','deduction_absence') THEN m.montant END), 0),
                COALESCE(-SUM(CASE WHEN m.type = 'avance' THEN m.montant END), 0),
                COALESCE(-SUM(CASE WHEN m.type IN ('retenue','consommation') THEN m.montant END), 0),
                COALESCE(-SUM(CASE WHEN m.type = 'paiement' THEN m.montant END), 0)
         FROM mouvements_employe m JOIN employes e ON e.id = m.employe_id
         WHERE m.date BETWEEN ?1 AND ?2 GROUP BY e.id ORDER BY e.nom",
        p,
        Some(F_SAL),
    )?);
    Ok(Rapport { titre: "Rapport d'activité".into(), debut: debut.into(), fin: fin.into(), indicateurs, tableaux })
}

/// « Où part le stock » (cahier §9) : entrées, ventes, sorties hors vente et écarts d'inventaire par article.
pub fn ou_part_le_stock(conn: &Connection, debut: &str, fin: &str) -> Resultat<Tableau> {
    requete_tableau(
        conn,
        "Où part le stock",
        &["Article", "Achats", "Ventes", "Offerts", "Employés", "Pertes/casse/vol", "Écarts inventaire", "Valeur des pertes et écarts"],
        "SELECT a.nom,
            COALESCE(SUM(CASE WHEN m.type = 'achat' THEN m.quantite END), 0),
            COALESCE(-SUM(CASE WHEN m.type IN ('vente','annulation_vente') THEN m.quantite END), 0),
            COALESCE(-SUM(CASE WHEN m.type = 'offert' THEN m.quantite END), 0),
            COALESCE(-SUM(CASE WHEN m.type IN ('consommation_employe','repas_personnel') THEN m.quantite END), 0),
            COALESCE(-SUM(CASE WHEN m.type IN ('perte','perime','casse','vol','consommation_interne') THEN m.quantite END), 0),
            COALESCE(SUM(CASE WHEN m.type = 'inventaire' THEN m.quantite END), 0),
            COALESCE(SUM(CASE WHEN m.type IN ('perte','perime','casse','vol','inventaire') THEN m.quantite * m.cout_unitaire END), 0)
         FROM mouvements_stock m JOIN articles_stock a ON a.id = m.article_id
         LEFT JOIN journees j ON j.id = m.journee_id
         WHERE COALESCE(j.date_exploitation, date(m.horodatage / 1000, 'unixepoch')) BETWEEN ?1 AND ?2
         GROUP BY a.id ORDER BY 8",
        &[&debut, &fin],
        Some("Écart d'inventaire = compté − théorique (négatif = manquant). Valeur au coût unitaire du moment."),
    )
}

/// Rapport de stock et valeur (instantané).
pub fn rapport_stock(conn: &Connection) -> Resultat<Rapport> {
    let n = stock::niveaux(conn)?;
    let valeur: i64 = n.iter().map(|x| x.valeur).sum();
    Ok(Rapport {
        titre: "Stock et valeur".into(),
        debut: String::new(),
        fin: String::new(),
        indicateurs: vec![ind("valeur_stock", "Valeur du stock", valeur, "Σ quantité positive × coût unitaire (dernier prix d'achat)")],
        tableaux: vec![Tableau {
            titre: "Niveaux".into(),
            colonnes: vec!["Article".into(), "Quantité".into(), "Unité".into(), "Seuil".into(), "Coût unitaire".into(), "Valeur".into()],
            lignes: n
                .iter()
                .map(|x| {
                    vec![
                        serde_json::json!(x.nom),
                        serde_json::json!(x.quantite),
                        serde_json::json!(x.unite),
                        serde_json::json!(x.seuil_alerte),
                        serde_json::json!(x.cout_unitaire),
                        serde_json::json!(x.valeur),
                    ]
                })
                .collect(),
            formule: Some("Quantité = Σ mouvements de stock".into()),
        }],
    })
}

/// Dettes clients et fournisseurs.
pub fn rapport_dettes(conn: &Connection, maintenant: i64) -> Resultat<Rapport> {
    let clients = crate::clients::dettes_par_anciennete(conn, maintenant)?;
    let fournisseurs = crate::achats::lister_fournisseurs(conn)?;
    let total_c: i64 = clients.iter().map(|c| c.dette).sum();
    let total_f: i64 = fournisseurs.iter().map(|f| f.dette).sum();
    Ok(Rapport {
        titre: "Dettes".into(),
        debut: String::new(),
        fin: String::new(),
        indicateurs: vec![
            ind("dettes_clients", "Dettes clients (on vous doit)", total_c, "Σ ventes à crédit − règlements clients"),
            ind("dettes_fournisseurs", "Dettes fournisseurs (vous devez)", total_f, "Σ achats à crédit − règlements fournisseurs"),
        ],
        tableaux: vec![
            Tableau {
                titre: "Dettes clients par ancienneté".into(),
                colonnes: vec!["Client".into(), "Téléphone".into(), "Dette".into(), "Limite".into(), "Ancienneté".into()],
                lignes: clients
                    .iter()
                    .map(|c| {
                        vec![
                            serde_json::json!(c.nom),
                            serde_json::json!(c.telephone),
                            serde_json::json!(c.dette),
                            serde_json::json!(c.limite),
                            serde_json::json!(c.tranche),
                        ]
                    })
                    .collect(),
                formule: Some("Ancienneté = âge de la plus ancienne vente à crédit non réglée (les règlements soldent d'abord les plus anciennes)".into()),
            },
            Tableau {
                titre: "Dettes fournisseurs".into(),
                colonnes: vec!["Fournisseur".into(), "Téléphone".into(), "Dette".into()],
                lignes: fournisseurs
                    .iter()
                    .filter(|f| f.dette != 0)
                    .map(|f| vec![serde_json::json!(f.nom), serde_json::json!(f.telephone), serde_json::json!(f.dette)])
                    .collect(),
                formule: None,
            },
        ],
    })
}

/// Export CSV (séparateur `;`, BOM UTF-8 pour Excel).
pub fn en_csv(r: &Rapport) -> String {
    let echapper = |v: &serde_json::Value| -> String {
        let s = match v {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Null => String::new(),
            autre => autre.to_string(),
        };
        if s.contains(';') || s.contains('"') || s.contains('\n') {
            format!("\"{}\"", s.replace('"', "\"\""))
        } else {
            s
        }
    };
    let mut out = String::from("\u{feff}");
    out.push_str(&format!("{};{} ;{}\n\n", r.titre, r.debut, r.fin));
    for i in &r.indicateurs {
        out.push_str(&format!("{};{};{}\n", i.libelle, i.valeur, echapper(&serde_json::json!(i.formule))));
    }
    for t in &r.tableaux {
        out.push_str(&format!("\n{}\n", t.titre));
        out.push_str(&t.colonnes.iter().map(|c| echapper(&serde_json::json!(c))).collect::<Vec<_>>().join(";"));
        out.push('\n');
        for l in &t.lignes {
            out.push_str(&l.iter().map(echapper).collect::<Vec<_>>().join(";"));
            out.push('\n');
        }
    }
    out
}

// ───────────── Rapport de clôture de caisse (Z) ─────────────

/// Ticket Z 80 mm d'une session de caisse.
pub fn rapport_z(conn: &Connection, session_id: &str) -> Resultat<String> {
    let s = caisse::session(conn, session_id)?;
    let p = crate::parametres::lire(conn)?;
    let w = p.largeur_ticket;
    let nom: String = conn.query_row("SELECT nom FROM restaurant LIMIT 1", [], |r| r.get(0))?;
    let j = journee::par_id(conn, &s.journee_id)?;
    let mut t = format!("##{nom}\n##RAPPORT Z\n");
    t.push_str(&format!("**Journée du {}\n", j.date_exploitation));
    t.push_str(&format!("Caisse : {}\nCaissier : {}\n", s.compte_nom, s.caissier_nom));
    t.push_str(&format!("Ouverture : {}\n", horloge::format_ms(s.ouverte_le + p.fuseau_minutes * 60_000)));
    if let Some(f) = s.fermee_le {
        t.push_str(&format!("Clôture : {}\n", horloge::format_ms(f + p.fuseau_minutes * 60_000)));
    }
    t.push_str("--\n");
    let mut stmt = conn.prepare(
        "SELECT type, COUNT(*), SUM(montant) FROM mouvements_tresorerie WHERE session_id = ?1 AND compte_id = ?2 GROUP BY type ORDER BY type",
    )?;
    let par_type: Vec<(String, i64, i64)> =
        stmt.query_map(params![session_id, s.compte_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?.collect::<Result<_, _>>()?;
    t.push_str(&ligne_montant("Fond compté à l'ouverture", &fcfa(s.fond_compte), w));
    t.push('\n');
    for (typ, n, m) in &par_type {
        if typ == "ecart_ouverture" || typ == "ecart_cloture" {
            continue;
        }
        t.push_str(&ligne_montant(&format!("{} ({n})", libelle_mouvement(typ)), &fcfa(*m), w));
        t.push('\n');
    }
    t.push_str("--\n");
    let theorique = s.theorique_cloture.unwrap_or(s.solde_actuel);
    t.push_str(&ligne_montant("Espèces attendues", &fcfa(theorique), w));
    t.push('\n');
    if let Some(c) = s.compte_final {
        t.push_str(&ligne_montant("Espèces comptées", &fcfa(c), w));
        t.push('\n');
        t.push_str(&format!("**{}\n", ligne_montant("ÉCART", &fcfa(s.ecart.unwrap_or(0)), w)));
        if let Some(m) = s.motif_ecart.as_ref().filter(|m| !m.is_empty()) {
            t.push_str(&format!("Motif : {m}\n"));
        }
    }
    t.push_str("--\n**Autres encaissements de la session\n");
    let mut stmt = conn.prepare(
        "SELECT CASE WHEN pp.moyen = 'credit' THEN 'Crédit (ardoise)' ELSE COALESCE(c.nom, pp.moyen) END, SUM(pp.montant)
         FROM parts_paiement pp JOIN paiements p ON p.id = pp.paiement_id LEFT JOIN comptes_tresorerie c ON c.id = pp.compte_id
         WHERE p.session_id = ?1 AND pp.moyen <> 'especes' GROUP BY 1",
    )?;
    for l in stmt.query_map(params![session_id], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))? {
        let (nom, m) = l?;
        t.push_str(&ligne_montant(&nom, &fcfa(m), w));
        t.push('\n');
    }
    let billets = caisse::billetage(conn, session_id, "cloture")?;
    if !billets.is_empty() {
        t.push_str("--\n**Billetage\n");
        for b in billets {
            t.push_str(&ligne_montant(&format!("{} × {}", b.nombre, fcfa(b.coupure)), &fcfa(b.nombre * b.coupure), w));
            t.push('\n');
        }
    }
    t.push_str("--\nSignature caissier :\n\n\nSignature responsable :\n");
    Ok(t)
}

pub fn libelle_mouvement(t: &str) -> &str {
    match t {
        "vente" => "Ventes espèces",
        "annulation_vente" => "Annulations de paiement",
        "depense" => "Dépenses",
        "annulation_depense" => "Dépenses annulées",
        "retrait_proprietaire" => "Retraits propriétaire",
        "retrait" => "Retraits",
        "entree_diverse" => "Entrées diverses",
        "apport" => "Apports",
        "achat" => "Achats payés",
        "reglement_client" => "Règlements clients",
        "reglement_fournisseur" => "Règlements fournisseurs",
        "avance_salaire" => "Avances sur salaire",
        "paiement_salaire" => "Salaires payés",
        "transfert_sortant" => "Transferts sortants",
        "transfert_entrant" => "Transferts entrants",
        "remise_livreur" => "Remises livreurs",
        "ecart_ouverture" => "Écart d'ouverture",
        "ecart_cloture" => "Écart de clôture",
        "ecart_livreur" => "Écart livreur",
        autre => autre,
    }
}
