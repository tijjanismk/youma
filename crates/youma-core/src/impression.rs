//! Tickets 80 mm : rendu texte (balisage minimal) puis conversion ESC/POS.
//! La file d'impression est en base : un envoi n'est jamais perdu si l'imprimante est en panne.

use rusqlite::{params, Connection};
use serde::Serialize;

use crate::db::{trouver, Acteur, Db, Op};
use crate::erreur::{Erreur, Resultat};
use crate::horloge::format_ms;
use crate::permissions as perm;

pub struct LigneTicket {
    pub quantite: i64,
    pub libelle: String,
    pub options: Vec<String>,
    pub commentaire: String,
}

pub struct TicketEnvoi {
    pub titre: String,
    pub numero_envoi: i64,
    pub heure: i64,
    pub serveur: String,
    pub lignes: Vec<LigneTicket>,
    pub annulation: bool,
}

/// Balises en début de ligne : `##` gros caractères, `**` gras, `--` trait, `>>` centré.
pub fn rendu_envoi(t: &TicketEnvoi, largeur: usize, fuseau_minutes: i64) -> String {
    let mut s = String::new();
    if t.annulation {
        s.push_str("##*** ANNULATION ***\n");
    }
    let heure = format_ms(t.heure + fuseau_minutes * 60_000);
    let heure = heure.split(' ').nth(1).unwrap_or(&heure);
    s.push_str(&format!("##{}\n", t.titre));
    if t.numero_envoi > 0 {
        s.push_str(&format!("**Envoi n°{} — {}\n", t.numero_envoi, heure));
    } else {
        s.push_str(&format!("**{heure}\n"));
    }
    if !t.serveur.is_empty() {
        s.push_str(&format!("Serveur : {}\n", t.serveur));
    }
    s.push_str("--\n");
    for l in &t.lignes {
        s.push_str(&format!("##{} × {}\n", l.quantite, l.libelle));
        for o in &l.options {
            s.push_str(&format!("   + {o}\n"));
        }
        if !l.commentaire.is_empty() {
            for morceau in couper(&format!("Note : « {} »", l.commentaire), largeur.saturating_sub(3)) {
                s.push_str(&format!("   {morceau}\n"));
            }
        }
    }
    s.push_str("--\n");
    s
}

pub fn couper(texte: &str, largeur: usize) -> Vec<String> {
    let mut lignes = Vec::new();
    let mut courante = String::new();
    for mot in texte.split_whitespace() {
        if !courante.is_empty() && courante.chars().count() + 1 + mot.chars().count() > largeur {
            lignes.push(std::mem::take(&mut courante));
        }
        if !courante.is_empty() {
            courante.push(' ');
        }
        courante.push_str(mot);
    }
    if !courante.is_empty() {
        lignes.push(courante);
    }
    lignes
}

/// Ligne « libellé ....... montant » sur la largeur du ticket.
pub fn ligne_montant(libelle: &str, montant: &str, largeur: usize) -> String {
    let l = libelle.chars().count();
    let m = montant.chars().count();
    if l + m + 1 > largeur {
        let tronque: String = libelle.chars().take(largeur.saturating_sub(m + 1)).collect();
        return format!("{tronque} {montant}");
    }
    format!("{libelle}{}{montant}", " ".repeat(largeur - l - m))
}

/// 12500 → « 12 500 ».
pub fn fcfa(montant: i64) -> String {
    let signe = if montant < 0 { "-" } else { "" };
    let chiffres = montant.abs().to_string();
    let mut out = String::new();
    for (i, c) in chiffres.chars().enumerate() {
        if i > 0 && (chiffres.len() - i) % 3 == 0 {
            out.push(' ');
        }
        out.push(c);
    }
    format!("{signe}{out}")
}

/// Conversion en octets ESC/POS (page de code PC858, accents français).
pub fn en_escpos(texte: &str, ouvrir_tiroir: bool) -> Vec<u8> {
    let mut b: Vec<u8> = vec![0x1b, b'@', 0x1b, b't', 19];
    if ouvrir_tiroir {
        b.extend_from_slice(&[0x1b, b'p', 0, 25, 250]);
    }
    for ligne in texte.lines() {
        let (style, contenu) = if let Some(r) = ligne.strip_prefix("##") {
            ("gros", r)
        } else if let Some(r) = ligne.strip_prefix("**") {
            ("gras", r)
        } else if let Some(r) = ligne.strip_prefix(">>") {
            ("centre", r)
        } else if ligne.starts_with("--") {
            ("trait", "")
        } else {
            ("normal", ligne)
        };
        match style {
            "gros" => b.extend_from_slice(&[0x1d, b'!', 0x11, 0x1b, b'E', 1]),
            "gras" => b.extend_from_slice(&[0x1b, b'E', 1]),
            "centre" => b.extend_from_slice(&[0x1b, b'a', 1]),
            _ => {}
        }
        if style == "trait" {
            b.extend(std::iter::repeat_n(b'-', 42));
        } else {
            b.extend(contenu.chars().map(vers_pc858));
        }
        b.push(b'\n');
        match style {
            "gros" => b.extend_from_slice(&[0x1d, b'!', 0x00, 0x1b, b'E', 0]),
            "gras" => b.extend_from_slice(&[0x1b, b'E', 0]),
            "centre" => b.extend_from_slice(&[0x1b, b'a', 0]),
            _ => {}
        }
    }
    b.extend_from_slice(b"\n\n\n");
    b.extend_from_slice(&[0x1d, b'V', 66, 0]); // coupe partielle
    b
}

fn vers_pc858(c: char) -> u8 {
    match c {
        'é' => 0x82,
        'è' => 0x8a,
        'ê' => 0x88,
        'ë' => 0x89,
        'à' => 0x85,
        'â' => 0x83,
        'ç' => 0x87,
        'ù' => 0x97,
        'û' => 0x96,
        'ô' => 0x93,
        'î' => 0x8c,
        'ï' => 0x8b,
        'É' => 0x90,
        'À' => 0xb7,
        'Ç' => 0x80,
        'È' => 0xd4,
        '°' => 0xf8,
        '×' => b'x',
        '«' => 0xae,
        '»' => 0xaf,
        '—' | '–' => b'-',
        '’' => b'\'',
        '\u{a0}' | '\u{202f}' => b' ',
        c if c.is_ascii() => c as u8,
        _ => b'?',
    }
}

/// Retire les balises pour l'affichage à l'écran.
pub fn texte_brut(texte: &str) -> String {
    texte
        .lines()
        .map(|l| {
            if l.starts_with("--") {
                "-".repeat(42)
            } else {
                l.trim_start_matches("##").trim_start_matches("**").trim_start_matches(">>").to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

// ───────────── File d'impression ─────────────

/// Met un ticket en file pour un poste (ou la destination de caisse si `poste` = None).
pub(crate) fn mettre_en_file(op: &mut Op, poste: Option<&str>, type_: &str, reference: Option<&str>, contenu: &str) -> Resultat<Option<String>> {
    let destination: String = match poste {
        Some(p) => op.query_row("SELECT imprimante FROM postes_preparation WHERE id = ?1", params![p], |r| r.get(0))?,
        None => op.params.imprimante_caisse.clone(),
    };
    if destination.trim().is_empty() {
        // Poste sans imprimante (écran seul) : rien à imprimer.
        return Ok(None);
    }
    let id = op.nouvel_id();
    op.execute(
        "INSERT INTO impressions(id, poste_id, destination, type, reference_id, contenu, statut, cree_le, modifie_le)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'en_attente', ?7, ?7)",
        params![id, poste, destination, type_, reference, contenu, op.maintenant],
    )?;
    op.evenement("impression", Some(&id));
    Ok(Some(id))
}

#[derive(Debug, Serialize, Clone)]
pub struct Job {
    pub id: String,
    pub poste_id: Option<String>,
    pub poste_nom: Option<String>,
    pub destination: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub contenu: String,
    pub statut: String,
    pub tentatives: i64,
    pub erreur: Option<String>,
    pub cree_le: i64,
}

pub fn jobs(conn: &Connection, statuts: &[&str]) -> Resultat<Vec<Job>> {
    let mut s = conn.prepare(
        "SELECT i.id, i.poste_id, p.nom, i.destination, i.type, i.contenu, i.statut, i.tentatives, i.erreur, i.cree_le
         FROM impressions i LEFT JOIN postes_preparation p ON p.id = i.poste_id
         ORDER BY i.cree_le DESC LIMIT 200",
    )?;
    let v = s
        .query_map([], |r| {
            Ok(Job {
                id: r.get(0)?,
                poste_id: r.get(1)?,
                poste_nom: r.get(2)?,
                destination: r.get(3)?,
                type_: r.get(4)?,
                contenu: r.get(5)?,
                statut: r.get(6)?,
                tentatives: r.get(7)?,
                erreur: r.get(8)?,
                cree_le: r.get(9)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(v.into_iter().filter(|j| statuts.is_empty() || statuts.contains(&j.statut.as_str())).collect())
}

/// Jobs à imprimer par le serveur (en attente, ou en erreur avec moins de 5 tentatives).
pub fn jobs_a_imprimer(conn: &Connection) -> Resultat<Vec<Job>> {
    let mut v: Vec<Job> = jobs(conn, &["en_attente", "erreur"])?.into_iter().filter(|j| j.statut == "en_attente" || j.tentatives < 5).collect();
    v.reverse();
    Ok(v)
}

/// Résultat d'une tentative d'impression (hors transaction métier : état technique).
pub fn marquer_job(db: &Db, id: &str, erreur: Option<&str>) -> Resultat<()> {
    db.conn().execute(
        "UPDATE impressions SET statut = ?1, tentatives = tentatives + 1, erreur = ?2, modifie_le = ?3 WHERE id = ?4",
        params![if erreur.is_some() { "erreur" } else { "imprime" }, erreur, db.maintenant(), id],
    )?;
    Ok(())
}

/// Réimpression, éventuellement vers une autre imprimante (scénario 15).
pub fn reimprimer(db: &mut Db, acteur: &Acteur, id: &str, destination: Option<&str>) -> Resultat<String> {
    db.executer(acteur, |op| {
        op.exiger(perm::CUISINE_VOIR)?;
        let (poste, dest, type_, reference, contenu): (Option<String>, String, String, Option<String>, String) = trouver(
            op.query_row(
                "SELECT poste_id, destination, type, reference_id, contenu FROM impressions WHERE id = ?1",
                params![id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
            ),
            "Impression",
        )?;
        let dest = destination.map(str::to_owned).filter(|d| !d.trim().is_empty()).unwrap_or(dest);
        if dest.is_empty() {
            return Err(Erreur::validation("Aucune imprimante de destination"));
        }
        let nid = op.nouvel_id();
        let contenu = if contenu.starts_with("##*** COPIE") { contenu } else { format!("##*** COPIE ***\n{contenu}") };
        op.execute(
            "INSERT INTO impressions(id, poste_id, destination, type, reference_id, contenu, statut, cree_le, modifie_le)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'en_attente', ?7, ?7)",
            params![nid, poste, dest, type_, reference, contenu, op.maintenant],
        )?;
        // L'original en erreur n'est plus retenté : la copie prend le relais.
        op.execute("UPDATE impressions SET tentatives = 99 WHERE id = ?1 AND statut = 'erreur'", params![id])?;
        op.evenement("impression", Some(&nid));
        Ok(nid)
    })
}

/// Ticket client (addition ou reçu) mis en file sur l'imprimante de caisse.
pub fn imprimer_ticket_client(db: &mut Db, acteur: &Acteur, commande_id: &str) -> Resultat<Option<String>> {
    db.executer(acteur, |op| {
        op.exiger(perm::COMMANDE_CREER)?;
        let texte = ticket_client(op, commande_id)?;
        mettre_en_file(op, None, "ticket_client", Some(commande_id), &texte)
    })
}

/// Texte de l'addition / du reçu.
pub fn ticket_client(conn: &Connection, commande_id: &str) -> Resultat<String> {
    let p = crate::parametres::lire(conn)?;
    let w = p.largeur_ticket;
    let (nom, adresse, tel, pied): (String, String, String, String) =
        conn.query_row("SELECT nom, adresse, telephone, pied_ticket FROM restaurant LIMIT 1", [], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
        })?;
    let c = crate::commandes::detail(conn, commande_id)?;
    let mut s = format!("##{nom}\n");
    if !adresse.is_empty() {
        s.push_str(&format!(">>{adresse}\n"));
    }
    if !tel.is_empty() {
        s.push_str(&format!(">>Tél. {tel}\n"));
    }
    s.push_str("--\n");
    let titre = match &c.table_nom {
        Some(t) => format!("Table {t}"),
        None => c.type_.replace('_', " "),
    };
    s.push_str(&format!("**Ticket n°{} — {titre}\n", c.numero));
    s.push_str(&format!("{}\n", format_ms(c.cree_le + p.fuseau_minutes * 60_000)));
    if let Some(serv) = &c.serveur_nom {
        s.push_str(&format!("Servi par : {serv}\n"));
    }
    s.push_str("--\n");
    for l in c.lignes.iter().filter(|l| l.quantite > l.quantite_annulee) {
        let q = l.quantite - l.quantite_annulee;
        let montant = if l.offert { "offert".to_string() } else { fcfa(l.montant) };
        s.push_str(&ligne_montant(&format!("{q} × {}", l.libelle), &montant, w));
        s.push('\n');
    }
    s.push_str("--\n");
    if c.totaux.remises != 0 {
        s.push_str(&ligne_montant("Remises", &format!("-{}", fcfa(c.totaux.remises)), w));
        s.push('\n');
    }
    if c.totaux.frais_livraison != 0 {
        s.push_str(&ligne_montant("Livraison", &fcfa(c.totaux.frais_livraison), w));
        s.push('\n');
    }
    s.push_str(&format!("##{}\n", ligne_montant("TOTAL", &format!("{} F", fcfa(c.totaux.total)), w / 2)));
    if c.totaux.paye > 0 {
        s.push_str(&ligne_montant("Payé", &fcfa(c.totaux.paye), w));
        s.push('\n');
        for p in crate::caisse::paiements_commande(conn, commande_id)? {
            for (moyen, m, compte) in p.parts {
                let lib = match moyen.as_str() {
                    "especes" => "  Espèces".to_string(),
                    "credit" => "  Crédit (ardoise)".to_string(),
                    _ => format!("  {}", compte.unwrap_or(moyen)),
                };
                s.push_str(&ligne_montant(&lib, &fcfa(m), w));
                s.push('\n');
            }
            if p.rendu > 0 {
                s.push_str(&ligne_montant("  Rendu", &fcfa(p.rendu), w));
                s.push('\n');
            }
        }
    }
    if c.totaux.reste > 0 {
        s.push_str(&format!("**{}\n", ligne_montant("Reste à payer", &fcfa(c.totaux.reste), w)));
    }
    s.push_str("--\n");
    s.push_str(&format!(">>{pied}\n"));
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_fcfa() {
        assert_eq!(fcfa(12_500), "12 500");
        assert_eq!(fcfa(750), "750");
        assert_eq!(fcfa(1_000_000), "1 000 000");
        assert_eq!(fcfa(-3_000), "-3 000");
    }

    #[test]
    fn ticket_cuisine_lisible() {
        let t = TicketEnvoi {
            titre: "TABLE 12".into(),
            numero_envoi: 2,
            heure: crate::horloge::ms_de("2026-03-14", 20, 14),
            serveur: "Awa".into(),
            lignes: vec![
                LigneTicket { quantite: 2, libelle: "Poulet braisé".into(), options: vec![], commentaire: "bien cuit".into() },
                LigneTicket { quantite: 1, libelle: "Riz sauce arachide".into(), options: vec![], commentaire: String::new() },
            ],
            annulation: false,
        };
        let r = rendu_envoi(&t, 42, 0);
        assert!(r.contains("##TABLE 12"));
        assert!(r.contains("Envoi n°2 — 20:14"));
        assert!(r.contains("Serveur : Awa"));
        assert!(r.contains("##2 × Poulet braisé"));
        assert!(r.contains("Note : « bien cuit »"));
        let octets = en_escpos(&r, false);
        assert_eq!(&octets[..2], &[0x1b, b'@']);
        assert!(octets.windows(4).any(|w| w == [0x1d, b'V', 66, 0]));
        // « é » en PC858
        assert!(octets.contains(&0x82));
    }

    #[test]
    fn alignement_montants() {
        assert_eq!(ligne_montant("TOTAL", "12 500", 20), "TOTAL         12 500");
    }
}
