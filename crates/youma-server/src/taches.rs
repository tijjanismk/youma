//! Tâches de fond : file d'impression, sauvegardes pendant le service, contrôle d'intégrité.

use std::time::Duration;

use youma_core::{impression, journee, parametres, sauvegarde};

use crate::{imprimantes, Etat};

pub fn lancer(etat: Etat) {
    let e = etat.clone();
    tokio::spawn(async move {
        loop {
            imprimer(&e).await;
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    });
    let e = etat.clone();
    tokio::spawn(async move {
        let mut dernier: i64 = 0;
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;
            sauvegarde_periodique(&e, &mut dernier).await;
        }
    });
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(24 * 3600)).await;
            let _ = etat
                .avec_db(|db| sauvegarde::verifier_integrite(db.conn(), true, db.maintenant()))
                .await
                .map(|r| if !r.ok { tracing::error!("Intégrité : {:?}", r.messages) });
        }
    });
}

/// Imprime les tickets en attente. L'envoi réseau se fait hors verrou de la base.
async fn imprimer(e: &Etat) {
    let Ok((jobs, tiroir)) = e
        .avec_db(|db| Ok((impression::jobs_a_imprimer(db.conn())?, parametres::lire(db.conn())?.ouvrir_tiroir)))
        .await
    else {
        return;
    };
    for job in jobs {
        let (dest, contenu) = (job.destination.clone(), job.contenu.clone());
        let tiroir = tiroir && job.type_ == "ticket_client";
        let r = tokio::task::spawn_blocking(move || imprimantes::envoyer(&dest, &contenu, tiroir)).await.unwrap_or_else(|e| Err(e.to_string()));
        let id = job.id.clone();
        let erreur = r.err();
        if let Some(err) = &erreur {
            tracing::warn!("Impression {} en échec : {err}", job.destination);
        }
        let _ = e.avec_db(move |db| impression::marquer_job(db, &id, erreur.as_deref())).await;
        let _ = e.evenements.send(r#"{"type":"impression","id":null}"#.into());
    }
}

/// Sauvegarde toutes les N minutes pendant qu'une journée est ouverte (cahier §20).
async fn sauvegarde_periodique(e: &Etat, dernier: &mut i64) {
    let dossier = e.config.dossier_sauvegardes();
    let d = *dernier;
    if let Ok(Some(t)) = e
        .avec_db(move |db| {
            let intervalle = parametres::lire(db.conn())?.intervalle_sauvegarde_minutes.max(5) * 60_000;
            let maintenant = db.maintenant();
            if journee::ouverte(db.conn())?.is_none() || maintenant - d < intervalle {
                return Ok(None);
            }
            sauvegarde::sauvegarder(db, &dossier, "service")?;
            sauvegarde::rotation(&dossier)?;
            Ok(Some(maintenant))
        })
        .await
    {
        *dernier = t;
    }
}
