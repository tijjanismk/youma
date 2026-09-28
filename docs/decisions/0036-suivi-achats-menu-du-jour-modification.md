# 0036 — Suivi jusqu'à « Servie », achats intégrés, menu du jour, modification par le client

## Contexte
Retours du porteur de projet (28/09/2026) après essai :
1. le suivi du client s'arrête à « Prête » ;
2. l'achat des produits n'est pas intégré (il faut créer un article de stock à part avant de pouvoir acheter un
   produit revendu), sans perdre l'estimation du coût des achats ;
3. il manque un menu du jour, les boissons et produits obligatoires (eau) restant toujours proposés ;
4. le client doit pouvoir modifier sa commande, avec avertissement, sans abus ;
5. les achats ne se paient pas avec la caisse ; si c'est le cas, il faut le signaler ;
6. un bug à la clôture de caisse.

## Décision
* **Suivi** : l'étape se calcule d'après les lignes de la commande (envoyée → en préparation → prête → servie). « Servi »
  à la cuisine, ou addition réglée avec tout prêt, donne « Servie » (hors livraison). Une commande QR ajoutée à l'addition
  déjà ouverte d'une table garde ses lignes (`lignes_commande.origine_commande_id`) : son suivi avance avec la cuisine et
  n'affiche plus « payé » à tort.
* **Achats (RG-CAT-06)** : un produit « revendu » sans article choisi crée son article de stock dans la même transaction
  (même nom, unité choisie, famille = catégorie, coût de départ = coût d'achat estimé). Chaque achat met ensuite le coût à
  jour (dernier prix d'achat), repris par chaque vente pour le bénéfice : l'estimation devient le coût réel. La fiche du
  produit affiche le coût du dernier achat.
* **Menu du jour (RG-CAT-07)** : case « Plat du jour » sur le produit ; table `menu_du_jour(journee_id, produit_id)`. Un plat
  du jour non coché pour la journée ouverte est absent de la prise de commande, du menu QR et en ligne, et refusé par le
  poste central. Carte « Menu du jour » à l'accueil (caissier ou gérant), avec « Reprendre le menu d'hier ». Sélection vide
  à chaque nouvelle journée.
* **Modification par le client (RG-CAN-06)** : bouton « Modifier ma commande » sur la page de suivi tant que le restaurant
  n'a pas accepté ; le menu s'ouvre avec la commande, un bandeau et une fenêtre d'avertissement (« le restaurant verra que
  vous l'avez modifiée », dernière modification signalée). Deux modifications au plus, mêmes contrôles qu'à la commande
  (disponibilité, menu du jour, plafond du paiement à la livraison). Lignes jamais envoyées : remplacées (RG-CMD-02).
  « Modifiée par le client (n fois) » dans Commandes reçues ; trace au journal. Par le relais : la modification est mise en
  file et transmise au poste comme les positions des livreurs ; le relais n'accepte que si le dernier suivi publié le permet.
* **Achats hors caisse (RG-ACH-05)** : le compte proposé est le coffre (puis banque, Mobile Money) ; le tiroir de la caisse
  reste possible, marqué « déconseillé », avec avertissement. Payé par un tiroir : trace `achat.paye_par_tiroir`, mention
  dans l'historique des achats et alerte dans « Ma journée » → Contrôle.
* **Clôture de caisse** : le calcul et l'enregistrement fonctionnaient (vérifiés sur PC et téléphone) ; deux défauts
  corrigés : le bouton « Imprimer » du rapport Z imprimait tout l'écran (désormais le rapport seul, comme le ticket, fiche
  0028), et le rapport Z ne pouvait pas partir sur l'imprimante de caisse (bouton « Imprimante de caisse »,
  `POST /api/caisse/{id}/z/imprimer`).
* Migration 0009.

## Alternatives écartées
* **Menu du jour par catégorie** (toute la catégorie Plats du jour) : moins souple qu'une case par produit.
* **Modification après acceptation par le client** : la cuisine a déjà reçu le bon ; le changement passe par un serveur
  (annulation motivée, RG-CMD-04).
* **Bloquer les achats payés par le tiroir** : un achat urgent au marché doit rester possible ; il est signalé.

## Conséquences
Chaque matin, composer le menu du jour s'il y a des plats du jour (rappel à l'accueil). Les produits revendus se créent en
un seul écran. Le porteur de projet n'a pas décrit le bug de clôture observé : **[HYPOTHÈSE]** il s'agissait de
l'impression du rapport Z ; à confirmer.
