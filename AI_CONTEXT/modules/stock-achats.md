# Module : stock et achats (stock, achats, recettes, consignes)

Rôle : mouvements de stock en ajout seul, inventaires, réceptions fournisseurs, recettes (coût matière), emballages consignés.

## Fichiers (`crates/youma-core/src/`)
- `stock.rs` (513 l.) — `sortie_vente` (RG-STK-01 l. 180), `retour_annulation` (225), `mouvement_manuel`, inventaires (`creer`/`saisir_comptage`/`valider`), `niveaux`, `contenance`.
- `achats.rs` (291 l.) — fournisseurs, `receptionner` (marché par défaut RG-ACH-01, payé hors caisse par défaut), `regler`, `historique_prix_achat`.
- `recettes.rs` (193 l.) — `definir`, `consommation_unitaire`, `couts_matiere` (RG-REC-04).
- `consignes.rs` (337 l.) — emballages, `a_la_reception`, `retour_fournisseur`, `inventaire`.

## Entrant
`stock` ← commandes (sortie/retour), achats ; `consignes` ← achats ; api.rs.

## Base de données
Écrit : articles_stock (aussi par catalogue — 4 points d'écriture), conditionnements, mouvements_stock, inventaires, lignes_inventaire, achats, lignes_achat, fournisseurs, mouvements_fournisseur, recettes, emballages, mouvements_emballages.

## Règles métier
- [CONFIRMÉ] Retour en stock à l'annulation sauf perte, ingrédients au prorata (RG-STK-02/RG-REC-03) — `stock.rs:225`.
