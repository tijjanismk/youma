# Module : relais Internet (`crates/youma-relais`) et licences (`crates/youma-licence`)

Rôle : serveur public facultatif (HTTPS) : menu en ligne, suivi, livreur, propriétaire, cloud multi-restaurants, SMS.

## Fichiers
- `youma-relais/src/lib.rs` (571 l.) — `Config`, `Etat`, `routeur`, limiteur par adresse et numéro, RG-CAN-08 sur suivi et position (15 codes inconnus / 15 min, 429), doublons de commande (`empreinte_panier`, `FENETRE_DOUBLON_MS`), file vers le poste.
- `youma-relais/src/cloud.rs` (258 l.) — restaurants, résumés, sauvegardes chiffrées (RG-CLO-05).
- `youma-relais/src/sms.rs` (166 l.) — Orange Mali ou simulation.
- `youma-licence/src/main.rs` — clés et licences ; clé privée par `--cle-privee-fichier` ou `YOUMA_CLE_PRIVEE` (guide `docs/guides/licences.md`).

## Base de données (propre au relais)
Écrit : commandes, modifications, suivis, positions, verifications, etat, cloud_restaurants, cloud_resumes, cloud_sauvegardes.

## Déploiement
Railway (`railway.toml`, `deploiement/railway/Dockerfile`) — instance du porteur : `https://youma-production.up.railway.app/`. Guide : `docs/guides/relais-en-ligne.md`.
