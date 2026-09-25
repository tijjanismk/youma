//! Cloud facultatif (fiche 0018, RG-CLO-01 à 05) : ce que le poste central prépare pour le serveur Internet.
//! Aucune fonction ici ne touche au réseau : le poste reste autonome, l'envoi est fait par le serveur local.

use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{Key, XChaCha20Poly1305, XNonce};
use rand::RngCore;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::db::{Acteur, Db};
use crate::erreur::{Erreur, Resultat};
use crate::permissions as perm;

const MAGIE: &[u8; 6] = b"YOUMA1";

fn cle_depuis_phrase(phrase: &str, sel: &[u8]) -> Resultat<[u8; 32]> {
    let mut cle = [0u8; 32];
    argon2::Argon2::default()
        .hash_password_into(phrase.as_bytes(), sel, &mut cle)
        .map_err(|e| Erreur::validation(format!("dérivation de clé impossible : {e}")))?;
    Ok(cle)
}

/// RG-CLO-02 : sauvegarde chiffrée avant de quitter le restaurant (XChaCha20-Poly1305, clé dérivée de la phrase
/// par Argon2id). Format : « YOUMA1 » + sel (16) + nonce (24) + chiffré. Le cloud ne peut pas la lire.
pub fn chiffrer(donnees: &[u8], phrase: &str) -> Resultat<Vec<u8>> {
    if phrase.chars().count() < 12 {
        return Err(Erreur::regle("RG-CLO-02", "Phrase de chiffrement : 12 caractères au moins"));
    }
    let mut sel = [0u8; 16];
    let mut nonce = [0u8; 24];
    rand::thread_rng().fill_bytes(&mut sel);
    rand::thread_rng().fill_bytes(&mut nonce);
    let cle = cle_depuis_phrase(phrase, &sel)?;
    let chiffre = XChaCha20Poly1305::new(Key::from_slice(&cle))
        .encrypt(XNonce::from_slice(&nonce), donnees)
        .map_err(|_| Erreur::validation("chiffrement impossible"))?;
    let mut v = Vec::with_capacity(MAGIE.len() + 40 + chiffre.len());
    v.extend_from_slice(MAGIE);
    v.extend_from_slice(&sel);
    v.extend_from_slice(&nonce);
    v.extend_from_slice(&chiffre);
    Ok(v)
}

/// Déchiffre une sauvegarde distante ; une mauvaise phrase ou un fichier altéré est refusé.
pub fn dechiffrer(donnees: &[u8], phrase: &str) -> Resultat<Vec<u8>> {
    if donnees.len() < MAGIE.len() + 40 || &donnees[..MAGIE.len()] != MAGIE {
        return Err(Erreur::regle("RG-CLO-02", "Ce fichier n'est pas une sauvegarde Youma chiffrée"));
    }
    let sel = &donnees[6..22];
    let nonce = &donnees[22..46];
    let cle = cle_depuis_phrase(phrase, sel)?;
    XChaCha20Poly1305::new(Key::from_slice(&cle))
        .decrypt(XNonce::from_slice(nonce), &donnees[46..])
        .map_err(|_| Erreur::regle("RG-CLO-02", "Phrase de chiffrement incorrecte ou fichier altéré"))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResumeJournee {
    /// Date d'exploitation AAAA-MM-JJ.
    pub date: String,
    pub cloturee: bool,
    pub chiffre_affaires: i64,
    pub commandes: i64,
    pub depenses: i64,
    /// (moyen, montant encaissé).
    pub encaissements: Vec<(String, i64)>,
    /// (nombre, montant) des paiements Mobile Money encore à vérifier.
    pub mobile_money_a_verifier: (i64, i64),
    /// (nombre, montant) des annulations après envoi.
    pub annulations: (i64, i64),
    /// Σ écarts de caisse des sessions clôturées ce jour.
    pub ecarts_caisse: i64,
    pub mis_a_jour: i64,
}

/// RG-CLO-01 : résumé d'une journée d'exploitation, seules données d'activité envoyées au cloud.
pub fn resume_journee(conn: &Connection, date: &str, maintenant: i64) -> Resultat<Option<ResumeJournee>> {
    let j: Option<(String, String)> = conn
        .query_row("SELECT id, statut FROM journees WHERE date_exploitation = ?1", params![date], |r| Ok((r.get(0)?, r.get(1)?)))
        .optional()?;
    let Some((jid, statut)) = j else { return Ok(None) };
    let c = crate::rapports::chiffres(conn, date, date)?;
    let encaissements: Vec<(String, i64)> = conn
        .prepare(
            "SELECT pp.moyen, SUM(pp.montant) FROM parts_paiement pp JOIN paiements p ON p.id = pp.paiement_id
             WHERE p.journee_id = ?1 GROUP BY pp.moyen HAVING SUM(pp.montant) <> 0 ORDER BY 2 DESC",
        )?
        .query_map(params![jid], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<Result<_, _>>()?;
    let params = crate::parametres::lire(conn)?;
    let mm = crate::caisse::parts_mobile_money(conn, Some("a_verifier"))?;
    let mm_jour: Vec<_> = mm
        .iter()
        .filter(|p| crate::horloge::date_exploitation(p.horodatage, params.fuseau_minutes, params.heure_bascule) == date)
        .collect();
    let annulations: (i64, i64) = conn.query_row(
        "SELECT COUNT(*), COALESCE(SUM(a.montant), 0) FROM annulations a JOIN commandes c ON c.id = a.commande_id WHERE c.journee_id = ?1",
        params![jid],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    let ecarts: i64 = conn.query_row(
        "SELECT COALESCE(SUM(ecart), 0) FROM sessions_caisse WHERE journee_id = ?1 AND statut = 'cloturee'",
        params![jid],
        |r| r.get(0),
    )?;
    Ok(Some(ResumeJournee {
        date: date.into(),
        cloturee: statut == "cloturee",
        chiffre_affaires: c.ca,
        commandes: c.nb_commandes,
        depenses: c.depenses,
        encaissements,
        mobile_money_a_verifier: (mm_jour.len() as i64, mm_jour.iter().map(|p| p.montant).sum()),
        annulations,
        ecarts_caisse: ecarts,
        mis_a_jour: maintenant,
    }))
}

/// Résumés des journées des `jours` derniers jours d'exploitation (renvoyés à chaque synchronisation : idempotent).
pub fn resumes_recents(conn: &Connection, jours: i64, maintenant: i64) -> Resultat<Vec<ResumeJournee>> {
    let dates: Vec<String> = conn
        .prepare("SELECT date_exploitation FROM journees ORDER BY date_exploitation DESC LIMIT ?1")?
        .query_map(params![jours], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    let mut v = Vec::new();
    for d in dates {
        if let Some(r) = resume_journee(conn, &d, maintenant)? {
            v.push(r);
        }
    }
    Ok(v)
}

/// RG-CLO-04 : texte du résumé de fin de journée (SMS, 1 à 2 messages).
pub fn texte_sms(restaurant: &str, r: &ResumeJournee) -> String {
    let f = crate::impression::fcfa;
    let date = format!("{}/{}", &r.date[8..10], &r.date[5..7]);
    let moyens: Vec<String> = r.encaissements.iter().map(|(m, x)| format!("{} {}", libelle_moyen(m), f(*x))).collect();
    let mut t = format!("{restaurant} {date} : CA {} F, {} cmd", f(r.chiffre_affaires), r.commandes);
    if !moyens.is_empty() {
        t.push_str(&format!(" ({})", moyens.join(", ")));
    }
    if r.depenses != 0 {
        t.push_str(&format!(". Dépenses {} F", f(r.depenses)));
    }
    if r.mobile_money_a_verifier.0 > 0 {
        t.push_str(&format!(". MM à vérifier : {} ({} F)", r.mobile_money_a_verifier.0, f(r.mobile_money_a_verifier.1)));
    }
    if r.annulations.0 > 0 {
        t.push_str(&format!(". Annulations : {} ({} F)", r.annulations.0, f(r.annulations.1)));
    }
    if r.ecarts_caisse != 0 {
        t.push_str(&format!(". Écart caisse {} F", f(r.ecarts_caisse)));
    }
    t
}

fn libelle_moyen(m: &str) -> &str {
    match m {
        "especes" => "espèces",
        "mobile_money" => "MM",
        "credit" => "crédit",
        autre => autre,
    }
}

/// RG-CLO-03 : mot de passe de l'espace propriétaire. Seule son empreinte Argon2 est gardée et envoyée.
pub fn definir_mot_de_passe(db: &mut Db, acteur: &Acteur, mot_de_passe: &str) -> Resultat<()> {
    if mot_de_passe.chars().count() < 8 {
        return Err(Erreur::regle("RG-CLO-03", "Mot de passe de l'espace propriétaire : 8 caractères au moins"));
    }
    let hash = crate::auth::hacher(mot_de_passe)?;
    db.executer(acteur, |op| {
        op.exiger(perm::PARAMETRE_GERER)?;
        let mut p = op.params.clone();
        p.cloud.mdp_hash = hash.clone();
        crate::parametres::ecrire(op, &p)?;
        op.audit("cloud.mot_de_passe", "parametres", None, None, None, None, None)?;
        Ok(())
    })
}

/// État local de la dernière sauvegarde envoyée (évite de renvoyer la même).
pub fn derniere_sauvegarde_envoyee(conn: &Connection) -> Resultat<Option<String>> {
    Ok(conn.query_row("SELECT valeur FROM systeme WHERE cle = 'cloud_derniere_sauvegarde'", [], |r| r.get(0)).optional()?)
}

pub fn marquer_sauvegarde_envoyee(conn: &Connection, nom: &str) -> Resultat<()> {
    conn.execute("INSERT OR REPLACE INTO systeme(cle, valeur) VALUES ('cloud_derniere_sauvegarde', ?1)", params![nom])?;
    Ok(())
}

/// Données envoyées à chaque synchronisation (hors sauvegardes).
pub fn charge_synchronisation(conn: &Connection, maintenant: i64) -> Resultat<serde_json::Value> {
    let p = crate::parametres::lire(conn)?;
    let r = crate::parametres::restaurant(conn)?;
    Ok(json!({
        "restaurant": r.nom,
        "telephone_proprietaire": crate::zones_risque::normaliser_telephone(&p.cloud.telephone_proprietaire),
        "mdp_hash": p.cloud.mdp_hash,
        "sms_resume": p.cloud.sms_resume,
        "resumes": resumes_recents(conn, 7, maintenant)?,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chiffrement_aller_retour() {
        let phrase = "baobab du marché 2026";
        let clair = b"SQLite format 3\0 donnees du restaurant".to_vec();
        let c = chiffrer(&clair, phrase).unwrap();
        assert_eq!(&c[..6], b"YOUMA1");
        assert!(!c.windows(6).any(|w| w == b"SQLite"), "le contenu n'est pas lisible");
        assert_eq!(dechiffrer(&c, phrase).unwrap(), clair);
        assert!(dechiffrer(&c, "mauvaise phrase !!").is_err());
        let mut altere = c.clone();
        let n = altere.len() - 1;
        altere[n] ^= 1;
        assert!(dechiffrer(&altere, phrase).is_err());
        assert!(chiffrer(&clair, "court").is_err());
        // Deux chiffrements de la même donnée diffèrent (sel et nonce aléatoires).
        assert_ne!(chiffrer(&clair, phrase).unwrap(), c);
    }
}
