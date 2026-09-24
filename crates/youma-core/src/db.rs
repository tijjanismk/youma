use std::collections::HashSet;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::Serialize;

use crate::erreur::{Erreur, Resultat};
use crate::horloge::{format_ms, Horloge, TOLERANCE_RECUL_MS};
use crate::journee::Journee;
use crate::parametres::{self, Parametres};
use crate::{auth, permissions};

const MIGRATIONS: &[(i64, &str)] = &[
    (1, include_str!("../migrations/0001_initial.sql")),
    (2, include_str!("../migrations/0002_mot_de_passe_sortie_operateurs.sql")),
    (3, include_str!("../migrations/0003_canaux_zones_risque.sql")),
];

pub fn version_schema() -> i64 {
    MIGRATIONS.last().map(|m| m.0).unwrap_or(0)
}

/// Qui agit. `autorisation` = PIN d'un responsable présent (RG-AUT-03).
#[derive(Debug, Clone, Default)]
pub struct Acteur {
    pub utilisateur_id: Option<String>,
    pub appareil_id: Option<String>,
    pub autorisation_pin: Option<String>,
    /// Session confirmée par mot de passe (RG-AUT-06).
    pub eleve: bool,
    systeme: bool,
}

impl Acteur {
    pub fn utilisateur(id: impl Into<String>) -> Self {
        Acteur { utilisateur_id: Some(id.into()), ..Default::default() }
    }
    /// Tâches internes (sauvegarde planifiée, données de démonstration).
    pub fn systeme() -> Self {
        Acteur { systeme: true, eleve: true, ..Default::default() }
    }
    pub fn avec_eleve(mut self, eleve: bool) -> Self {
        self.eleve = eleve;
        self
    }
    pub fn avec_appareil(mut self, appareil: Option<String>) -> Self {
        self.appareil_id = appareil;
        self
    }
    pub fn avec_autorisation(mut self, pin: Option<String>) -> Self {
        self.autorisation_pin = pin.filter(|p| !p.is_empty());
        self
    }
}

pub type Ecouteur = Arc<dyn Fn(Evenement) + Send + Sync>;

pub struct Db {
    conn: Connection,
    horloge: Arc<dyn Horloge>,
    chemin: Option<PathBuf>,
    ecouteur: Option<Ecouteur>,
}

impl Db {
    /// Ouvre (ou crée) la base, applique les PRAGMA de durabilité et les migrations.
    pub fn ouvrir(chemin: &Path, horloge: Arc<dyn Horloge>) -> Resultat<Db> {
        if let Some(dossier) = chemin.parent() {
            std::fs::create_dir_all(dossier)?;
        }
        let conn = Connection::open(chemin)?;
        let mut db = Db { conn, horloge, chemin: Some(chemin.to_path_buf()), ecouteur: None };
        db.configurer()?;
        db.migrer()?;
        Ok(db)
    }

    pub fn en_memoire(horloge: Arc<dyn Horloge>) -> Resultat<Db> {
        let conn = Connection::open_in_memory()?;
        let mut db = Db { conn, horloge, chemin: None, ecouteur: None };
        db.configurer()?;
        db.migrer()?;
        Ok(db)
    }

    fn configurer(&self) -> Resultat<()> {
        // Fiche 0004 : WAL + synchronous=FULL → un commit validé survit à une coupure.
        self.conn.pragma_update(None, "journal_mode", "WAL")?;
        self.conn.pragma_update(None, "synchronous", "FULL")?;
        self.conn.pragma_update(None, "foreign_keys", "ON")?;
        self.conn.pragma_update(None, "busy_timeout", 5_000)?;
        self.conn.pragma_update(None, "wal_autocheckpoint", 1_000)?;
        Ok(())
    }

    fn migrer(&mut self) -> Resultat<()> {
        let actuelle: i64 = self.conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
        if actuelle > 0 && actuelle < version_schema() {
            // Sauvegarde automatique avant mise à jour (cahier §20).
            if let Some(chemin) = &self.chemin {
                let dest = chemin.with_extension(format!("avant-migration-v{actuelle}.db"));
                let _ = std::fs::remove_file(&dest);
                self.conn.execute("VACUUM INTO ?1", params![dest.to_string_lossy()])?;
            }
        }
        for (version, sql) in MIGRATIONS {
            if *version > actuelle {
                let tx = self.conn.transaction()?;
                tx.execute_batch(sql)?;
                tx.pragma_update(None, "user_version", version)?;
                tx.commit()?;
            }
        }
        let maintenant = self.horloge.maintenant_ms();
        let tx = self.conn.transaction()?;
        initialiser_si_vide(&tx, maintenant)?;
        tx.commit()?;
        Ok(())
    }

    /// Événements métier émis après chaque commit (poussés sur le WebSocket).
    pub fn definir_ecouteur(&mut self, e: Ecouteur) {
        self.ecouteur = Some(e);
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    pub fn chemin(&self) -> Option<&Path> {
        self.chemin.as_deref()
    }

    pub fn horloge(&self) -> Arc<dyn Horloge> {
        self.horloge.clone()
    }

    pub fn maintenant(&self) -> i64 {
        self.horloge.maintenant_ms()
    }

    /// Remplace la connexion (restauration de sauvegarde).
    pub(crate) fn remplacer_connexion(&mut self, conn: Connection) -> Resultat<()> {
        self.conn = conn;
        self.configurer()?;
        self.migrer()
    }

    pub(crate) fn fermer_pour_restauration(&mut self) -> Resultat<()> {
        // Point de contrôle WAL puis connexion mémoire temporaire, pour libérer le fichier.
        self.conn.pragma_update(None, "wal_checkpoint", "TRUNCATE")?;
        self.conn = Connection::open_in_memory()?;
        Ok(())
    }

    /// Lecture seule, hors transaction d'écriture.
    pub fn lire<T>(&self, f: impl FnOnce(&Connection) -> Resultat<T>) -> Resultat<T> {
        f(&self.conn)
    }

    /// État de l'horloge, pour l'écran d'accueil (RG-SYS-01).
    pub fn etat_horloge(&self) -> Resultat<EtatHorloge> {
        let dernier = dernier_horodatage(&self.conn)?;
        let maintenant = self.maintenant();
        Ok(EtatHorloge {
            maintenant,
            dernier_evenement: dernier,
            coherente: maintenant + TOLERANCE_RECUL_MS >= dernier,
        })
    }

    /// Opération métier : une transaction `BEGIN IMMEDIATE` (RG-SYS-02), garde d'horloge (RG-SYS-01).
    pub fn executer<T>(&mut self, acteur: &Acteur, f: impl FnOnce(&mut Op) -> Resultat<T>) -> Resultat<T> {
        self.executer_interne(acteur, true, f)
    }

    pub(crate) fn executer_sans_garde<T>(
        &mut self,
        acteur: &Acteur,
        f: impl FnOnce(&mut Op) -> Resultat<T>,
    ) -> Resultat<T> {
        self.executer_interne(acteur, false, f)
    }

    fn executer_interne<T>(
        &mut self,
        acteur: &Acteur,
        garde_horloge: bool,
        f: impl FnOnce(&mut Op) -> Resultat<T>,
    ) -> Resultat<T> {
        let maintenant = self.horloge.maintenant_ms();
        let tx = self.conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let dernier = dernier_horodatage(&tx)?;
        if garde_horloge && maintenant + TOLERANCE_RECUL_MS < dernier {
            return Err(Erreur::HorlogeIncoherente(format!(
                "PC : {}, dernier enregistrement : {}",
                format_ms(maintenant),
                format_ms(dernier)
            )));
        }
        let params = parametres::lire(&tx)?;
        let (permissions, plafond) = if acteur.systeme {
            (permissions::TOUTES.iter().map(|s| s.to_string()).collect(), 100)
        } else if let Some(uid) = &acteur.utilisateur_id {
            auth::permissions_utilisateur(&tx, uid)?
        } else {
            return Err(Erreur::NonAuthentifie);
        };
        let autorisateur = match &acteur.autorisation_pin {
            Some(pin) => Some(auth::verifier_pin_autorisation(&tx, pin, maintenant)?),
            None => None,
        };
        let mut op = Op {
            tx,
            maintenant,
            utilisateur_id: acteur.utilisateur_id.clone(),
            appareil_id: acteur.appareil_id.clone(),
            permissions,
            plafond_remise_pct: plafond,
            autorisateur,
            eleve: acteur.eleve,
            params,
            evenements: Vec::new(),
        };
        let resultat = f(&mut op)?;
        let nouveau = maintenant.max(dernier);
        op.tx.execute(
            "INSERT INTO systeme(cle, valeur) VALUES ('dernier_horodatage', ?1)
             ON CONFLICT(cle) DO UPDATE SET valeur = excluded.valeur",
            params![nouveau.to_string()],
        )?;
        let Op { tx, evenements, .. } = op;
        tx.commit()?;
        if let Some(ecouteur) = &self.ecouteur {
            for e in evenements {
                ecouteur(e);
            }
        }
        Ok(resultat)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Evenement {
    #[serde(rename = "type")]
    pub type_: &'static str,
    pub id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct EtatHorloge {
    pub maintenant: i64,
    pub dernier_evenement: i64,
    pub coherente: bool,
}

pub(crate) fn dernier_horodatage(conn: &Connection) -> Resultat<i64> {
    let v: Option<String> = conn
        .query_row("SELECT valeur FROM systeme WHERE cle = 'dernier_horodatage'", [], |r| r.get(0))
        .optional()?;
    Ok(v.and_then(|s| s.parse().ok()).unwrap_or(0))
}

/// Contexte d'une opération métier en cours (dans la transaction).
pub struct Op<'a> {
    pub tx: rusqlite::Transaction<'a>,
    pub maintenant: i64,
    pub utilisateur_id: Option<String>,
    pub appareil_id: Option<String>,
    permissions: HashSet<String>,
    pub plafond_remise_pct: i64,
    autorisateur: Option<auth::Autorisateur>,
    eleve: bool,
    pub params: Parametres,
    evenements: Vec<Evenement>,
}

impl Deref for Op<'_> {
    type Target = Connection;
    fn deref(&self) -> &Connection {
        &self.tx
    }
}

impl Op<'_> {
    pub fn a_permission(&self, p: &str) -> bool {
        self.permissions.contains(p)
    }

    /// RG-AUT-03 : `Ok(None)` si l'utilisateur a la permission, `Ok(Some(id))` si un
    /// responsable l'a autorisée ponctuellement, sinon `AUTORISATION_REQUISE`.
    pub fn exiger(&self, p: &str) -> Resultat<Option<String>> {
        // RG-AUT-06 : administration = mot de passe de l'utilisateur lui-même, jamais un PIN prêté.
        if permissions::ADMINISTRATION.contains(&p) {
            return match (self.a_permission(p), self.eleve) {
                (true, true) => Ok(None),
                (true, false) => Err(Erreur::MotDePasseRequis(p.to_string())),
                (false, _) => Err(Erreur::Interdit(p.to_string())),
            };
        }
        if self.a_permission(p) {
            return Ok(None);
        }
        match &self.autorisateur {
            Some(a) if a.permissions.contains(p) => Ok(Some(a.utilisateur_id.clone())),
            _ => Err(Erreur::AutorisationRequise(p.to_string())),
        }
    }

    /// Plafond de remise effectif (le plus haut entre l'utilisateur et l'autorisateur).
    pub fn plafond_remise(&self) -> (i64, Option<String>) {
        match &self.autorisateur {
            Some(a) if a.plafond_remise_pct > self.plafond_remise_pct => {
                (a.plafond_remise_pct, Some(a.utilisateur_id.clone()))
            }
            _ => (self.plafond_remise_pct, None),
        }
    }

    pub fn nouvel_id(&self) -> String {
        uuid::Uuid::now_v7().to_string()
    }

    /// RG-SYS-04 : numéro séquentiel sans trou (dans la transaction).
    pub fn sequence(&self, nom: &str) -> Resultat<i64> {
        let v: i64 = self.tx.query_row(
            "INSERT INTO sequences(nom, valeur) VALUES (?1, 1)
             ON CONFLICT(nom) DO UPDATE SET valeur = valeur + 1 RETURNING valeur",
            params![nom],
            |r| r.get(0),
        )?;
        Ok(v)
    }

    /// RG-SYS-05.
    #[allow(clippy::too_many_arguments)]
    pub fn audit(
        &self,
        action: &str,
        entite: &str,
        entite_id: Option<&str>,
        avant: Option<serde_json::Value>,
        apres: Option<serde_json::Value>,
        motif: Option<&str>,
        autorise_par: Option<&str>,
    ) -> Resultat<()> {
        self.tx.execute(
            "INSERT INTO journal_audit(id, horodatage, utilisateur_id, autorise_par, appareil_id, action, entite,
                                       entite_id, avant, apres, motif)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                self.nouvel_id(),
                self.maintenant,
                self.utilisateur_id,
                autorise_par,
                self.appareil_id,
                action,
                entite,
                entite_id,
                avant.map(|v| v.to_string()),
                apres.map(|v| v.to_string()),
                motif,
            ],
        )?;
        Ok(())
    }

    /// Changement à pousser vers le cloud (V2), dans la même transaction.
    pub fn outbox(&self, entite: &str, entite_id: &str, operation: &str) -> Resultat<()> {
        self.tx.execute(
            "INSERT INTO outbox(id, horodatage, entite, entite_id, operation) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![self.nouvel_id(), self.maintenant, entite, entite_id, operation],
        )?;
        Ok(())
    }

    /// Événement temps réel émis après commit.
    pub fn evenement(&mut self, type_: &'static str, id: Option<&str>) {
        self.evenements.push(Evenement { type_, id: id.map(str::to_owned) });
    }

    /// RG-JOU-01.
    pub fn journee_ouverte(&self) -> Resultat<Journee> {
        crate::journee::ouverte(&self.tx)?
            .ok_or_else(|| Erreur::regle("RG-JOU-01", "Aucune journée ouverte. Ouvrez la journée d'abord."))
    }

    pub fn utilisateur(&self) -> Option<&str> {
        self.utilisateur_id.as_deref()
    }
}

fn initialiser_si_vide(conn: &Connection, maintenant: i64) -> Resultat<()> {
    let existe: Option<String> = conn
        .query_row("SELECT valeur FROM systeme WHERE cle = 'installation_id'", [], |r| r.get(0))
        .optional()?;
    if existe.is_some() {
        return Ok(());
    }
    let id = || uuid::Uuid::now_v7().to_string();
    conn.execute("INSERT INTO systeme(cle, valeur) VALUES ('installation_id', ?1)", params![id()])?;
    conn.execute(
        "INSERT INTO restaurant(id, nom, modifie_le) VALUES (?1, 'Mon restaurant', ?2)",
        params![id(), maintenant],
    )?;
    parametres::ecrire(conn, &Parametres::default())?;
    for (code, nom, plafond, perms) in permissions::roles_par_defaut() {
        let rid = id();
        conn.execute(
            "INSERT INTO roles(id, code, nom, plafond_remise_pct, systeme, modifie_le) VALUES (?1, ?2, ?3, ?4, 1, ?5)",
            params![rid, code, nom, plafond, maintenant],
        )?;
        for p in perms {
            conn.execute("INSERT INTO role_permissions(role_id, permission) VALUES (?1, ?2)", params![rid, p])?;
        }
    }
    let comptes = [
        ("Caisse principale", "especes", ""),
        ("Orange Money", "mobile_money", "Orange"),
        ("Moov Money", "mobile_money", "Moov"),
        ("Wave", "mobile_money", "Wave"),
        ("Sama Money", "mobile_money", "Sama"),
        ("Coffre / propriétaire", "coffre", ""),
    ];
    for (i, (nom, type_, operateur)) in comptes.iter().enumerate() {
        conn.execute(
            "INSERT INTO comptes_tresorerie(id, nom, type, operateur, ordre, modifie_le) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id(), nom, type_, operateur, i as i64, maintenant],
        )?;
    }
    for nom in [
        "Électricité", "Eau", "Gaz / charbon", "Transport", "Salaire", "Entretien / réparation", "Achat urgent",
        "Téléphone / crédit / Internet", "Loyer", "Taxes", "Autres dépenses",
    ] {
        conn.execute("INSERT INTO categories_depense(id, nom) VALUES (?1, ?2)", params![id(), nom])?;
    }
    Ok(())
}

pub fn installation_id(conn: &Connection) -> Resultat<String> {
    Ok(conn.query_row("SELECT valeur FROM systeme WHERE cle = 'installation_id'", [], |r| r.get(0))?)
}

/// Aide : lit une ligne ou renvoie `NonTrouve`.
pub fn trouver<T>(r: rusqlite::Result<T>, quoi: &str) -> Resultat<T> {
    match r {
        Ok(v) => Ok(v),
        Err(rusqlite::Error::QueryReturnedNoRows) => Err(Erreur::NonTrouve(quoi.to_string())),
        Err(e) => Err(e.into()),
    }
}
