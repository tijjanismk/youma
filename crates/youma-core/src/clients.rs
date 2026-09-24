use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::catalogue::non_vide;
use crate::db::{trouver, Acteur, Db, Op};
use crate::erreur::{Erreur, Resultat};
use crate::permissions as perm;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Client {
    #[serde(default)]
    pub id: String,
    pub nom: String,
    #[serde(default)]
    pub telephone: Option<String>,
    #[serde(default)]
    pub adresse: String,
    #[serde(default)]
    pub reperes: String,
    /// RG-CLI-01 : faux par défaut.
    #[serde(default)]
    pub credit_autorise: bool,
    #[serde(default)]
    pub limite_credit: i64,
    #[serde(default = "vrai")]
    pub actif: bool,
    #[serde(default)]
    pub dette: i64,
}

fn vrai() -> bool {
    true
}

pub fn dette(conn: &Connection, client_id: &str) -> Resultat<i64> {
    Ok(conn.query_row(
        "SELECT COALESCE(SUM(montant), 0) FROM mouvements_client WHERE client_id = ?1",
        params![client_id],
        |r| r.get(0),
    )?)
}

fn client_depuis(r: &rusqlite::Row) -> rusqlite::Result<Client> {
    Ok(Client {
        id: r.get(0)?,
        nom: r.get(1)?,
        telephone: r.get(2)?,
        adresse: r.get(3)?,
        reperes: r.get(4)?,
        credit_autorise: r.get(5)?,
        limite_credit: r.get(6)?,
        actif: r.get(7)?,
        dette: r.get(8)?,
    })
}

const COLS: &str = "c.id, c.nom, c.telephone, c.adresse, c.reperes, c.credit_autorise, c.limite_credit, c.actif,
    (SELECT COALESCE(SUM(m.montant), 0) FROM mouvements_client m WHERE m.client_id = c.id)";

/// Recherche par nom ou téléphone (clé principale).
pub fn rechercher(conn: &Connection, texte: &str) -> Resultat<Vec<Client>> {
    let motif = format!("%{}%", texte.trim());
    let mut s = conn.prepare(&format!(
        "SELECT {COLS} FROM clients c WHERE c.actif = 1 AND (c.nom LIKE ?1 OR c.telephone LIKE ?1)
         ORDER BY c.nom LIMIT 100"
    ))?;
    let v = s.query_map(params![motif], client_depuis)?.collect::<Result<Vec<_>, _>>()?;
    Ok(v)
}

pub fn client(conn: &Connection, id: &str) -> Resultat<Client> {
    trouver(conn.query_row(&format!("SELECT {COLS} FROM clients c WHERE c.id = ?1"), params![id], client_depuis), "Client")
}

pub fn enregistrer(db: &mut Db, acteur: &Acteur, c: &Client) -> Resultat<String> {
    db.executer(acteur, |op| {
        op.exiger(perm::CLIENT_GERER)?;
        non_vide(&c.nom, "nom du client")?;
        let tel = c.telephone.as_deref().map(|t| t.chars().filter(|c| !c.is_whitespace()).collect::<String>()).filter(|t| !t.is_empty());
        if let Some(t) = &tel {
            let autre: i64 = op.query_row(
                "SELECT COUNT(*) FROM clients WHERE telephone = ?1 AND id <> ?2",
                params![t, c.id],
                |r| r.get(0),
            )?;
            if autre > 0 {
                return Err(Erreur::regle("RG-CLI-04", "Un client a déjà ce numéro de téléphone"));
            }
        }
        let id = if c.id.is_empty() { op.nouvel_id() } else { c.id.clone() };
        let (avant_credit, avant_limite): (bool, i64) = op
            .query_row("SELECT credit_autorise, limite_credit FROM clients WHERE id = ?1", params![id], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap_or((false, 0));
        let mut autorise_par = None;
        if c.credit_autorise != avant_credit || c.limite_credit != avant_limite {
            autorise_par = op.exiger(perm::CLIENT_CREDIT)?;
        }
        if c.limite_credit < 0 {
            return Err(Erreur::validation("Limite de crédit négative"));
        }
        op.execute(
            "INSERT INTO clients(id, nom, telephone, adresse, reperes, credit_autorise, limite_credit, actif, cree_le, modifie_le)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)
             ON CONFLICT(id) DO UPDATE SET nom = excluded.nom, telephone = excluded.telephone, adresse = excluded.adresse,
               reperes = excluded.reperes, credit_autorise = excluded.credit_autorise, limite_credit = excluded.limite_credit,
               actif = excluded.actif, modifie_le = excluded.modifie_le, version = version + 1",
            params![id, c.nom.trim(), tel, c.adresse, c.reperes, c.credit_autorise, c.limite_credit, c.actif, op.maintenant],
        )?;
        if c.credit_autorise != avant_credit || c.limite_credit != avant_limite {
            op.audit(
                "client.credit",
                "client",
                Some(&id),
                Some(json!({ "credit": avant_credit, "limite": avant_limite })),
                Some(json!({ "credit": c.credit_autorise, "limite": c.limite_credit })),
                None,
                autorise_par.as_deref(),
            )?;
        }
        op.outbox("client", &id, "enregistrer")?;
        Ok(id)
    })
}

/// RG-CLI-01/02 : vente à crédit, appelée depuis l'encaissement.
pub(crate) fn vente_credit(op: &Op, client_id: &str, commande_id: &str, paiement_id: &str, montant: i64) -> Resultat<()> {
    let c = client(op, client_id)?;
    if !c.credit_autorise {
        return Err(Erreur::regle("RG-CLI-01", format!("{} n'est pas autorisé à acheter à crédit", c.nom)));
    }
    let mut autorise_par = None;
    if c.dette + montant > c.limite_credit {
        autorise_par = op.exiger(perm::CLIENT_DEPASSER_LIMITE).map_err(|_| {
            Erreur::AutorisationRequise(format!(
                "{} — dette {} + {} dépasse la limite de {}",
                perm::CLIENT_DEPASSER_LIMITE,
                c.dette,
                montant,
                c.limite_credit
            ))
        })?;
        op.audit(
            "client.depassement_limite",
            "client",
            Some(client_id),
            Some(json!({ "dette": c.dette, "limite": c.limite_credit })),
            Some(json!({ "montant": montant })),
            None,
            autorise_par.as_deref(),
        )?;
    }
    op.execute(
        "INSERT INTO mouvements_client(id, client_id, type, montant, commande_id, paiement_id, horodatage, utilisateur_id, autorise_par)
         VALUES (?1, ?2, 'vente_credit', ?3, ?4, ?5, ?6, ?7, ?8)",
        params![op.nouvel_id(), client_id, montant, commande_id, paiement_id, op.maintenant, op.utilisateur(), autorise_par],
    )?;
    Ok(())
}

pub(crate) fn contrepasser_credit(op: &Op, client_id: &str, commande_id: Option<&str>, paiement_id: &str, montant: i64, motif: &str) -> Resultat<()> {
    op.execute(
        "INSERT INTO mouvements_client(id, client_id, type, montant, commande_id, paiement_id, motif, horodatage, utilisateur_id)
         VALUES (?1, ?2, 'contrepassation', ?3, ?4, ?5, ?6, ?7, ?8)",
        params![op.nouvel_id(), client_id, -montant, commande_id, paiement_id, motif, op.maintenant, op.utilisateur()],
    )?;
    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct Reglement {
    pub client_id: String,
    pub montant: i64,
    /// Compte crédité (défaut : caisse de la session).
    #[serde(default)]
    pub compte_id: Option<String>,
    #[serde(default)]
    pub reference: String,
}

/// RG-CLI-03.
pub fn regler(db: &mut Db, acteur: &Acteur, r: &Reglement) -> Resultat<String> {
    db.executer(acteur, |op| {
        op.exiger(perm::CAISSE_ENCAISSER)?;
        op.journee_ouverte()?;
        let d = dette(op, &r.client_id)?;
        if r.montant <= 0 {
            return Err(Erreur::validation("Montant invalide"));
        }
        if r.montant > d {
            return Err(Erreur::regle("RG-CLI-03", format!("Le règlement ({}) dépasse la dette ({d})", r.montant)));
        }
        let session = op.utilisateur().map(|u| crate::caisse::session_utilisateur(op, u)).transpose()?.flatten();
        let compte = match (&r.compte_id, &session) {
            (Some(c), _) => c.clone(),
            (None, Some(s)) => s.compte_id.clone(),
            (None, None) => return Err(Erreur::regle("RG-CAI-01", "Ouvrez une session de caisse ou choisissez un compte")),
        };
        let nom = client(op, &r.client_id)?.nom;
        let id = op.nouvel_id();
        let mvt = crate::caisse::mouvement(
            op,
            &compte,
            session.as_ref().map(|s| s.id.as_str()),
            "reglement_client",
            r.montant,
            Some(("client", &r.client_id)),
            &format!("Règlement {nom} {}", r.reference.trim()).trim().to_string(),
            None,
        )?;
        op.execute(
            "INSERT INTO mouvements_client(id, client_id, type, montant, mouvement_tresorerie_id, motif, horodatage, utilisateur_id)
             VALUES (?1, ?2, 'reglement', ?3, ?4, ?5, ?6, ?7)",
            params![id, r.client_id, -r.montant, mvt, r.reference.trim(), op.maintenant, op.utilisateur()],
        )?;
        op.audit("client.reglement", "client", Some(&r.client_id), None, Some(json!({ "montant": r.montant })), None, None)?;
        op.evenement("caisse", None);
        Ok(id)
    })
}

#[derive(Debug, Serialize)]
pub struct LigneReleve {
    pub horodatage: i64,
    #[serde(rename = "type")]
    pub type_: String,
    pub montant: i64,
    pub commande_numero: Option<i64>,
    pub motif: String,
    pub solde: i64,
}

/// Relevé imprimable, solde cumulé.
pub fn releve(conn: &Connection, client_id: &str) -> Resultat<Vec<LigneReleve>> {
    let mut s = conn.prepare(
        "SELECT m.horodatage, m.type, m.montant, c.numero, m.motif FROM mouvements_client m
         LEFT JOIN commandes c ON c.id = m.commande_id WHERE m.client_id = ?1 ORDER BY m.horodatage, m.rowid",
    )?;
    let mut solde = 0;
    let v = s
        .query_map(params![client_id], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, r.get::<_, i64>(2)?, r.get(3)?, r.get::<_, String>(4)?)))?
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .map(|(h, t, m, n, motif)| {
            solde += m;
            LigneReleve { horodatage: h, type_: t, montant: m, commande_numero: n, motif, solde }
        })
        .collect();
    Ok(v)
}

#[derive(Debug, Serialize)]
pub struct DetteAgee {
    pub client_id: String,
    pub nom: String,
    pub telephone: Option<String>,
    pub dette: i64,
    pub limite: i64,
    /// Jours depuis la plus ancienne vente à crédit non couverte (méthode FIFO).
    pub anciennete_jours: i64,
    /// 0-7 j, 8-30 j, 31-90 j, > 90 j
    pub tranche: String,
}

/// Dettes clients par ancienneté (FIFO : les règlements soldent d'abord les ventes les plus anciennes).
pub fn dettes_par_anciennete(conn: &Connection, maintenant: i64) -> Resultat<Vec<DetteAgee>> {
    let clients: Vec<(String, String, Option<String>, i64, i64)> = conn
        .prepare(
            "SELECT c.id, c.nom, c.telephone, c.limite_credit, SUM(m.montant) FROM clients c
             JOIN mouvements_client m ON m.client_id = c.id GROUP BY c.id HAVING SUM(m.montant) > 0 ORDER BY SUM(m.montant) DESC",
        )?
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)))?
        .collect::<Result<_, _>>()?;
    let mut v = Vec::new();
    for (id, nom, tel, limite, d) in clients {
        let ventes: Vec<(i64, i64)> = conn
            .prepare("SELECT horodatage, montant FROM mouvements_client WHERE client_id = ?1 AND montant > 0 ORDER BY horodatage DESC")?
            .query_map(params![id], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<Result<_, _>>()?;
        // La dette restante correspond aux ventes les plus récentes ; on remonte jusqu'à la couvrir.
        let mut reste = d;
        let mut plus_ancienne = maintenant;
        for (h, m) in ventes {
            plus_ancienne = h;
            reste -= m;
            if reste <= 0 {
                break;
            }
        }
        let jours = ((maintenant - plus_ancienne) / 86_400_000).max(0);
        let tranche = match jours {
            0..=7 => "0-7 j",
            8..=30 => "8-30 j",
            31..=90 => "31-90 j",
            _ => "> 90 j",
        };
        v.push(DetteAgee { client_id: id, nom, telephone: tel, dette: d, limite, anciennete_jours: jours, tranche: tranche.into() });
    }
    Ok(v)
}
