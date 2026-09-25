//! Rapprochement Mobile Money par relevé d'opérateur (fiche 0016, RG-RMM-01 à 05).
//! Le relevé (CSV exporté d'Orange Money, Moov, Wave…) sert de preuve : un paiement saisi à la caisse dont la
//! référence figure au relevé avec le même montant est vérifié ; le reste est listé pour le responsable.

use chrono::NaiveDateTime;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use serde_json::json;

use crate::db::{trouver, Acteur, Db, Op};
use crate::erreur::{Erreur, Resultat};
use crate::permissions as perm;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LigneReleve {
    pub reference: String,
    pub montant: i64,
    pub date: Option<i64>,
    pub numero: String,
}

fn normaliser_entete(s: &str) -> String {
    s.trim()
        .trim_matches('"')
        .to_lowercase()
        .replace(['é', 'è', 'ê'], "e")
        .replace(['_', '-', '.'], " ")
}

fn colonne(entetes: &[String], candidats: &[&str]) -> Option<usize> {
    // Correspondance exacte d'abord, puis « contient ».
    entetes
        .iter()
        .position(|e| candidats.contains(&e.as_str()))
        .or_else(|| entetes.iter().position(|e| candidats.iter().any(|c| e.contains(c))))
}

/// Montant d'un relevé en FCFA entiers : « 12 500 », « 12500,00 », « 12.500 », « 12 500 FCFA ».
pub fn lire_montant(s: &str) -> Option<i64> {
    let t: String = s.chars().filter(|c| c.is_ascii_digit() || matches!(c, ',' | '.' | '-')).collect();
    if t.is_empty() {
        return None;
    }
    // Décimales (le FCFA n'a pas de centimes) : « ,00 » ou « .00 » final retiré.
    let entier = match t.rfind([',', '.']) {
        Some(i) if t.len() - i == 3 => &t[..i],
        _ => &t[..],
    };
    let chiffres: String = entier.chars().filter(|c| c.is_ascii_digit() || *c == '-').collect();
    chiffres.parse().ok()
}

/// Date d'un relevé (heure du Mali = UTC) : « 14/03/2026 18:30[:05] », « 2026-03-14 18:30:05 », « 14/03/2026 ».
pub fn lire_date(s: &str) -> Option<i64> {
    let s = s.trim().trim_matches('"');
    for f in ["%d/%m/%Y %H:%M:%S", "%d/%m/%Y %H:%M", "%Y-%m-%d %H:%M:%S", "%Y-%m-%dT%H:%M:%S", "%Y-%m-%d %H:%M", "%d-%m-%Y %H:%M:%S"] {
        if let Ok(d) = NaiveDateTime::parse_from_str(s, f) {
            return Some(d.and_utc().timestamp_millis());
        }
    }
    for f in ["%d/%m/%Y", "%Y-%m-%d"] {
        if let Ok(d) = chrono::NaiveDate::parse_from_str(s, f) {
            return d.and_hms_opt(0, 0, 0).map(|d| d.and_utc().timestamp_millis());
        }
    }
    None
}

fn decouper(ligne: &str, sep: char) -> Vec<String> {
    // CSV simple : guillemets possibles autour d'un champ contenant le séparateur.
    let mut champs = Vec::new();
    let mut courant = String::new();
    let mut entre = false;
    for c in ligne.chars() {
        match c {
            '"' => entre = !entre,
            c if c == sep && !entre => champs.push(std::mem::take(&mut courant)),
            c => courant.push(c),
        }
    }
    champs.push(courant);
    champs.into_iter().map(|c| c.trim().to_string()).collect()
}

/// RG-RMM-01 : lecture d'un relevé CSV. Séparateur (`;` `,` tabulation) et colonnes reconnus par leur nom ;
/// seules les lignes créditrices (montant > 0) avec une référence sont gardées.
pub fn lire_csv(texte: &str) -> Resultat<Vec<LigneReleve>> {
    let texte = texte.trim_start_matches('\u{feff}');
    let mut lignes = texte.lines().filter(|l| !l.trim().is_empty());
    let entete = lignes.next().ok_or_else(|| Erreur::regle("RG-RMM-01", "Relevé vide"))?;
    let sep = [';', '\t', ','].into_iter().max_by_key(|s| entete.matches(*s).count()).unwrap_or(';');
    let entetes: Vec<String> = decouper(entete, sep).iter().map(|e| normaliser_entete(e)).collect();
    let i_ref = colonne(&entetes, &["reference", "ref", "id transaction", "transaction id", "txn id", "id", "numero de transaction"]);
    let i_montant = colonne(&entetes, &["montant", "amount", "credit", "montant credit", "valeur"]);
    let i_date = colonne(&entetes, &["date", "date operation", "date heure", "datetime", "horodatage"]);
    let i_numero = colonne(&entetes, &["numero", "msisdn", "expediteur", "sender", "de", "from", "telephone", "payeur"]);
    let (Some(i_ref), Some(i_montant)) = (i_ref, i_montant) else {
        return Err(Erreur::regle("RG-RMM-01", format!("Colonnes « référence » et « montant » introuvables (colonnes lues : {})", entetes.join(", "))));
    };
    let mut v = Vec::new();
    for l in lignes {
        let c = decouper(l, sep);
        let champ = |i: usize| c.get(i).map(String::as_str).unwrap_or("");
        let reference = champ(i_ref).trim_matches('"').trim().to_string();
        let Some(montant) = lire_montant(champ(i_montant)) else { continue };
        if reference.is_empty() || montant <= 0 {
            continue;
        }
        v.push(LigneReleve {
            reference,
            montant,
            date: i_date.and_then(|i| lire_date(champ(i))),
            numero: i_numero.map(|i| crate::zones_risque::normaliser_telephone(champ(i))).unwrap_or_default(),
        });
    }
    Ok(v)
}

#[derive(Debug, Serialize)]
pub struct Ecart {
    pub reference: String,
    pub montant_releve: i64,
    pub montant_caisse: i64,
    pub part_id: String,
    pub commande_numero: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct Bilan {
    pub releve_id: String,
    pub lignes_lues: usize,
    /// Références déjà importées (relevé chargé deux fois) : ignorées.
    pub deja_importees: usize,
    /// Paiements vérifiés par ce rapprochement.
    pub verifies: usize,
    /// Déjà vérifiés auparavant (à la main ou par un relevé précédent).
    pub deja_verifies: usize,
    /// Même référence, montant différent : à examiner.
    pub ecarts: Vec<Ecart>,
    /// Au relevé mais pas en caisse : paiement reçu non saisi, ou autre encaissement.
    pub inconnues: Vec<LigneReleve>,
    /// En caisse, « à vérifier », mais absents du relevé sur sa période : SMS douteux ?
    pub absents: Vec<crate::caisse::PartMobileMoney>,
}

fn statut_part(op: &Connection, part_id: &str) -> Resultat<String> {
    Ok(op
        .query_row(
            "SELECT statut FROM verifications_mm WHERE part_id = ?1 ORDER BY horodatage DESC, rowid DESC LIMIT 1",
            params![part_id],
            |r| r.get(0),
        )
        .optional()?
        .unwrap_or_else(|| "a_verifier".into()))
}

/// RG-RMM-02/03 : rapproche les lignes d'un relevé des paiements saisis (même compte, même référence).
fn rapprocher(op: &Op, compte_id: &str, releve_id: &str) -> Resultat<(usize, usize, Vec<Ecart>, Vec<LigneReleve>)> {
    let lignes: Vec<LigneReleve> = op
        .prepare("SELECT reference, montant, date_operation, numero FROM lignes_releve_mm WHERE releve_id = ?1 ORDER BY date_operation")?
        .query_map(params![releve_id], |r| Ok(LigneReleve { reference: r.get(0)?, montant: r.get(1)?, date: r.get(2)?, numero: r.get(3)? }))?
        .collect::<Result<_, _>>()?;
    let (mut verifies, mut deja) = (0, 0);
    let (mut ecarts, mut inconnues) = (Vec::new(), Vec::new());
    for l in lignes {
        let part: Option<(String, i64, Option<i64>)> = op
            .query_row(
                "SELECT pp.id, pp.montant, cm.numero FROM parts_paiement pp JOIN paiements p ON p.id = pp.paiement_id
                 LEFT JOIN commandes cm ON cm.id = p.commande_id
                 WHERE pp.moyen = 'mobile_money' AND pp.montant > 0 AND pp.compte_id = ?1 AND upper(trim(pp.reference)) = upper(?2)",
                params![compte_id, l.reference],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?;
        match part {
            None => inconnues.push(l),
            Some((id, montant, numero)) if montant != l.montant => {
                ecarts.push(Ecart { reference: l.reference, montant_releve: l.montant, montant_caisse: montant, part_id: id, commande_numero: numero })
            }
            Some((id, _, _)) => {
                if statut_part(op, &id)? == "verifie" {
                    deja += 1;
                } else {
                    op.execute(
                        "INSERT INTO verifications_mm(id, part_id, statut, note, horodatage, utilisateur_id) VALUES (?1, ?2, 'verifie', ?3, ?4, ?5)",
                        params![op.nouvel_id(), id, format!("Relevé opérateur ({})", l.reference), op.maintenant, op.utilisateur()],
                    )?;
                    verifies += 1;
                }
            }
        }
    }
    Ok((verifies, deja, ecarts, inconnues))
}

fn absents(conn: &Connection, compte_id: &str, debut: Option<i64>, fin: Option<i64>) -> Resultat<Vec<crate::caisse::PartMobileMoney>> {
    let nom: String = conn.query_row("SELECT nom FROM comptes_tresorerie WHERE id = ?1", params![compte_id], |r| r.get(0))?;
    Ok(crate::caisse::parts_mobile_money(conn, Some("a_verifier"))?
        .into_iter()
        .filter(|p| p.compte == nom)
        // Période du relevé (à la journée près) quand ses dates sont lisibles.
        .filter(|p| debut.is_none_or(|d| p.horodatage >= d - 86_400_000) && fin.is_none_or(|f| p.horodatage <= f + 86_400_000))
        // Présents au relevé (même avec un écart de montant) : pas « absents ».
        .filter(|p| {
            let r = p.reference.as_deref().unwrap_or("").trim().to_uppercase();
            r.is_empty()
                || conn
                    .query_row(
                        "SELECT COUNT(*) FROM lignes_releve_mm WHERE compte_id = ?1 AND upper(reference) = ?2",
                        params![compte_id, r],
                        |x| x.get::<_, i64>(0),
                    )
                    .map(|n| n == 0)
                    .unwrap_or(true)
        })
        .collect())
}

/// RG-RMM-01 à 04 : import d'un relevé et rapprochement immédiat.
pub fn importer(db: &mut Db, acteur: &Acteur, compte_id: &str, nom_fichier: &str, texte: &str) -> Resultat<Bilan> {
    let lues = lire_csv(texte)?;
    db.executer(acteur, |op| {
        let autorise = op.exiger(perm::CAISSE_VERIFIER_MM)?;
        let type_: String = trouver(op.query_row("SELECT type FROM comptes_tresorerie WHERE id = ?1", params![compte_id], |r| r.get(0)), "Compte")?;
        if type_ != "mobile_money" {
            return Err(Erreur::regle("RG-RMM-01", "Choisissez un compte Mobile Money"));
        }
        let debut = lues.iter().filter_map(|l| l.date).min();
        let fin = lues.iter().filter_map(|l| l.date).max();
        let id = op.nouvel_id();
        op.execute(
            "INSERT INTO releves_mm(id, compte_id, nom_fichier, debut, fin, importe_le, utilisateur_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![id, compte_id, nom_fichier.trim(), debut, fin, op.maintenant, op.utilisateur()],
        )?;
        // RG-RMM-04 : une référence n'est importée qu'une fois (relevés qui se chevauchent).
        let mut deja_importees = 0;
        for l in &lues {
            let n = op.execute(
                "INSERT OR IGNORE INTO lignes_releve_mm(id, releve_id, compte_id, reference, montant, date_operation, numero)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![op.nouvel_id(), id, compte_id, l.reference, l.montant, l.date, l.numero],
            )?;
            if n == 0 {
                deja_importees += 1;
            }
        }
        let (verifies, deja_verifies, ecarts, inconnues) = rapprocher(op, compte_id, &id)?;
        let absents = absents(op, compte_id, debut, fin)?;
        op.audit(
            "mobile_money.releve",
            "compte",
            Some(compte_id),
            None,
            Some(json!({ "lignes": lues.len(), "verifies": verifies, "ecarts": ecarts.len(), "inconnues": inconnues.len(), "absents": absents.len() })),
            None,
            autorise.as_deref(),
        )?;
        op.outbox("releve_mm", &id, "creer")?;
        op.evenement("paiement", None);
        Ok(Bilan { releve_id: id, lignes_lues: lues.len(), deja_importees, verifies, deja_verifies, ecarts, inconnues, absents })
    })
}

/// RG-RMM-05 : relancer le rapprochement de tous les relevés d'un compte (paiement saisi après l'import).
pub fn relancer(db: &mut Db, acteur: &Acteur, compte_id: &str) -> Resultat<usize> {
    db.executer(acteur, |op| {
        op.exiger(perm::CAISSE_VERIFIER_MM)?;
        let releves: Vec<String> = op
            .prepare("SELECT id FROM releves_mm WHERE compte_id = ?1 ORDER BY importe_le")?
            .query_map(params![compte_id], |r| r.get(0))?
            .collect::<Result<_, _>>()?;
        let mut n = 0;
        for r in releves {
            n += rapprocher(op, compte_id, &r)?.0;
        }
        op.evenement("paiement", None);
        Ok(n)
    })
}

#[derive(Debug, Serialize)]
pub struct ReleveLu {
    pub id: String,
    pub compte: String,
    pub nom_fichier: String,
    pub debut: Option<i64>,
    pub fin: Option<i64>,
    pub importe_le: i64,
    pub lignes: i64,
}

pub fn lister(conn: &Connection) -> Resultat<Vec<ReleveLu>> {
    let mut s = conn.prepare(
        "SELECT r.id, c.nom, r.nom_fichier, r.debut, r.fin, r.importe_le, (SELECT COUNT(*) FROM lignes_releve_mm l WHERE l.releve_id = r.id)
         FROM releves_mm r JOIN comptes_tresorerie c ON c.id = r.compte_id ORDER BY r.importe_le DESC LIMIT 100",
    )?;
    let v = s
        .query_map([], |r| {
            Ok(ReleveLu { id: r.get(0)?, compte: r.get(1)?, nom_fichier: r.get(2)?, debut: r.get(3)?, fin: r.get(4)?, importe_le: r.get(5)?, lignes: r.get(6)? })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn montants_dates_et_colonnes() {
        assert_eq!(lire_montant("12 500"), Some(12_500));
        assert_eq!(lire_montant("12500,00"), Some(12_500));
        assert_eq!(lire_montant("12.500"), Some(12_500));
        assert_eq!(lire_montant("\"4 000 FCFA\""), Some(4_000));
        assert_eq!(lire_montant("-1500"), Some(-1_500));
        assert_eq!(lire_montant("abc"), None);
        assert_eq!(lire_date("14/03/2026 18:30"), Some(chrono::DateTime::parse_from_rfc3339("2026-03-14T18:30:00Z").unwrap().timestamp_millis()));
        assert!(lire_date("2026-03-14 18:30:05").is_some());
        assert!(lire_date("n'importe").is_none());
        let csv = "\u{feff}Date;Référence;Expéditeur;Montant;Solde\n\
                   14/03/2026 18:30;PP260314.1830.A1;+223 70 11 22 33;4 000;10 000\n\
                   14/03/2026 19:00;PP260314.1900.B2;76001122;\"12 500,00\";22 500\n\
                   14/03/2026 19:05;RETRAIT1;;-5 000;17 500\n";
        let l = lire_csv(csv).unwrap();
        assert_eq!(l.len(), 2, "le débit est ignoré");
        assert_eq!((l[0].reference.as_str(), l[0].montant, l[0].numero.as_str()), ("PP260314.1830.A1", 4_000, "70112233"));
        assert_eq!(l[1].montant, 12_500);
        // Format anglais à virgules (Wave, par exemple).
        let l = lire_csv("Transaction ID,Amount,Date,From\nT1,2500,2026-03-14 12:00:00,76000001\n").unwrap();
        assert_eq!((l[0].reference.as_str(), l[0].montant), ("T1", 2_500));
        assert_eq!(lire_csv("a;b\n1;2").unwrap_err().regle_code(), Some("RG-RMM-01"));
    }
}
