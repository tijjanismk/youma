# Module : rapports et statistiques

Rôle : tableau de bord, rapport de période, Z, stock, dettes, statistiques, export CSV. Lecture seule.

## Fichiers
- `crates/youma-core/src/rapports.rs` (905 l.) — `chiffres`, `tableau_de_bord`, `rapport_periode`, `rapport_z`, `rapport_stock`, `ou_part_le_stock`, `rapport_dettes`, `rapport_statistiques` (RG-STA-01..04 l. 745–862), `libelle_mouvement` (libellés remise_coffre, vente_carte_cadeau), `en_csv`.

## Entrant
api.rs, `impression.rs` (Z), `cloud.rs` (chiffres).

## Base de données
Écrit : rien. Lit : ≈ 20 tables (commandes, paiements, parts_paiement, mouvements_*…).
