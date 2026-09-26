# 0034 — Stock saisi par des listes plutôt qu'en texte libre

## Contexte
Le porteur de projet (26/09/2026) : « On n'a pas une vraie gestion de stock ici : utilise des select au lieu de
texte libre si possible. » Les restaurants suivent un stock simple (boissons, quelques ingrédients) ; les champs
libres (unité, famille, conditionnement, motif) donnaient des variantes (« btl », « Bouteille », « bouteilles »)
qui faussent les regroupements et fatiguent la saisie.

## Décision
Listes dans `ui/src/listesStock.ts`, avec le composant existant `ChoixOuAutre` (liste + « Autre… » pour les cas
rares, comme les quartiers, fiche 0021) :
* **Article** : unité de base (bouteille, canette, pièce, sachet, boîte, portion ; g, ml, kg, litre), famille
  (liste + familles déjà utilisées dans le restaurant), conditionnement d'achat (casiers, cartons, packs, sacs,
  bidons). La contenance est **déduite** du conditionnement choisi dans l'unité de base (« Sac de 25 kg » en g →
  25 000), et reste modifiable.
* **Sortie de stock** : motif choisi dans une liste propre au type (casse, périmé, repas du personnel…).
* **Consignes** : nom de l'emballage et motif du mouvement choisis dans une liste.
* Restent en saisie : le **nom** de l'article (propre à chaque restaurant) et les **nombres** (quantités, seuils).
* Le poste central ne change pas : il reçoit les mêmes textes.

## Alternatives écartées
* **Listes fermées, sans « Autre… »** : bloquerait un produit ou un motif imprévu.
* **Listes modifiables dans l'Administration** : plus d'écrans pour un besoin simple ; à reprendre si les
  restaurants le demandent.

## Conséquences
Saisie plus rapide sur téléphone et données homogènes. Les articles existants gardent leurs valeurs ; une valeur
hors liste s'affiche dans « Autre… ».
