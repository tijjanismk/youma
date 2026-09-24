use serde::Serialize;

pub type Resultat<T> = std::result::Result<T, Erreur>;

/// Erreur métier. `code` est stable : l'interface s'en sert pour réagir
/// (ex. `AUTORISATION_REQUISE` → demander le PIN d'un responsable).
#[derive(Debug, thiserror::Error)]
pub enum Erreur {
    #[error("{0} introuvable")]
    NonTrouve(String),
    #[error("Permission manquante : {0}")]
    Interdit(String),
    /// RG-AUT-03 : l'action est possible avec le PIN d'un responsable.
    #[error("Autorisation d'un responsable requise ({0})")]
    AutorisationRequise(String),
    /// RG-AUT-06 : confirmer la session par le mot de passe de l'utilisateur.
    #[error("Confirmez avec votre mot de passe pour accéder à l'administration")]
    MotDePasseRequis(String),
    #[error("{0}")]
    Validation(String),
    /// Violation d'une règle métier numérotée.
    #[error("{message}")]
    Regle { regle: &'static str, message: String },
    #[error("L'horloge du PC est antérieure au dernier enregistrement ({0}). Corrigez la date de Windows.")]
    HorlogeIncoherente(String),
    #[error("Non connecté")]
    NonAuthentifie,
    #[error("Code PIN incorrect")]
    PinIncorrect,
    #[error("Utilisateur verrouillé, réessayez dans quelques minutes")]
    Verrouille,
    #[error("Base de données : {0}")]
    Base(#[from] rusqlite::Error),
    #[error("Fichier : {0}")]
    Fichier(#[from] std::io::Error),
    #[error("Données : {0}")]
    Json(#[from] serde_json::Error),
}

impl Erreur {
    pub fn regle(regle: &'static str, message: impl Into<String>) -> Self {
        Erreur::Regle { regle, message: message.into() }
    }

    pub fn validation(message: impl Into<String>) -> Self {
        Erreur::Validation(message.into())
    }

    pub fn code(&self) -> &'static str {
        match self {
            Erreur::NonTrouve(_) => "NON_TROUVE",
            Erreur::Interdit(_) => "INTERDIT",
            Erreur::AutorisationRequise(_) => "AUTORISATION_REQUISE",
            Erreur::MotDePasseRequis(_) => "MOT_DE_PASSE_REQUIS",
            Erreur::Validation(_) => "VALIDATION",
            Erreur::Regle { .. } => "REGLE_METIER",
            Erreur::HorlogeIncoherente(_) => "HORLOGE_INCOHERENTE",
            Erreur::NonAuthentifie => "NON_AUTHENTIFIE",
            Erreur::PinIncorrect => "PIN_INCORRECT",
            Erreur::Verrouille => "VERROUILLE",
            Erreur::Base(e) if est_ajout_seul(e) => "AJOUT_SEUL",
            Erreur::Base(_) => "BASE",
            Erreur::Fichier(_) => "FICHIER",
            Erreur::Json(_) => "DONNEES",
        }
    }

    /// Règle métier en cause, si connue.
    pub fn regle_code(&self) -> Option<&'static str> {
        match self {
            Erreur::Regle { regle, .. } => Some(regle),
            Erreur::HorlogeIncoherente(_) => Some("RG-SYS-01"),
            Erreur::AutorisationRequise(_) => Some("RG-AUT-03"),
            Erreur::MotDePasseRequis(_) => Some("RG-AUT-06"),
            Erreur::Base(e) if est_ajout_seul(e) => Some("RG-SYS-03"),
            _ => None,
        }
    }

    pub fn permission(&self) -> Option<&str> {
        match self {
            Erreur::AutorisationRequise(p) | Erreur::Interdit(p) | Erreur::MotDePasseRequis(p) => Some(p),
            _ => None,
        }
    }
}

fn est_ajout_seul(e: &rusqlite::Error) -> bool {
    e.to_string().contains("AJOUT_SEUL")
}

/// Format d'erreur de l'API.
#[derive(Debug, Serialize)]
pub struct ErreurApi {
    pub code: &'static str,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub regle: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission: Option<String>,
}

impl From<&Erreur> for ErreurApi {
    fn from(e: &Erreur) -> Self {
        ErreurApi {
            code: e.code(),
            message: e.to_string(),
            regle: e.regle_code(),
            permission: e.permission().map(str::to_owned),
        }
    }
}
