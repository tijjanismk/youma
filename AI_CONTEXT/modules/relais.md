# Module : relais Internet (`crates/youma-relais`) et licences (`crates/youma-licence`)

Rôle : serveur public facultatif (HTTPS) : menu en ligne, suivi, livreur, propriétaire, cloud multi-restaurants, SMS.

## Fichiers
- `youma-relais/src/lib.rs` (571 l.) — `Config`, `Etat`, `routeur`, limiteur par adresse et numéro, RG-CAN-08 sur suivi et position (15 codes inconnus / 15 min, 429), doublons de commande (`empreinte_panier`, `FENETRE_DOUBLON_MS`), file vers le poste.
- `youma-relais/src/cloud.rs` (258 l.) — restaurants, résumés, sauvegardes chiffrées (RG-CLO-05).
- `youma-relais/src/sms.rs` — codes par SMS (Orange Mali ou simulation) ou WhatsApp Cloud (`WhatsApp`, `Canal`), au choix du client ; `Envoyeur::canaux` (fiche 0046).
- Numéro vérifié une fois (fiche 0046) : `POST /api/public/verification/confirmer` → `jeton_client` (empreinte dans `clients_verifies`, 180 jours après la dernière commande) ; `commande` accepte `jeton_client` ou `code_verification`.
- `youma-licence/src/main.rs` — clés et licences ; clé privée par `--cle-privee-fichier` ou `YOUMA_CLE_PRIVEE` (guide `docs/guides/licences.md`).

## Base de données (propre au relais)
Écrit : commandes, modifications, suivis, positions, verifications, clients_verifies, avis, etat, cloud_restaurants, cloud_resumes, cloud_sauvegardes.

## Déploiement
Railway (`railway.toml`, `deploiement/railway/Dockerfile`) — instance du porteur : `https://youma-production.up.railway.app/`. Guide : `docs/guides/relais-en-ligne.md`.
