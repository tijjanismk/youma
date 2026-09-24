//! Événements temps réel : nouvelle commande, envoi, commande prête, paiement, table…
//! Le navigateur ne peut pas poser d'en-tête sur un WebSocket : le jeton passe en paramètre.

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::response::Response;
use futures_util::{SinkExt, StreamExt};
use std::collections::HashMap;
use youma_core::auth;

use crate::erreurs::ApiErreur;
use crate::{Etat, Poste};

pub async fn ws(
    State(e): State<Etat>,
    _poste: Poste,
    Query(p): Query<HashMap<String, String>>,
    upgrade: WebSocketUpgrade,
) -> Result<Response, ApiErreur> {
    let jeton = p.get("jeton").cloned().unwrap_or_default();
    e.avec_db(move |db| auth::verifier_session(db, &jeton)).await?;
    let rx = e.evenements.subscribe();
    Ok(upgrade.on_upgrade(move |socket| relayer(socket, rx)))
}

async fn relayer(socket: WebSocket, mut rx: tokio::sync::broadcast::Receiver<String>) {
    let (mut envoi, mut reception) = socket.split();
    let _ = envoi.send(Message::Text(r#"{"type":"connecte","id":null}"#.into())).await;
    loop {
        tokio::select! {
            ev = rx.recv() => match ev {
                Ok(texte) => {
                    if envoi.send(Message::Text(texte.into())).await.is_err() {
                        break;
                    }
                }
                // Client trop lent : il se resynchronise en rechargeant.
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                    let _ = envoi.send(Message::Text(r#"{"type":"resynchroniser","id":null}"#.into())).await;
                }
                Err(_) => break,
            },
            msg = reception.next() => match msg {
                Some(Ok(Message::Ping(p))) => { let _ = envoi.send(Message::Pong(p)).await; }
                Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                _ => {}
            },
        }
    }
}
