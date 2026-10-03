# Module : rapports et statistiques

Rôle : tableau de bord, rapport de période, Z, stock, dettes, statistiques, export CSV. Lecture seule.

## Fichiers
- `crates/youma-core/src/rapports.rs` (905 l.) — `chiffres`, `tableau_de_bord`, `rapport_periode`, `rapport_z`, `rapport_stock`, `ou_part_le_stock`, `rapport_dettes`, `rapport_statistiques` (RG-STA-01..04 l. 745–862), `libelle_mouvement` (libellés remise_coffre, vente_carte_cadeau), `en_csv`, `rapport_non_honorees` (RG-RAP-04 : refusées, livraisons en échec/annulées, abandonnées ; par motif, canal, quartier, heure, livreur, numéro), `LIBELLE_CANAL`/`LIBELLE_TYPE`.
- `crates/youma-core/src/avis.rs` — avis clients (RG-AVI-01..03) : `donner` (code de suivi), `lister`, `traiter` (perm COMMANDE_OFFRIR), `nombre_a_traiter`, `rapport`. Table `avis` (migration 0012, figée par déclencheurs).

## Entrant
api.rs, `impression.rs` (Z), `cloud.rs` (chiffres).

## Base de données
Écrit : rien. Lit : ≈ 20 tables (commandes, paiements, parts_paiement, mouvements_*…).
