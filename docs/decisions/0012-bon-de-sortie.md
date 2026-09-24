# 0012 — Pas de facture séparée : le ticket de caisse sert de bon de sortie

## Contexte
Le porteur de projet ne veut pas de numérotation de factures distincte. Il veut une preuve simple que le client
a commandé et payé ce qu'il emporte : il repart avec son ticket de caisse, contrôlé à la sortie.

## Décision
* Quand l'addition est entièrement payée, le ticket porte « TICKET DE CAISSE », « BON DE SORTIE n° » (numéro
  de la commande), « PAYÉ » et un **code de contrôle** de 4 caractères (RG-SOR-01/02).
* Il est imprimé d'office sur l'imprimante de caisse si elle est configurée ; l'écran de reçu affiche aussi
  le numéro et le code (écriture à la main possible sans imprimante).
* Écran « Contrôle de sortie » : numéro + code → PAYÉ (vert), NON PAYÉ ou DÉJÀ PRÉSENTÉ (rouge), avec la
  liste des articles. Chaque présentation est enregistrée ; un code faux et une deuxième présentation sont journalisés (RG-SOR-03).
* Le code est dérivé de l'identifiant d'installation et de la commande (SHA-256), alphabet sans caractères ambigus.

## Alternatives écartées
* Factures numérotées séparées : refusé par le porteur de projet.
* QR code sur le ticket : utile plus tard, mais exige un téléphone pour contrôler ; le code court se lit à l'œil.

## Conséquences
Table `controles_sortie` (ajout seul, migration 0002), permission `sortie.controler`.
