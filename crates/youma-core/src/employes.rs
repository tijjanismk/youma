//! Employés : conçu pour le terrain malien. La plupart des employés n'ont ni contrat
//! écrit, ni INPS, ni AMO : tout cela est facultatif (RG-EMP-01/02).

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::catalogue::non_vide;
use crate::db::{trouver, Acteur, Db, Op};
use crate::erreur::{Erreur, Resultat};
use crate::horloge::date_locale;
use crate::permissions as perm;
use crate::{caisse, commandes};

pub const TYPES_CONTRAT: &[&str] = &["aucun", "verbal", "journalier", "essai", "apprentissage", "stage", "cdd", "cdi"];
pub const TYPES_REMUNERATION: &[&str] = &["mensuel", "hebdomadaire", "journalier", "tache", "aucun"];
pub const FONCTIONS: &[&str] = &[
    "gérant", "serveur", "caissier", "cuisinier", "aide-cuisinier", "grilleur", "barman", "livreur", "plongeur",
    "gardien", "agent d'entretien", "responsable", "autre",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Employe {
    #[serde(default)]
    pub id: String,
    pub nom: String,
    #[serde(default)]
    pub surnom: String,
    #[serde(default)]
    pub telephone: String,
    #[serde(default = "autre")]
    pub fonction: String,
    #[serde(default)]
    pub date_embauche: Option<String>,
    pub type_remuneration: String,
    /// Salaire mensuel / hebdomadaire, taux journalier ou prix par tâche.
    #[serde(default)]
    pub montant_base: i64,
    #[serde(default = "aucun")]
    pub type_contrat: String,
    #[serde(default)]
    pub date_fin_contrat: Option<String>,
    #[serde(default)]
    pub piece_identite: String,
    #[serde(default)]
    pub contact_urgence: String,
    #[serde(default)]
    pub quartier: String,
    #[serde(default)]
    pub declare_inps: bool,
    #[serde(default)]
    pub numero_inps: String,
    #[serde(default)]
    pub affilie_amo: bool,
    #[serde(default)]
    pub numero_amo: String,
    /// RG-EMP-05 : logé, nourri… (informatif).
    #[serde(default)]
    pub avantages_nature: String,
    /// Plafond d'avance propre à l'employé (sinon paramètre général).
    #[serde(default)]
    pub plafond_avance: Option<i64>,
    #[serde(default)]
    pub horaires: String,
    #[serde(default = "actif")]
    pub statut: String,
    #[serde(default)]
    pub date_depart: Option<String>,
    #[serde(default)]
    pub notes: String,
    /// Solde du compte employé (+ dû à l'employé, − dû par l'employé). Lecture seule.
    #[serde(default)]
    pub solde: i64,
}

fn autre() -> String {
    "autre".into()
}
fn aucun() -> String {
    "aucun".into()
}
fn actif() -> String {
    "actif".into()
}

const COLS: &str = "e.id, e.nom, e.surnom, e.telephone, e.fonction, e.date_embauche, e.type_remuneration, e.montant_base,
    e.type_contrat, e.date_fin_contrat, e.piece_identite, e.contact_urgence, e.quartier, e.declare_inps, e.numero_inps,
    e.affilie_amo, e.numero_amo, e.avantages_nature, e.plafond_avance, e.horaires, e.statut, e.date_depart, e.notes,
    (SELECT COALESCE(SUM(m.montant), 0) FROM mouvements_employe m WHERE m.employe_id = e.id)";

fn depuis(r: &rusqlite::Row) -> rusqlite::Result<Employe> {
    Ok(Employe {
        id: r.get(0)?,
        nom: r.get(1)?,
        surnom: r.get(2)?,
        telephone: r.get(3)?,
        fonction: r.get(4)?,
        date_embauche: r.get(5)?,
        type_remuneration: r.get(6)?,
        montant_base: r.get(7)?,
        type_contrat: r.get(8)?,
        date_fin_contrat: r.get(9)?,
        piece_identite: r.get(10)?,
        contact_urgence: r.get(11)?,
        quartier: r.get(12)?,
        declare_inps: r.get(13)?,
        numero_inps: r.get(14)?,
        affilie_amo: r.get(15)?,
        numero_amo: r.get(16)?,
        avantages_nature: r.get(17)?,
        plafond_avance: r.get(18)?,
        horaires: r.get(19)?,
        statut: r.get(20)?,
        date_depart: r.get(21)?,
        notes: r.get(22)?,
        solde: r.get(23)?,
    })
}

pub fn lister(conn: &Connection, inclure_partis: bool) -> Resultat<Vec<Employe>> {
    let mut s = conn.prepare(&format!(
        "SELECT {COLS} FROM employes e WHERE (?1 = 1 OR e.statut <> 'parti') ORDER BY e.statut, e.nom"
    ))?;
    let v = s.query_map(params![inclure_partis], depuis)?.collect::<Result<Vec<_>, _>>()?;
    Ok(v)
}

pub fn employe(conn: &Connection, id: &str) -> Resultat<Employe> {
    trouver(conn.query_row(&format!("SELECT {COLS} FROM employes e WHERE e.id = ?1"), params![id], depuis), "Employé")
}

fn valider_date(d: &Option<String>, quoi: &str) -> Resultat<()> {
    if let Some(d) = d {
        if !d.is_empty() && chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").is_err() {
            return Err(Erreur::validation(format!("{quoi} : date invalide (AAAA-MM-JJ)")));
        }
    }
    Ok(())
}

/// RG-EMP-01/02/04 : seuls le nom et le mode de rémunération sont obligatoires.
pub fn enregistrer(db: &mut Db, acteur: &Acteur, e: &Employe) -> Resultat<String> {
    db.executer(acteur, |op| {
        op.exiger(perm::EMPLOYE_GERER)?;
        non_vide(&e.nom, "nom")?;
        if !TYPES_REMUNERATION.contains(&e.type_remuneration.as_str()) {
            return Err(Erreur::regle("RG-EMP-04", "Mode de rémunération inconnu"));
        }
        if !TYPES_CONTRAT.contains(&e.type_contrat.as_str()) {
            return Err(Erreur::regle("RG-EMP-02", "Type de contrat inconnu"));
        }
        if !["actif", "suspendu", "parti"].contains(&e.statut.as_str()) {
            return Err(Erreur::validation("Statut inconnu"));
        }
        if e.montant_base < 0 || e.plafond_avance.is_some_and(|p| p < 0) {
            return Err(Erreur::validation("Montant négatif"));
        }
        if e.type_remuneration != "aucun" && e.type_remuneration != "tache" && e.montant_base == 0 {
            return Err(Erreur::regle("RG-EMP-04", "Indiquez le salaire ou le taux journalier"));
        }
        valider_date(&e.date_embauche, "Date d'embauche")?;
        valider_date(&e.date_fin_contrat, "Fin de contrat")?;
        valider_date(&e.date_depart, "Date de départ")?;
        let nouveau = e.id.is_empty();
        let id = if nouveau { op.nouvel_id() } else { e.id.clone() };
        let avant: Option<(String, i64, String)> = op
            .query_row(
                "SELECT type_remuneration, montant_base, statut FROM employes WHERE id = ?1",
                params![id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?;
        let date_depart = if e.statut == "parti" {
            e.date_depart.clone().filter(|d| !d.is_empty()).or_else(|| Some(date_locale(op.maintenant, op.params.fuseau_minutes)))
        } else {
            None
        };
        let vide = |s: &Option<String>| s.clone().filter(|d| !d.is_empty());
        op.execute(
            "INSERT INTO employes(id, nom, surnom, telephone, fonction, date_embauche, type_remuneration, montant_base,
                type_contrat, date_fin_contrat, piece_identite, contact_urgence, quartier, declare_inps, numero_inps,
                affilie_amo, numero_amo, avantages_nature, plafond_avance, horaires, statut, date_depart, notes, cree_le, modifie_le)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?24)
             ON CONFLICT(id) DO UPDATE SET nom = excluded.nom, surnom = excluded.surnom, telephone = excluded.telephone,
                fonction = excluded.fonction, date_embauche = excluded.date_embauche,
                type_remuneration = excluded.type_remuneration, montant_base = excluded.montant_base,
                type_contrat = excluded.type_contrat, date_fin_contrat = excluded.date_fin_contrat,
                piece_identite = excluded.piece_identite, contact_urgence = excluded.contact_urgence,
                quartier = excluded.quartier, declare_inps = excluded.declare_inps, numero_inps = excluded.numero_inps,
                affilie_amo = excluded.affilie_amo, numero_amo = excluded.numero_amo,
                avantages_nature = excluded.avantages_nature, plafond_avance = excluded.plafond_avance,
                horaires = excluded.horaires, statut = excluded.statut, date_depart = excluded.date_depart,
                notes = excluded.notes, modifie_le = excluded.modifie_le, version = version + 1",
            params![
                id,
                e.nom.trim(),
                e.surnom.trim(),
                e.telephone.trim(),
                e.fonction.trim(),
                vide(&e.date_embauche),
                e.type_remuneration,
                e.montant_base,
                e.type_contrat,
                vide(&e.date_fin_contrat),
                e.piece_identite.trim(),
                e.contact_urgence.trim(),
                e.quartier.trim(),
                e.declare_inps,
                e.numero_inps.trim(),
                e.affilie_amo,
                e.numero_amo.trim(),
                e.avantages_nature.trim(),
                e.plafond_avance,
                e.horaires.trim(),
                e.statut,
                date_depart,
                e.notes,
                op.maintenant
            ],
        )?;
        match avant {
            None => op.audit("employe.creer", "employe", Some(&id), None, Some(json!({ "nom": e.nom, "remuneration": e.type_remuneration, "montant": e.montant_base })), None, None)?,
            Some((t, m, s)) if t != e.type_remuneration || m != e.montant_base || s != e.statut => op.audit(
                "employe.salaire",
                "employe",
                Some(&id),
                Some(json!({ "remuneration": t, "montant": m, "statut": s })),
                Some(json!({ "remuneration": e.type_remuneration, "montant": e.montant_base, "statut": e.statut })),
                None,
                None,
            )?,
            _ => {}
        }
        op.outbox("employe", &id, "enregistrer")?;
        Ok(id)
    })
}

// ───────────── Présences ─────────────

pub const STATUTS_PRESENCE: &[&str] = &["present", "retard", "absent_justifie", "absent_non_justifie", "conge", "repos"];

#[derive(Debug, Deserialize)]
pub struct SaisiePresence {
    pub employe_id: String,
    /// Défaut : date de la journée d'exploitation ouverte (ou date du jour).
    #[serde(default)]
    pub date: Option<String>,
    pub statut: String,
    #[serde(default)]
    pub minutes_retard: i64,
    #[serde(default)]
    pub note: String,
}

/// RG-EMP-07 : la dernière saisie du jour fait foi (ajout seul).
pub fn pointer(db: &mut Db, acteur: &Acteur, saisies: &[SaisiePresence]) -> Resultat<()> {
    db.executer(acteur, |op| {
        op.exiger(perm::EMPLOYE_PRESENCE)?;
        let defaut = match crate::journee::ouverte(op)? {
            Some(j) => j.date_exploitation,
            None => date_locale(op.maintenant, op.params.fuseau_minutes),
        };
        for s in saisies {
            if !STATUTS_PRESENCE.contains(&s.statut.as_str()) {
                return Err(Erreur::validation("Statut de présence inconnu"));
            }
            let date = s.date.clone().filter(|d| !d.is_empty()).unwrap_or_else(|| defaut.clone());
            valider_date(&Some(date.clone()), "Date")?;
            employe(op, &s.employe_id)?;
            op.execute(
                "INSERT INTO presences(id, employe_id, date, statut, minutes_retard, note, horodatage, utilisateur_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![op.nouvel_id(), s.employe_id, date, s.statut, s.minutes_retard.max(0), s.note.trim(), op.maintenant, op.utilisateur()],
            )?;
        }
        op.evenement("employes", None);
        Ok(())
    })
}

#[derive(Debug, Serialize, Clone)]
pub struct Presence {
    pub employe_id: String,
    pub date: String,
    pub statut: String,
    pub minutes_retard: i64,
    pub note: String,
}

/// Présences effectives (dernière saisie par jour) sur une période.
pub fn presences(conn: &Connection, employe_id: Option<&str>, debut: &str, fin: &str) -> Resultat<Vec<Presence>> {
    let mut s = conn.prepare(
        "SELECT p.employe_id, p.date, p.statut, p.minutes_retard, p.note FROM presences p
         WHERE p.date BETWEEN ?1 AND ?2 AND (?3 IS NULL OR p.employe_id = ?3)
           AND p.rowid = (SELECT q.rowid FROM presences q WHERE q.employe_id = p.employe_id AND q.date = p.date
                          ORDER BY q.horodatage DESC, q.rowid DESC LIMIT 1)
         ORDER BY p.date, p.employe_id",
    )?;
    let v = s
        .query_map(params![debut, fin, employe_id], |r| {
            Ok(Presence { employe_id: r.get(0)?, date: r.get(1)?, statut: r.get(2)?, minutes_retard: r.get(3)?, note: r.get(4)? })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(v)
}

// ───────────── Compte employé ─────────────

pub fn solde(conn: &Connection, employe_id: &str) -> Resultat<i64> {
    Ok(conn.query_row(
        "SELECT COALESCE(SUM(montant), 0) FROM mouvements_employe WHERE employe_id = ?1",
        params![employe_id],
        |r| r.get(0),
    )?)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn inserer_mouvement(
    op: &Op,
    employe_id: &str,
    type_: &str,
    montant: i64,
    quantite: Option<i64>,
    compte: Option<(&str, &str)>,
    commande_id: Option<&str>,
    bulletin_id: Option<&str>,
    motif: &str,
    autorise_par: Option<&str>,
) -> Resultat<String> {
    inserer_mouvement_date(op, None, employe_id, type_, montant, quantite, compte, commande_id, bulletin_id, motif, autorise_par)
}

/// `date` : date de rattachement (fin de période pour les salaires d'une clôture de paie),
/// sinon la journée d'exploitation ouverte.
#[allow(clippy::too_many_arguments)]
pub(crate) fn inserer_mouvement_date(
    op: &Op,
    date: Option<&str>,
    employe_id: &str,
    type_: &str,
    montant: i64,
    quantite: Option<i64>,
    compte: Option<(&str, &str)>,
    commande_id: Option<&str>,
    bulletin_id: Option<&str>,
    motif: &str,
    autorise_par: Option<&str>,
) -> Resultat<String> {
    let date = match (date, crate::journee::ouverte(op)?) {
        (Some(d), _) => d.to_string(),
        (None, Some(j)) => j.date_exploitation,
        (None, None) => date_locale(op.maintenant, op.params.fuseau_minutes),
    };
    let id = op.nouvel_id();
    op.execute(
        "INSERT INTO mouvements_employe(id, employe_id, type, montant, quantite, date, compte_id, mouvement_tresorerie_id,
            commande_id, bulletin_id, motif, horodatage, utilisateur_id, autorise_par)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
        params![
            id,
            employe_id,
            type_,
            montant,
            quantite,
            date,
            compte.map(|c| c.0),
            compte.map(|c| c.1),
            commande_id,
            bulletin_id,
            motif,
            op.maintenant,
            op.utilisateur(),
            autorise_par
        ],
    )?;
    op.outbox("mouvement_employe", &id, "creer")?;
    Ok(id)
}

/// Salaire mensuel de référence, pour le plafond d'avance.
fn reference_mensuelle(e: &Employe, jours_ouvrables: i64) -> i64 {
    match e.type_remuneration.as_str() {
        "mensuel" => e.montant_base,
        "hebdomadaire" => e.montant_base * 4,
        "journalier" => e.montant_base * jours_ouvrables,
        _ => 0,
    }
}

#[derive(Debug, Deserialize)]
pub struct Avance {
    pub employe_id: String,
    pub montant: i64,
    /// Compte payeur (défaut : caisse de la session).
    #[serde(default)]
    pub compte_id: Option<String>,
    #[serde(default)]
    pub motif: String,
}

/// RG-PAI-02.
pub fn avance(db: &mut Db, acteur: &Acteur, a: &Avance) -> Resultat<String> {
    db.executer(acteur, |op| {
        let mut autorise_par = op.exiger(perm::EMPLOYE_AVANCE)?;
        let e = employe(op, &a.employe_id)?;
        if a.montant <= 0 {
            return Err(Erreur::validation("Montant invalide"));
        }
        let reference = reference_mensuelle(&e, op.params.paie.jours_ouvrables_mois);
        let plafond = e.plafond_avance.or_else(|| {
            (op.params.paie.plafond_avance_pct > 0 && reference > 0).then(|| reference * op.params.paie.plafond_avance_pct / 100)
        });
        if let Some(plafond) = plafond {
            let deja = avances_depuis_bulletin(op, &a.employe_id)?;
            if deja + a.montant > plafond {
                autorise_par = op.exiger(perm::EMPLOYE_DEPASSER_PLAFOND).map_err(|_| {
                    Erreur::AutorisationRequise(format!(
                        "{} — avances {} + {} > plafond {plafond}",
                        perm::EMPLOYE_DEPASSER_PLAFOND,
                        deja,
                        a.montant
                    ))
                })?;
            }
        }
        let (compte, session) = compte_payeur(op, a.compte_id.as_deref())?;
        let libelle = format!("Avance {}", e.nom);
        let mvt = caisse::mouvement(op, &compte, session.as_deref(), "avance_salaire", -a.montant, Some(("employe", &a.employe_id)), &libelle, autorise_par.as_deref())?;
        let id = inserer_mouvement(op, &a.employe_id, "avance", -a.montant, None, Some((&compte, &mvt)), None, None, a.motif.trim(), autorise_par.as_deref())?;
        op.audit("employe.avance", "employe", Some(&a.employe_id), None, Some(json!({ "montant": a.montant })), Some(a.motif.trim()).filter(|m| !m.is_empty()), autorise_par.as_deref())?;
        op.evenement("caisse", None);
        Ok(id)
    })
}

fn avances_depuis_bulletin(conn: &Connection, employe_id: &str) -> Resultat<i64> {
    let dernier: i64 = conn.query_row(
        "SELECT COALESCE(MAX(dernier_seq), 0) FROM bulletins WHERE employe_id = ?1",
        params![employe_id],
        |r| r.get(0),
    )?;
    let v: i64 = conn.query_row(
        "SELECT COALESCE(-SUM(montant), 0) FROM mouvements_employe WHERE employe_id = ?1 AND type = 'avance' AND seq > ?2",
        params![employe_id, dernier],
        |r| r.get(0),
    )?;
    Ok(v)
}

pub(crate) fn compte_payeur(op: &Op, compte_id: Option<&str>) -> Resultat<(String, Option<String>)> {
    let session = op.utilisateur().map(|u| caisse::session_utilisateur(op, u)).transpose()?.flatten();
    match (compte_id, session) {
        // Rattachée à la session seulement si l'argent sort de son tiroir (un paiement depuis le coffre n'y est pour rien).
        (Some(c), s) => Ok((c.to_string(), s.filter(|s| s.compte_id == c).map(|s| s.id))),
        (None, Some(s)) => Ok((s.compte_id.clone(), Some(s.id))),
        (None, None) => Err(Erreur::regle("RG-CAI-01", "Ouvrez une session de caisse ou choisissez le compte payeur")),
    }
}

#[derive(Debug, Deserialize)]
pub struct Evenement {
    pub employe_id: String,
    /// prime | retenue | tache | regularisation
    #[serde(rename = "type")]
    pub type_: String,
    /// Montant positif (régularisation : signé). Pour une tâche : prix unitaire optionnel.
    #[serde(default)]
    pub montant: i64,
    /// Nombre de tâches (courses, services…).
    #[serde(default)]
    pub quantite: Option<i64>,
    #[serde(default)]
    pub motif: String,
}

/// Prime, retenue, tâches effectuées, régularisation (RG-PAI-01/06).
pub fn evenement(db: &mut Db, acteur: &Acteur, ev: &Evenement) -> Resultat<String> {
    db.executer(acteur, |op| {
        let autorise_par = op.exiger(perm::PAIE_GERER)?;
        let e = employe(op, &ev.employe_id)?;
        let (montant, quantite) = match ev.type_.as_str() {
            "prime" if ev.montant > 0 => (ev.montant, None),
            "retenue" if ev.montant > 0 => (-ev.montant, None),
            "tache" => {
                let q = ev.quantite.unwrap_or(1);
                let pu = if ev.montant > 0 { ev.montant } else { e.montant_base };
                if q <= 0 || pu <= 0 {
                    return Err(Erreur::validation("Nombre de tâches ou prix invalide"));
                }
                (q * pu, Some(q))
            }
            "regularisation" if ev.montant != 0 => (ev.montant, None),
            _ => return Err(Erreur::validation("Type ou montant invalide")),
        };
        if ev.type_ != "tache" && ev.motif.trim().is_empty() {
            return Err(Erreur::validation("Motif obligatoire"));
        }
        let id = inserer_mouvement(op, &ev.employe_id, &ev.type_, montant, quantite, None, None, None, ev.motif.trim(), autorise_par.as_deref())?;
        op.audit(&format!("employe.{}", ev.type_), "employe", Some(&ev.employe_id), None, Some(json!({ "montant": montant, "quantite": quantite })), Some(ev.motif.trim()).filter(|m| !m.is_empty()), autorise_par.as_deref())?;
        Ok(id)
    })
}

/// RG-CMD-07 : la consommation d'un employé est imputée sur son compte, sans chiffre d'affaires.
pub fn imputer_commande(db: &mut Db, acteur: &Acteur, commande_id: &str) -> Resultat<i64> {
    db.executer(acteur, |op| {
        let autorise_par = op.exiger(perm::COMMANDE_CONSO_EMPLOYE)?;
        let c = commandes::detail(op, commande_id)?;
        let employe_id = c.employe_id.clone().ok_or_else(|| Erreur::validation("Commande sans employé"))?;
        if c.statut != "ouverte" {
            return Err(Erreur::regle("RG-CMD-08", "Commande déjà réglée"));
        }
        // Envoi et sortie de stock des articles encore en brouillon.
        commandes::envoyer_op(op, commande_id)?;
        let total = commandes::totaux(op, commande_id)?.total;
        if total > 0 {
            inserer_mouvement(op, &employe_id, "consommation", -total, None, None, Some(commande_id), None, &format!("Commande n°{}", c.numero), autorise_par.as_deref())?;
        }
        op.execute(
            "UPDATE commandes SET statut = 'payee', payee_le = ?1, modifie_le = ?1 WHERE id = ?2",
            params![op.maintenant, commande_id],
        )?;
        op.audit("employe.consommation", "employe", Some(&employe_id), None, Some(json!({ "commande": c.numero, "montant": total })), None, autorise_par.as_deref())?;
        op.evenement("table", None);
        Ok(total)
    })
}

#[derive(Debug, Serialize)]
pub struct LigneCompte {
    pub seq: i64,
    pub date: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub montant: i64,
    pub quantite: Option<i64>,
    pub motif: String,
    pub bulletin_id: Option<String>,
    pub solde: i64,
}

/// Relevé du compte employé, solde cumulé.
pub fn releve(conn: &Connection, employe_id: &str) -> Resultat<Vec<LigneCompte>> {
    let mut s = conn.prepare(
        "SELECT seq, date, type, montant, quantite, motif, bulletin_id FROM mouvements_employe WHERE employe_id = ?1 ORDER BY seq",
    )?;
    let mut solde = 0;
    let v = s
        .query_map(params![employe_id], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?, r.get::<_, i64>(3)?, r.get(4)?, r.get::<_, String>(5)?, r.get(6)?))
        })?
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .map(|(seq, date, type_, montant, quantite, motif, bulletin_id)| {
            solde += montant;
            LigneCompte { seq, date, type_, montant, quantite, motif, bulletin_id, solde }
        })
        .collect();
    Ok(v)
}
