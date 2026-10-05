# 0051 — Ruptures depuis la prise de commande, menu personnalisé par utilisateur

## Contexte

Demande du porteur de projet (05/10/2026) : « ajoute le bouton rupture sur l'écran de prise de commande et personnaliser
des onglets, ajout et suppression ». Précision donnée : il s'agit des écrans du **menu de l'application** (Salle,
Caisse, Stock, Paie…). Jusqu'ici, une rupture se mettait seulement dans Administration → Catalogue, et le menu montrait
tous les écrans permis par le rôle, dans un ordre fixe.

## Décision

1. **Mode « Ruptures »** sur l'écran de prise de commande, pour qui peut encaisser ou gérer le catalogue (mêmes droits
   que `definir_disponibilite`) : un bouton bascule le mode ; tant qu'il est actif, toucher un plat le met en rupture ou
   le rend disponible (au lieu de l'ajouter), avec un message clair. Les règles ne changent pas (RG-CAT-05, RG-CAT-08 :
   rupture jusqu'à la nouvelle journée ; RG-STK-08 : une rupture mise à la main ne bouge pas avec le stock).
2. **Menu personnalisé** : bouton « Personnaliser » en bas du menu. Chaque utilisateur coche les écrans à afficher et
   les range (monter, descendre), ou revient au « Menu d'origine ». Le choix est gardé **sur l'appareil, par
   utilisateur** (`ui/src/menuPerso.ts`, stockage local) ; il s'applique au menu latéral, à la barre du bas et aux
   tuiles de l'accueil. Un nouvel écran apparaît à la fin, coché.

## Alternatives écartées

- **Bouton « Rupture » sur chaque plat** : trop proche du bouton d'ajout, risque de rupture par erreur en plein service.
- **Menu réglé par le propriétaire pour chaque rôle, sur le poste central** : plus lourd ; chacun range d'abord son
  propre téléphone. À ajouter si le pilote le demande.
- **Cacher un écran = retirer le droit** : un écran caché reste ouvrable (lien, accueil) ; les droits restent ceux du rôle.

## Conséquences

- Le choix du menu ne suit pas l'utilisateur sur un autre appareil, et disparaît si le navigateur efface ses données.
- Un plat mis en rupture depuis la commande l'est partout (salle, QR, en ligne) et c'est journalisé comme avant.
