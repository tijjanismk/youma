# Module : relais Internet (`crates/youma-relais`) et licences (`crates/youma-licence`)

Rôle : serveur public facultatif (HTTPS) : menu en ligne, suivi, livreur, propriétaire, cloud multi-restaurants, SMS.

## Fichiers
- `youma-relais/src/lib.rs` (571 l.) — `Config`, `Etat`, `routeur`, limiteur par adresse et numéro, doublons de commande (`empreinte_panier`, `FENETRE_DOUBLON_MS`), file vers le poste.
- `youma-relais/src/cloud.rs` (258 l.) — restaurants, résumés, sauvegardes chiffrées (RG-CLO-05).
- `youma-relais/src/sms.rs` (166 l.) — Orange Mali ou simulation.
- `youma-licence/src/main.rs` (87 l.) — signature des licences (clé privée hors dépôt).

## Base de données (propre au relais)
Écrit : commandes, modifications, suivis, positions, verifications, etat, cloud_restaurants, cloud_resumes, cloud_sauvegardes.

## Déploiement
Railway (`railway.toml`, `deploiement/railway/Dockerfile`) — instance du porteur : `https://youma-production.up.railway.app/`. Guide : `docs/guides/relais-en-ligne.md`.
