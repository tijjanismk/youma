# 0017 — Promotions et happy hours

## Contexte
Les maquis et bars pratiquent le happy hour (bière moins chère en fin d'après-midi) et des promotions ponctuelles
(fête, match). Le cahier des charges (V2) les demande. Il faut que le prix soit juste sans que le serveur y pense, et
que le propriétaire voie ce que la promotion a coûté.

## Décision
* Table `promotions` : produit ou catégorie, prix fixe ou remise en points de base, plage horaire (même logique que
  les zones à risque, désormais partagée : `horloge::plage_active`), jours, dates facultatives.
* Application **à la saisie de l'article** : `promotions::prix_du_moment` retient le prix le plus bas parmi le prix de
  la zone et les promotions actives. Le prix, la promotion et le prix normal sont copiés sur la ligne ; la fin du happy
  hour ne change pas un article déjà saisi. Même calcul pour le menu QR / en ligne et le recalcul des commandes reçues.
* L'écran de prise de commande affiche le prix du moment et le nom de la promotion (rafraîchi chaque minute).
* Rapport d'activité : tableau « Promotions et happy hours » avec le manque à gagner.

## Alternatives écartées
* Remise appliquée à l'encaissement : le serveur et le client ne voient pas le bon prix pendant le service.
* Offres « 2 achetées, 1 offerte » : demandent un calcul sur plusieurs lignes ; l'offert motivé existe déjà ;
  à ajouter si le pilote le demande **[HYPOTHÈSE]**.

## Conséquences
Migration 0007 (dont `lignes_commande.promotion_id` et `prix_normal`) ; règles RG-PRO-01 à 04 ; onglet
Administration → « Promotions » ; `menu_public` reçoit l'instant de consultation (horloge injectable conservée).
