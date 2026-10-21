# Module : stock et achats (stock, achats, recettes, consignes)

Rôle : mouvements de stock en ajout seul, inventaires, réceptions fournisseurs, recettes (coût matière), emballages consignés.

## Fichiers (`crates/youma-core/src/`)
- `stock.rs` (540 l.) — `inserer_mouvement` (l. 147, passage obligé de tout mouvement) → `rupture_selon_stock` (l. 185, RG-STK-08), `sortie_vente` (RG-STK-01), `retour_annulation` (225), `mouvement_manuel`, inventaires (`creer`/`saisir_comptage`/`valider`), `niveaux`, `contenance`.
- `achats.rs` (291 l.) — fournisseurs, `receptionner` (marché par défaut RG-ACH-01, payé hors caisse par défaut), `regler`, `historique_prix_achat`.
- `recettes.rs` (193 l.) — `definir`, `consommation_unitaire`, `couts_matiere` (RG-REC-04).
- `consignes.rs` (337 l.) — emballages, `a_la_reception`, `retour_fournisseur`, `inventaire`.

## Entrant
`stock` ← commandes (sortie/retour), achats ; `consignes` ← achats ; api.rs.

## Base de données
Écrit : produits (disponible, rupture_auto, par RG-STK-08), articles_stock (aussi par catalogue — 4 points d'écriture), conditionnements, mouvements_stock, inventaires, lignes_inventaire, achats, lignes_achat, fournisseurs, mouvements_fournisseur, recettes, emballages, mouvements_emballages.

## Règles métier
- [CONFIRMÉ] Rupture auto au passage du stock de > 0 à ≤ 0, levée au retour > 0 ; `produits.rupture_auto` (migration 0014) distingue la rupture de stock de la rupture manuelle (RG-STK-08) — `stock.rs:185`, `catalogue.rs:519`.
- [CONFIRMÉ] Retour en stock à l'annulation sauf perte, ingrédients au prorata (RG-STK-02/RG-REC-03) — `stock.rs:225`.
