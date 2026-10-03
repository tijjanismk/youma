# Module : ventes (commandes, salle, catalogue, promotions, impression, sortie)

Rôle : prise de commande, envoi en cuisine par poste, annulations, remises, plan de salle, carte des plats, tickets.

## Fichiers (`crates/youma-core/src/`)
- `commandes.rs` (1140 l.) — `ouvrir(_op)`, `ajouter_lignes(_op)`, `envoyer(_op)` (RG-CMD-03, l. 607), `annuler_ligne`, `remise`, `offrir`, `transferer`, `fusionner`, `abandonner`, `totaux`, `etat`, `exiger_ouverte`/`toucher` (pub(crate)), `envois_actifs` (VIP d'abord, RG-VIP-03 l. 1017), `diviser_parts_egales` (RG-CMD-12), `definir_client`.
- `salle.rs` (210 l.) — zones, tables, `plan`, `marquer_table`, `liberer_table`, `zone_de_table`.
- `catalogue.rs` (676 l.) — postes, catégories, produits, options, `prix_effectif` (prix par zone), menu du jour (RG-CAT-07), `importer_csv`, `disponibles`.
- `promotions.rs` (211 l.) — `prix_du_moment` (utilisé par commandes et entrantes), `bilan`.
- `impression.rs` (461 l.) — rendu ESC/POS, file `impressions`, ticket client, Z.
- `sortie.rs` (94 l.) — code de contrôle du ticket à la sortie (RG-SOR).

## Entrant
`commandes.rs` ← caisse, contrats, employes, entrantes, api.rs. `promotions::prix_du_moment` ← commandes, entrantes.

## Base de données
Écrit : commandes, lignes_commande, envois, annulations, remises, tables_salle, zones, produits, categories, options, groupes_options, prix_zone, historique_prix, menu_du_jour, postes_preparation, promotions, impressions, controles_sortie.

## Règles métier
- [CONFIRMÉ] Article envoyé : annulation avec motif seulement (RG-CMD-04) — `commandes.rs:593`.
- [CONFIRMÉ] Zones à risque appliquées aussi aux commandes du personnel (RG-ZON-03) — `commandes.rs:462`.
- [CONFIRMÉ] Écart d'arrondi des parts égales sur la dernière part (RG-CMD-12) — `commandes.rs:1092`.
