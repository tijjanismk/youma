//! Paie simple : le compte employé est un journal, le bulletin en est un instantané figé.

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::db::{trouver, Acteur, Db};
use crate::employes::{self, employe, inserer_mouvement, inserer_mouvement_date, presences, Employe};
use crate::erreur::{Erreur, Resultat};
use crate::parametres::{appliquer_bp, Parametres};
use crate::permissions as perm;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LigneBulletin {
    #[serde(rename = "type")]
    pub type_: String,
    pub libelle: String,
    pub montant: i64,
    pub quantite: Option<i64>,
}

#[derive(Debug, Serialize, Clone)]
pub struct Bulletin {
    pub id: Option<String>,
    pub numero: Option<i64>,
    pub employe_id: String,
    pub employe_nom: String,
    pub fonction: String,
    pub debut: String,
    pub fin: String,
    pub type_remuneration: String,
    pub type_contrat: String,
    pub base: i64,
    pub jours_presents: i64,
    pub jours_absence_nj: i64,
    pub report_precedent: i64,
    pub total_gains: i64,
    pub total_retenues: i64,
    pub cotisations_salarie: i64,
    pub charges_employeur: i64,
    pub deja_paye: i64,
    pub net_a_payer: i64,
    pub lignes: Vec<LigneBulletin>,
    /// Payé depuis la clôture (bulletins figés uniquement).
    pub paye_depuis: i64,
    pub reste_a_payer: i64,
}

/// Mouvements générés par la clôture (salaire, absences, cotisations).
struct Calcul {
    salaire: i64,
    deduction: i64,
    jours_presents: i64,
    jours_absence_nj: i64,
    inps_salarie: i64,
    amo_salarie: i64,
    charges_employeur: i64,
}

fn libelle_type(t: &str) -> &'static str {
    match t {
        "salaire" => "Salaire",
        "deduction_absence" => "Absences non justifiées",
        "prime" => "Prime",
        "tache" => "Tâches effectuées",
        "avance" => "Avance",
        "retenue" => "Retenue",
        "consommation" => "Consommations",
        "cotisation_inps" => "Cotisation INPS (salarié)",
        "cotisation_amo" => "Cotisation AMO (salarié)",
        "paiement" => "Déjà payé",
        "regularisation" => "Régularisation",
        _ => "Autre",
    }
}

fn valider_periode(debut: &str, fin: &str) -> Resultat<()> {
    let d = chrono::NaiveDate::parse_from_str(debut, "%Y-%m-%d").map_err(|_| Erreur::validation("Date de début invalide"))?;
    let f = chrono::NaiveDate::parse_from_str(fin, "%Y-%m-%d").map_err(|_| Erreur::validation("Date de fin invalide"))?;
    if f < d {
        return Err(Erreur::validation("La fin précède le début"));
    }
    Ok(())
}

/// RG-PAI-03 / RG-PAI-07 / RG-PAI-08.
fn calculer(conn: &Connection, e: &Employe, p: &Parametres, debut: &str, fin: &str, gains_variables: i64) -> Resultat<Calcul> {
    let pres = presences(conn, Some(&e.id), debut, fin)?;
    let jours_presents = pres.iter().filter(|p| p.statut == "present" || p.statut == "retard").count() as i64;
    let jours_absence_nj = pres.iter().filter(|p| p.statut == "absent_non_justifie").count() as i64;
    let (salaire, deduction) = match e.type_remuneration.as_str() {
        "mensuel" | "hebdomadaire" => {
            let jours_ref = if e.type_remuneration == "mensuel" { p.paie.jours_ouvrables_mois.max(1) } else { 6 };
            let d = if p.paie.deduire_absences {
                ((e.montant_base * jours_absence_nj + jours_ref / 2) / jours_ref).min(e.montant_base)
            } else {
                0
            };
            (e.montant_base, d)
        }
        "journalier" => (e.montant_base * jours_presents, 0),
        _ => (0, 0),
    };
    // Assiette des cotisations : brut de la période (salaire − absences + primes + tâches).
    let assiette = (salaire - deduction + gains_variables).max(0);
    let c = &p.cotisations;
    let inps = c.inps_active && e.declare_inps;
    let amo = c.amo_active && e.affilie_amo;
    let inps_salarie = if inps { appliquer_bp(assiette, c.inps_salarie_bp) } else { 0 };
    let amo_salarie = if amo { appliquer_bp(assiette, c.amo_salarie_bp) } else { 0 };
    let charges_employeur = if inps { appliquer_bp(assiette, c.inps_employeur_bp) } else { 0 }
        + if amo { appliquer_bp(assiette, c.amo_employeur_bp) } else { 0 };
    Ok(Calcul { salaire, deduction, jours_presents, jours_absence_nj, inps_salarie, amo_salarie, charges_employeur })
}

fn dernier_bulletin(conn: &Connection, employe_id: &str) -> Resultat<Option<(String, String, i64)>> {
    Ok(conn
        .query_row(
            "SELECT id, fin, dernier_seq FROM bulletins WHERE employe_id = ?1 ORDER BY dernier_seq DESC LIMIT 1",
            params![employe_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?)
}

struct Mvt {
    type_: String,
    montant: i64,
    quantite: Option<i64>,
    bulletin_id: Option<String>,
}

fn mouvements_apres(conn: &Connection, employe_id: &str, seq: i64) -> Resultat<Vec<Mvt>> {
    let mut s = conn.prepare(
        "SELECT type, montant, quantite, bulletin_id FROM mouvements_employe WHERE employe_id = ?1 AND seq > ?2 ORDER BY seq",
    )?;
    let v = s
        .query_map(params![employe_id, seq], |r| Ok(Mvt { type_: r.get(0)?, montant: r.get(1)?, quantite: r.get(2)?, bulletin_id: r.get(3)? }))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(v)
}

fn solde_jusqua(conn: &Connection, employe_id: &str, seq: i64) -> Resultat<i64> {
    Ok(conn.query_row(
        "SELECT COALESCE(SUM(montant), 0) FROM mouvements_employe WHERE employe_id = ?1 AND seq <= ?2",
        params![employe_id, seq],
        |r| r.get(0),
    )?)
}

/// Assemble le bulletin à partir des mouvements postérieurs au bulletin précédent.
fn assembler(conn: &Connection, e: &Employe, debut: &str, fin: &str, mvts: &[Mvt], prev_seq: i64, c: &Calcul) -> Resultat<Bulletin> {
    // RG-PAI-04 : report = solde au bulletin précédent, diminué des paiements rattachés aux anciens bulletins.
    let paiements_anciens: i64 = mvts.iter().filter(|m| m.type_ == "paiement" && m.bulletin_id.is_some()).map(|m| m.montant).sum();
    let report = solde_jusqua(conn, &e.id, prev_seq)? + paiements_anciens;
    let mut lignes: Vec<LigneBulletin> = Vec::new();
    let mut ajouter = |t: &str, m: i64, q: Option<i64>| {
        if m == 0 {
            return;
        }
        if let Some(l) = lignes.iter_mut().find(|l| l.type_ == t && (t != "regularisation" || l.montant.signum() == m.signum())) {
            l.montant += m;
            l.quantite = match (l.quantite, q) {
                (Some(a), Some(b)) => Some(a + b),
                (a, b) => a.or(b),
            };
        } else {
            lignes.push(LigneBulletin { type_: t.into(), libelle: libelle_type(t).into(), montant: m, quantite: q });
        }
    };
    for m in mvts.iter().filter(|m| !(m.type_ == "paiement" && m.bulletin_id.is_some())) {
        ajouter(&m.type_, m.montant, m.quantite);
    }
    let gains: i64 = lignes.iter().filter(|l| l.montant > 0).map(|l| l.montant).sum();
    let paye: i64 = -lignes.iter().filter(|l| l.type_ == "paiement").map(|l| l.montant).sum::<i64>();
    let cotis: i64 = -lignes.iter().filter(|l| l.type_.starts_with("cotisation")).map(|l| l.montant).sum::<i64>();
    let retenues: i64 = -lignes
        .iter()
        .filter(|l| l.montant < 0 && l.type_ != "paiement" && !l.type_.starts_with("cotisation"))
        .map(|l| l.montant)
        .sum::<i64>();
    let net = report + gains - retenues - cotis - paye;
    Ok(Bulletin {
        id: None,
        numero: None,
        employe_id: e.id.clone(),
        employe_nom: e.nom.clone(),
        fonction: e.fonction.clone(),
        debut: debut.into(),
        fin: fin.into(),
        type_remuneration: e.type_remuneration.clone(),
        type_contrat: e.type_contrat.clone(),
        base: e.montant_base,
        jours_presents: c.jours_presents,
        jours_absence_nj: c.jours_absence_nj,
        report_precedent: report,
        total_gains: gains,
        total_retenues: retenues,
        cotisations_salarie: cotis,
        charges_employeur: c.charges_employeur,
        deja_paye: paye,
        net_a_payer: net,
        lignes,
        paye_depuis: 0,
        reste_a_payer: net,
    })
}

fn gains_variables(mvts: &[Mvt]) -> i64 {
    mvts.iter().filter(|m| m.type_ == "prime" || m.type_ == "tache").map(|m| m.montant).sum()
}

/// Aperçu sans rien écrire.
pub fn apercu(conn: &Connection, employe_id: &str, debut: &str, fin: &str) -> Resultat<Bulletin> {
    valider_periode(debut, fin)?;
    let e = employe(conn, employe_id)?;
    let p = crate::parametres::lire(conn)?;
    let prev = dernier_bulletin(conn, employe_id)?.map(|b| b.2).unwrap_or(0);
    let mut mvts = mouvements_apres(conn, employe_id, prev)?;
    let c = calculer(conn, &e, &p, debut, fin, gains_variables(&mvts))?;
    for (t, m) in [
        ("salaire", c.salaire),
        ("deduction_absence", -c.deduction),
        ("cotisation_inps", -c.inps_salarie),
        ("cotisation_amo", -c.amo_salarie),
    ] {
        if m != 0 {
            mvts.push(Mvt { type_: t.into(), montant: m, quantite: None, bulletin_id: None });
        }
    }
    assembler(conn, &e, debut, fin, &mvts, prev, &c)
}

/// RG-PAI-03/04/06 : clôture de la période et bulletin figé.
pub fn cloturer(db: &mut Db, acteur: &Acteur, employe_id: &str, debut: &str, fin: &str) -> Resultat<Bulletin> {
    valider_periode(debut, fin)?;
    let id = db.executer(acteur, |op| {
        let autorise_par = op.exiger(perm::PAIE_GERER)?;
        let e = employe(op, employe_id)?;
        let prev = dernier_bulletin(op, employe_id)?;
        if let Some((_, fin_prev, _)) = &prev {
            if debut <= fin_prev.as_str() {
                return Err(Erreur::regle(
                    "RG-PAI-06",
                    format!("Période déjà clôturée jusqu'au {fin_prev} : commencez après cette date"),
                ));
            }
        }
        let prev_seq = prev.as_ref().map(|b| b.2).unwrap_or(0);
        let avant = mouvements_apres(op, employe_id, prev_seq)?;
        let c = calculer(op, &e, &op.params, debut, fin, gains_variables(&avant))?;
        let bid = op.nouvel_id();
        let motif = format!("Période du {debut} au {fin}");
        for (t, m, q) in [
            ("salaire", c.salaire, (e.type_remuneration == "journalier").then_some(c.jours_presents)),
            ("deduction_absence", -c.deduction, Some(c.jours_absence_nj)),
            ("cotisation_inps", -c.inps_salarie, None),
            ("cotisation_amo", -c.amo_salarie, None),
        ] {
            if m != 0 {
                // Daté de la fin de période : le rapport du mois compte bien ses salaires (RG-RAP-03).
                inserer_mouvement_date(op, Some(fin), employe_id, t, m, q, None, None, Some(&bid), &motif, autorise_par.as_deref())?;
            }
        }
        let mvts = mouvements_apres(op, employe_id, prev_seq)?;
        let premier: i64 = op.query_row(
            "SELECT COALESCE(MIN(seq), 0) FROM mouvements_employe WHERE employe_id = ?1 AND seq > ?2",
            params![employe_id, prev_seq],
            |r| r.get(0),
        )?;
        let dernier: i64 = op.query_row(
            "SELECT COALESCE(MAX(seq), ?2) FROM mouvements_employe WHERE employe_id = ?1",
            params![employe_id, prev_seq],
            |r| r.get(0),
        )?;
        // Les mouvements de la clôture portent bulletin_id : on les compte comme « du bulletin » et non « anciens ».
        let mvts: Vec<Mvt> = mvts
            .into_iter()
            .map(|m| if m.type_ == "paiement" { m } else { Mvt { bulletin_id: None, ..m } })
            .collect();
        let b = assembler(op, &e, debut, fin, &mvts, prev_seq, &c)?;
        let solde = employes::solde(op, employe_id)?;
        debug_assert_eq!(b.net_a_payer, solde, "RG-PAI-04 : net = solde du compte");
        if b.net_a_payer != solde {
            return Err(Erreur::validation("Incohérence du compte employé : contactez le support"));
        }
        let numero = op.sequence("bulletin")?;
        op.execute(
            "INSERT INTO bulletins(id, numero, employe_id, debut, fin, type_remuneration, type_contrat, base, jours_presents,
                jours_absence_nj, report_precedent, total_gains, total_retenues, cotisations_salarie, charges_employeur,
                deja_paye, net_a_payer, premier_seq, dernier_seq, detail_json, horodatage, utilisateur_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22)",
            params![
                bid, numero, employe_id, debut, fin, e.type_remuneration, e.type_contrat, e.montant_base, c.jours_presents,
                c.jours_absence_nj, b.report_precedent, b.total_gains, b.total_retenues, b.cotisations_salarie,
                b.charges_employeur, b.deja_paye, b.net_a_payer, premier, dernier, serde_json::to_string(&b.lignes)?,
                op.maintenant, op.utilisateur()
            ],
        )?;
        op.audit("paie.cloturer", "bulletin", Some(&bid), None, Some(json!({ "employe": e.nom, "net": b.net_a_payer, "debut": debut, "fin": fin })), None, autorise_par.as_deref())?;
        op.outbox("bulletin", &bid, "creer")?;
        Ok(bid)
    })?;
    bulletin(db.conn(), &id)
}

pub fn bulletin(conn: &Connection, id: &str) -> Resultat<Bulletin> {
    let mut b = trouver(
        conn.query_row(
            "SELECT b.id, b.numero, b.employe_id, e.nom, e.fonction, b.debut, b.fin, b.type_remuneration, b.type_contrat, b.base,
                    b.jours_presents, b.jours_absence_nj, b.report_precedent, b.total_gains, b.total_retenues,
                    b.cotisations_salarie, b.charges_employeur, b.deja_paye, b.net_a_payer, b.detail_json
             FROM bulletins b JOIN employes e ON e.id = b.employe_id WHERE b.id = ?1",
            params![id],
            |r| {
                let detail: String = r.get(19)?;
                Ok(Bulletin {
                    id: Some(r.get(0)?),
                    numero: Some(r.get(1)?),
                    employe_id: r.get(2)?,
                    employe_nom: r.get(3)?,
                    fonction: r.get(4)?,
                    debut: r.get(5)?,
                    fin: r.get(6)?,
                    type_remuneration: r.get(7)?,
                    type_contrat: r.get(8)?,
                    base: r.get(9)?,
                    jours_presents: r.get(10)?,
                    jours_absence_nj: r.get(11)?,
                    report_precedent: r.get(12)?,
                    total_gains: r.get(13)?,
                    total_retenues: r.get(14)?,
                    cotisations_salarie: r.get(15)?,
                    charges_employeur: r.get(16)?,
                    deja_paye: r.get(17)?,
                    net_a_payer: r.get(18)?,
                    lignes: serde_json::from_str(&detail).unwrap_or_default(),
                    paye_depuis: 0,
                    reste_a_payer: 0,
                })
            },
        ),
        "Bulletin",
    )?;
    b.paye_depuis = conn.query_row(
        "SELECT COALESCE(-SUM(montant), 0) FROM mouvements_employe WHERE bulletin_id = ?1 AND type = 'paiement'",
        params![id],
        |r| r.get(0),
    )?;
    b.reste_a_payer = b.net_a_payer - b.paye_depuis;
    Ok(b)
}

pub fn bulletins(conn: &Connection, employe_id: Option<&str>) -> Resultat<Vec<Bulletin>> {
    let ids: Vec<String> = conn
        .prepare("SELECT id FROM bulletins WHERE (?1 IS NULL OR employe_id = ?1) ORDER BY horodatage DESC LIMIT 200")?
        .query_map(params![employe_id], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    ids.iter().map(|i| bulletin(conn, i)).collect()
}

#[derive(Debug, Deserialize)]
pub struct PaiementSalaire {
    pub employe_id: String,
    pub montant: i64,
    #[serde(default)]
    pub bulletin_id: Option<String>,
    #[serde(default)]
    pub compte_id: Option<String>,
    #[serde(default)]
    pub note: String,
}

/// RG-PAI-05 : paiement (éventuellement partiel) depuis un compte de trésorerie.
pub fn payer(db: &mut Db, acteur: &Acteur, p: &PaiementSalaire) -> Resultat<String> {
    db.executer(acteur, |op| {
        let autorise_par = op.exiger(perm::PAIE_GERER)?;
        let e = employe(op, &p.employe_id)?;
        if p.montant <= 0 {
            return Err(Erreur::validation("Montant invalide"));
        }
        if let Some(b) = &p.bulletin_id {
            let bul = bulletin(op, b)?;
            if bul.employe_id != p.employe_id {
                return Err(Erreur::validation("Bulletin d'un autre employé"));
            }
            if p.montant > bul.reste_a_payer {
                return Err(Erreur::regle("RG-PAI-05", format!("Il reste {} à payer sur ce bulletin", bul.reste_a_payer)));
            }
        } else {
            let s = employes::solde(op, &p.employe_id)?;
            if p.montant > s {
                return Err(Erreur::regle("RG-PAI-05", format!("Le compte de l'employé n'est créditeur que de {s}")));
            }
        }
        let (compte, session) = employes::compte_payeur(op, p.compte_id.as_deref())?;
        let mvt = crate::caisse::mouvement(op, &compte, session.as_deref(), "paiement_salaire", -p.montant, Some(("employe", &p.employe_id)), &format!("Salaire {}", e.nom), autorise_par.as_deref())?;
        let id = inserer_mouvement(op, &p.employe_id, "paiement", -p.montant, None, Some((&compte, &mvt)), None, p.bulletin_id.as_deref(), p.note.trim(), autorise_par.as_deref())?;
        op.audit("paie.payer", "employe", Some(&p.employe_id), None, Some(json!({ "montant": p.montant, "bulletin": p.bulletin_id })), None, autorise_par.as_deref())?;
        op.evenement("caisse", None);
        Ok(id)
    })
}

/// Salaires et avances à venir (tableau de bord) : soldes créditeurs des employés actifs.
pub fn a_payer(conn: &Connection) -> Resultat<(i64, i64)> {
    let (dus, avances): (i64, i64) = conn.query_row(
        "SELECT
            COALESCE((SELECT SUM(s) FROM (SELECT SUM(m.montant) s FROM mouvements_employe m JOIN employes e ON e.id = m.employe_id
                       WHERE e.statut <> 'parti' GROUP BY m.employe_id HAVING SUM(m.montant) > 0)), 0),
            COALESCE((SELECT -SUM(m.montant) FROM mouvements_employe m WHERE m.type = 'avance'
                       AND m.seq > COALESCE((SELECT MAX(b.dernier_seq) FROM bulletins b WHERE b.employe_id = m.employe_id), 0)), 0)",
        [],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    Ok((dus, avances))
}
