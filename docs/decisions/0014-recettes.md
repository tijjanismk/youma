# 0014 — Recettes et consommation théorique

## Contexte
Le cahier des charges (§9, V2) demande des recettes facultatives : vendre un plat sort ses ingrédients du stock,
pour comparer la consommation théorique à l'inventaire réel. Les boissons restent prioritaires (suivi à l'unité,
déjà en place) ; les recettes ne doivent jamais être obligatoires. Les montants et quantités sont des entiers.

## Décision
* Table `recettes` : une ligne appartient à un produit (plat de base) **ou** à une option (supplément). Quantité
  entière dans l'unité de base de l'article. Pour rester en entiers, un ingrédient se gère dans une **petite unité**
  (g, ml, pièce) ; les conditionnements font la conversion à l'achat (sac de 25 kg = 25 000 g, bidon de 20 L = 20 000 ml).
* Vente : `stock::sortie_vente` sort chaque ingrédient (plat + options choisies, lues dans la copie JSON de la ligne)
  au moment de l'envoi, comme un article revendu. Le coût d'une unité (Σ quantité × coût unitaire) est copié sur la
  ligne : le bénéfice estimé l'utilise.
* Annulation sans perte : retour au prorata calculé sur ce qui est **réellement sorti** pour cette ligne (mouvements
  de stock), pas sur la recette actuelle — une recette modifiée entre-temps ne fausse pas le retour.
* Les recettes des options sont préservées quand on réenregistre le produit (les options gardent leurs identifiants) ;
  la recette d'une option supprimée disparaît avec elle.
* Coût matière par plat (rapport « Coût matière ») avec sa formule et sa part du prix.

## Alternatives écartées
* Quantités décimales (0,25 kg) : contraire à la règle des entiers ; les petites unités donnent la même précision.
* Sortie des ingrédients au paiement : l'envoi est le moment où la cuisine consomme (cohérent avec RG-STK-01).
* Recette recalculée à l'annulation : fausse si la recette a changé depuis la vente.

## Conséquences
Migration 0004 ; règles RG-REC-01 à 04 ; écran « Recette » depuis Administration → Produits ; rapport « Coût matière ».
Démonstration : Frites (250 g de pommes de terre, 30 ml d'huile ; option « Grande » + 150 g).
