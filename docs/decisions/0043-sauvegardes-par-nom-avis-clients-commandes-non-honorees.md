# 0043 — Sauvegardes par nom (C4), avis des clients, rapport des commandes non honorées

## Contexte

Demande du porteur de projet (03/10/2026) : « corrige C4 et intègre un système d'avis client et un reportage des
commandes non honorées pour des prises de décision ».

- **C4** (revue du 30/09) : `/api/sauvegardes/exporter` et `/api/sauvegardes/restaurer` acceptaient un chemin
  quelconque du serveur ; un appareil appairé, en administration, choisissait où le poste écrivait ou lisait.
- Aucun retour des clients n'était enregistré ; le restaurant ne voyait ni la satisfaction, ni les commandes perdues
  (refusées, livraisons ratées) autrement qu'une par une.

## Décision

- **C4** : plus aucun chemin venu de l'interface.
  - L'export va au **second emplacement** réglé dans Administration → Restaurant et règles (paramètre protégé par le
    mot de passe d'administration) ; sans lui, l'export est refusé avec la marche à suivre.
  - La restauration prend le **nom** d'une sauvegarde (`youma-AAAAMMJJ-HHMMSS-motif.db`, lettres, chiffres, `-_.`,
    pas de `..`), cherché dans le dossier des sauvegardes puis dans le second emplacement. Nouveau PC : régler le
    second emplacement sur la clé USB, la liste « Sur la clé USB » apparaît (`GET /api/sauvegardes/externes`).
- **Avis (RG-AVI-01 à 03)** : table `avis` (migration 0012), une note 1 à 5 et un commentaire par commande, depuis la
  page de suivi (poste : `POST /api/public/avis/{code}` ; relais : même route, transmis au poste à la synchronisation
  et revérifié). Avis figé (déclencheurs) ; seule la suite d'un avis faible s'ajoute, une fois. Écran Clients → Avis,
  ligne « Clients mécontents à rappeler » au tableau de bord, rapport « Avis clients » (moyenne, répartition, canal,
  type, livreur, serveur, commentaires).
- **Commandes non honorées (RG-RAP-04)** : rapport « Non honorées » (Rapports, export CSV) sur les journées
  d'exploitation : refusées, livraisons en échec ou annulées, commandes à distance abandonnées après acceptation ; par
  raison, motif, canal, quartier, heure, livreur, numéros revenant ; articles annulés après envoi ; refus automatiques
  à la réception (journal d'audit). Part des commandes à distance non honorées en indicateur.

## Alternatives écartées

- **C4 : liste blanche de dossiers saisie à chaque export** : revient à laisser choisir un chemin. Le paramètre
  existant, protégé par mot de passe, suffit.
- **Avis par SMS ou WhatsApp** : coût et dépendance ; la page de suivi est déjà ouverte par le client.
- **Avis modifiable** : un avis qui change perd sa valeur pour la décision ; le client peut ajouter un commentaire
  au moment de noter.
- **Compter les additions abandonnées par le personnel** : brouillons vides sans valeur pour la décision ; seules les
  commandes à distance acceptées puis abandonnées comptent.

## Conséquences

- Le guide d'installation et la formation indiquent de régler le second emplacement avant d'exporter ou de restaurer.
- Un avis donné sur le relais n'est possible que tant que le suivi y est publié (commandes récentes).
- **[HYPOTHÈSE]** Montant d'une commande non honorée = articles commandés (prix du moment) + frais de livraison, sans
  remise : c'est un ordre de grandeur du manque à gagner, pas du chiffre d'affaires.
