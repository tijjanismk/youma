use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use youma_core::erreur::{Erreur, ErreurApi};

/// Erreur renvoyée par l'API : `{ code, message, regle?, permission? }`.
pub struct ApiErreur(pub Erreur);

impl From<Erreur> for ApiErreur {
    fn from(e: Erreur) -> Self {
        ApiErreur(e)
    }
}

impl IntoResponse for ApiErreur {
    fn into_response(self) -> Response {
        let statut = match &self.0 {
            Erreur::NonTrouve(_) => StatusCode::NOT_FOUND,
            Erreur::Interdit(_) | Erreur::AutorisationRequise(_) | Erreur::MotDePasseRequis(_) => StatusCode::FORBIDDEN,
            Erreur::Validation(_) | Erreur::Regle { .. } => StatusCode::UNPROCESSABLE_ENTITY,
            Erreur::HorlogeIncoherente(_) => StatusCode::CONFLICT,
            Erreur::NonAuthentifie | Erreur::PinIncorrect => StatusCode::UNAUTHORIZED,
            Erreur::Verrouille => StatusCode::LOCKED,
            Erreur::Base(_) if self.0.code() == "AJOUT_SEUL" => StatusCode::CONFLICT,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        if statut == StatusCode::INTERNAL_SERVER_ERROR {
            tracing::error!("{}", self.0);
        }
        (statut, Json(ErreurApi::from(&self.0))).into_response()
    }
}

pub type Rep<T> = Result<Json<T>, ApiErreur>;
