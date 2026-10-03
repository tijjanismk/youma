# Module : canaux à distance (QR, en ligne, livraison, zones à risque, cloud)

Rôle : recevoir les commandes des clients (QR de table, menu en ligne via le relais), file de validation, suivi, livreurs, positions.

## Fichiers (`crates/youma-core/src/`)
- `entrantes.rs` (959 l.) — `menu_public`, `recevoir` (RG-CAN-01..05, RG-ZON-02, l. 206), `empreinte_panier` + `FENETRE_DOUBLON_MS` (RG-CAN-07, l. 230 ; contrôle après la liste noire), `modifier` (RG-CAN-06), `file` (VIP d'abord, RG-VIP-02 l. 556), `valider`, `suivi`, `ajouter_position` (RG-LIV-04 l. 867), `liens`, codes QR.
- `zones_risque.rs` (239 l.) — `evaluer` (quartier et/ou cercle GPS, `distance_m`), liste noire de numéros, `normaliser_telephone`.
- `livraison.rs` (179 l.) — `assigner`, `changer_statut`, `remise_livreur` (RG-LIV-03), `positions_en_cours`.
- `cloud.rs` (241 l.) — résumé de journée, chiffrement XChaCha20 des sauvegardes, texte SMS.

## Entrant
api.rs (routes `/public/*`, `/entrantes`, `/livraisons/positions`) ; `youma-server/src/relais.rs` (synchronisation 10 s) ; `youma-relais` réutilise `empreinte_panier`.

## Base de données
Écrit : commandes, lignes_commande, tables_salle, positions_livreur, zones_risque, numeros_bloques, comptes_tresorerie (livraison), systeme (cloud).

## Règles métier
- [CONFIRMÉ] Refus journalisés sans rien créer — `entrantes.rs:206`.
- [CONFIRMÉ] `zones_risque::evaluer` ne reçoit pas la précision GPS (constat L3, `ETUDE-MOBILE.md`) ; la précision ne sert qu'au client pour rendre le repère facultatif (≤ 100 m, `ui/src/public/MenuClient.tsx:390`).
