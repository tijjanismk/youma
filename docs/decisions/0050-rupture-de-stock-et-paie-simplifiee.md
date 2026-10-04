# 0050 — Rupture automatique quand le stock tombe à 0, paie simplifiée

## Contexte

Demandes du porteur de projet (04/10/2026) :

- « mets en rupture automatiquement quand le stock tombe à 0 » ;
- « clôture veut dire quoi dans la paie ? il faut une gestion de paie simplifiée ».

Jusqu'ici, un produit revendu (boisson) restait proposé à stock nul (RG-STK-07 : stock négatif permis, vente non
bloquée). La paie se faisait en deux temps par employé : **Clôturer** (le salaire de la période est calculé et figé
dans un bulletin, RG-PAI-03/06) puis **Payer** le bulletin (RG-PAI-05), un mot comptable mal compris.

## Décision

1. **RG-STK-08** : à chaque mouvement de stock (`stock::inserer_mouvement`, passage obligé), si le stock d'un article
   **passe** de positif à 0 ou moins, ses produits revendus actifs passent en rupture (`produits.rupture_auto = 1`,
   migration 0014) ; s'il redevient positif, ces ruptures-là sont levées. Journalisé (`produit.rupture_stock`,
   `produit.retour_en_stock`).
   - Seul le **passage** compte : un restaurant qui ne saisit pas ses achats (stock jamais positif) vend comme avant
     (RG-STK-07 reste vrai pour lui).
   - Une rupture mise à la main (`definir_disponibilite`) remet `rupture_auto` à 0 : le stock ne la lève pas.
   - La nouvelle journée (RG-CAT-08) lève les ruptures du jour, pas celles de stock.
2. **RG-PAI-10, paie simplifiée** : `paie::payer_periode` arrête le salaire de la période et le paie dans **une seule
   transaction** (refus du compte = rien n'est arrêté). L'écran Paie montre un bouton **Payer** par employé (montant
   partiel possible, ou « Enregistrer sans payer »), **Tout payer** (un même compte pour tous), des périodes toutes
   faites (ce mois, mois dernier, cette semaine, aujourd'hui) et une phrase qui explique ce qui se passe. Le mot
   « Clôturer » disparaît de l'écran ; le bulletin figé et ses règles (RG-PAI-03 à 09) ne changent pas.

## Alternatives écartées

- **Bloquer la vente à stock nul pour tous** : les restaurants qui ne saisissent pas leurs achats ne pourraient plus
  vendre leurs boissons.
- **Rupture selon les ingrédients d'une recette** : un plat dépend de plusieurs ingrédients et de quantités
  approximatives ; les ruptures se décident encore à la main (à revoir après le pilote).
- **Supprimer le bulletin** (payer directement le compte employé) : on perdrait la pièce signée et la période figée
  qui protègent employeur et employé.

## Conséquences

- Une boisson finie disparaît du menu (salle, QR, en ligne) à la vente de la dernière bouteille ; l'écran du serveur
  la montre en rupture au prochain rafraîchissement (l'ajout est de toute façon refusé, RG-CAT-05).
- Un inventaire qui trouve 0 met aussi en rupture ; un achat reçu la lève.
- « Tout payer » enchaîne un paiement par employé : si l'un échoue (compte insuffisant…), les précédents restent payés
  et le message dit lequel a échoué.
