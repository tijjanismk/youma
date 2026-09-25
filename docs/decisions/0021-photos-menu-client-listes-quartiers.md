# 0021 — Photos au menu client, menu envoyé au relais s'il change, listes de villes et quartiers

## Contexte
Retours du porteur de projet : les plats n'ont pas d'images côté client, et les saisies de lieux (quartiers de
livraison, zones à risque) sont des champs texte libres, sources de fautes (« Kalaban-Coura », « kalaban coura »…).
Il vise d'abord les restaurants seuls, Bamako en premier, sans fermer la porte aux autres villes.

Constats : la photo d'un plat existait (fiche 0019, URL `data:` d'environ 20 à 30 Ko) mais la page `/menu` ne
l'affichait pas ; et le poste renvoyait **tout le menu, photos comprises, au relais toutes les 10 s** : 40 plats
avec photo, c'est environ 1 Mo toutes les 10 s, plusieurs Go par jour sur une connexion mobile.

## Décision
* **Menu client** : photo ronde du plat (ou icône de sa catégorie, comme en salle).
* **Synchronisation du relais** : le poste envoie toujours `menu_empreinte` (SHA-256 du menu public) et n'envoie le
  `menu` que si le relais n'a pas renvoyé cette empreinte au passage précédent. Le relais garde son menu quand il
  n'en reçoit pas. Un relais qui perd sa base, ou d'une version antérieure, ne renvoie pas l'empreinte : le poste
  renvoie alors le menu. L'état « ouvert » et les prix de l'happy hour font partie du menu : un changement le renvoie.
* **Listes de choix avec « Autre… »** (`composants/Base.tsx`, `ChoixOuAutre`) : la valeur reste un texte libre en
  base, la liste ne fait que proposer. Villes du Mali pour la fiche du restaurant ; quartiers de Bamako groupés par
  commune pour les quartiers de livraison (selon la ville du restaurant) ; quartiers livrés pour les zones à risque.
  Le client en ligne et la prise de commande choisissaient déjà dans les quartiers livrés.
* La liste est dans l'interface (`ui/src/quartiers.ts`) : ce sont des suggestions de saisie, pas une règle métier.
  **[HYPOTHÈSE]** Découpage de Bamako par commune à faire valider ; autres villes : saisie libre pour l'instant.

## Alternatives écartées
* Photos servies à part par le relais (`/api/public/photo/{id}`) : plus économe pour le client aussi, mais demande un
  stockage de fichiers sur le relais ; à reprendre si le poids du menu gêne les clients.
* Table des quartiers en base avec identifiants : rigide (quartiers qui changent de nom, périphérie qui grandit), et
  les commandes gardent de toute façon le nom saisi.
* Carte pour choisir le quartier : exige Internet et des tuiles (fiche 0013).

## Conséquences
Pas de migration. Réponse de `POST /api/relais/synchroniser` : champ `menu_empreinte` ; `menu` devient facultatif.
Test e2e de la zone à risque : le quartier se choisit dans la liste.
