# Formation du personnel — Youma

Pour former l'équipe le jour de la mise en route (environ 1 h, sur la vraie caisse, avec la base du restaurant).
Chaque partie tient sur une page : l'imprimer et la laisser au poste concerné. Version illustrée à imprimer :
`formation/formation-personnel.pdf` (captures d'écran, produite par `cd ui && node outils/formation.mjs`).

> **À retenir pour tout le monde**
> - Chacun a **son propre code PIN** (4 à 6 chiffres). On ne le prête jamais : tout ce qui est fait avec votre PIN
>   est enregistré à votre nom.
> - On travaille **sans Internet** : si Internet coupe, on continue de vendre.
> - 5 codes faux de suite bloquent la personne 5 minutes.

---

## 1. Tout le monde : se connecter

1. Toucher **son nom** sur l'écran de connexion.
2. Taper **son code PIN**, puis **Entrer**.
3. En quittant le poste (pause, fin de service) : **Changer d'utilisateur** (en haut à droite), pour que le suivant
   ne travaille pas sous votre nom.

Si l'écran affiche **« Poste central injoignable »** sur un téléphone : continuer la saisie, elle est gardée ;
elle partira toute seule quand la caisse sera rallumée ou le Wi-Fi revenu. Prévenir le gérant si ça dure.

---

## 2. Serveur ou serveuse : prendre une commande

**À table**
1. Menu **Salle** : toucher la table (vert = **Libre**, orange = **Occupée**).
2. Choisir la catégorie (Boissons, Grillades…), puis toucher les plats. Toucher deux fois = 2 articles.
3. Pour une grande quantité, toucher le nombre à côté du plat (« 1× ») et **taper la quantité** (ex. 24).
4. Toucher le nom d'un plat pour ajouter une **option** ou une **note** (« sans piment »).
5. **Envoyer** : les bons partent en cuisine ou au bar.
6. Pour ajouter une tournée plus tard : revenir sur la même table, ajouter, **Envoyer**.

Sur téléphone, la commande est en bas : **Voir la commande (n)**.

**Quand la cuisine a fini** : la table affiche **Prêt**. Aller chercher et servir.

**Emporter ou livraison** : Salle → **+ Emporter / livraison**, puis comme à table. En livraison, choisir le
quartier et noter le téléphone du client.

**Erreurs fréquentes**
- Un article **pas encore envoyé** : le retirer avec **−** ou taper 0.
- Un article **déjà envoyé** : le toucher → annuler, donner le **motif** ; le **PIN du gérant** est demandé.
- Une table ouverte par erreur (rien d'envoyé) : dans la Salle, bouton **Libérer** sous la table.
  Si des plats ont été envoyés, Youma refuse : il faut encaisser, ou faire annuler les articles par le gérant.

**Addition au client** : dans la commande, **Plus…** → **Addition / ticket** (imprimer ou envoyer par WhatsApp).

---

## 3. Caisse : encaisser

**En début de service**
1. Menu **Caisse** → compter l'argent du tiroir → **Fond de caisse compté** → **Ouvrir ma session**.
   S'il y a une différence avec le fond prévu, donner le **motif de l'écart**.

**Encaisser une addition**
1. Ouvrir la commande (Salle ou Caisse) → **Encaisser …**.
2. Toucher le moyen de paiement :
   - **Espèces** : taper ou toucher le billet reçu dans **Espèces reçues du client** ; Youma affiche la **monnaie
     à rendre**.
   - **Orange Money / Moov / Wave** : taper la **référence de la transaction** lue sur le téléphone du client
     (et son numéro si possible). Une même référence ne peut servir qu'une fois.
   - **Carte (TPE)** : passer la carte sur le TPE, puis taper le **numéro d'autorisation** imprimé sur le ticket
     du TPE.
   - **Crédit** : seulement pour un client connu, autorisé par le gérant.
   - Un client peut payer en plusieurs fois ou avec plusieurs moyens : toucher un second moyen, répartir.
3. **Valider le paiement**, puis **Imprimer le ticket (bon de sortie)** : le client le garde, il le montre à la
   sortie.

**Dépenses et argent sorti du tiroir** (charbon, glace, taxi…) : Caisse → **Nouvelle dépense**, avec le
bénéficiaire. Jamais d'argent sorti du tiroir sans l'enregistrer. Les **achats de marchandises** se paient hors
caisse (coffre) : payés par le tiroir, ils sont signalés au propriétaire.

**Carte cadeau, société, points de fidélité**
- **Carte cadeau ou bon d'avoir** : à l'encaissement, **Carte cadeau / bon d'avoir**, taper le code, **Vérifier le solde**.
  Le solde baisse du montant payé ; le client peut compléter avec un autre moyen.
- **Société sous contrat** : **Société : …**, taper le **nom de l'employé**. La société paie sa part (inscrite sur son
  compte, à facturer), l'employé paie le reste.
- **Points de fidélité** : choisir le client sur l'addition, puis **Points de fidélité** : les points deviennent une réduction.
- **Vendre une carte cadeau** : Clients → Cartes cadeaux → **Vendre une carte cadeau**, puis remettre la carte imprimée.

**En fin de service**
1. Caisse → **Clôturer ma caisse**.
2. Compter l'argent : **Compter billet par billet** (le plus sûr) ou taper le total.
3. S'il y a un écart, le **motif est obligatoire**.
4. **Fond gardé dans le tiroir** : laisser la monnaie pour le prochain service (le fond du matin, proposé) ;
   le reste est remis au coffre.
5. Imprimer le **Rapport de clôture (Z)** et le remettre au gérant avec l'argent remis au coffre.

---

## 4. Cuisine, grill, bar : l'écran de préparation

1. Menu **Cuisine / Bar** : les bons arrivent tout seuls, les plus anciens en premier.
2. **En préparation** quand on commence, **Prêt** quand c'est fini : le serveur voit « Prêt » sur sa table.
3. **Servi** quand le plat est parti.
4. Un produit manque (« plus de poulet ») : bouton **Problème**, puis écrire ce qui manque : le serveur est prévenu.

Les bons s'impriment aussi à l'imprimante du poste, si elle est installée.

---

## 5. Contrôle de sortie (vigile ou caissier)

1. Menu **Contrôle de sortie** : taper le **N° du bon de sortie** et le **Code de contrôle** imprimés sur le ticket.
2. Réponses :
   - **PAYÉ — peut sortir** : laisser passer ;
   - **NON PAYÉ** : renvoyer à la caisse ;
   - **DÉJÀ PRÉSENTÉ** : ce ticket a déjà servi, prévenir le gérant.

---

## 6. Gérant et propriétaire

**Ouvrir et clôturer la journée**
- Le matin : Accueil → **Ouvrir la journée**. Sans journée ouverte, personne ne peut vendre.
- Le soir, quand toutes les caisses sont clôturées : Accueil → **Clôturer la journée** (Youma fait une sauvegarde).
  Si le bouton est grisé, la raison est écrite dessous (addition ouverte, caisse ouverte, commande à accepter).
- Après l'ouverture : **Menu du jour** (carte de l'accueil) → cocher les plats du jour proposés aujourd'hui
  (« Reprendre le menu d'hier » si rien ne change). Boissons et eau restent toujours proposées.

**Client mécontent, client privilégié** : Clients → Cartes cadeaux → **Offrir un bon d'avoir** (motif obligatoire, PIN du
gérant). Fiche du client → **Rendre privilégié** : ses commandes à distance passent en tête ; les tables restent par
ordre d'arrivée.

**Ce qui demande le PIN du gérant** : annuler un article envoyé, une remise au-delà du plafond, certains retraits.
Le gérant tape son PIN **lui-même** sur l'écran : ne jamais le donner au personnel.
Après **5 PIN faux en 15 minutes**, les autorisations sont bloquées 5 minutes sur ce compte (le gérant, lui, n'est pas
bloqué : il peut faire l'action depuis sa propre session).

**Chaque jour**
- **Ma journée** : chiffre d'affaires, dépenses, bénéfice estimé, annulations, remises, stock critique ;
  **Résumé par WhatsApp** pour l'envoyer au propriétaire.
- **Mobile Money** : vérifier les paiements marqués « à vérifier » avec le relevé de l'opérateur.
- **Stock** → **Perte / sortie** pour une casse, un produit périmé, un repas du personnel (motif choisi dans la
  liste).

**Administration** (mot de passe personnel en plus du PIN) : produits et prix, tables, utilisateurs et PIN,
moyens de paiement, imprimantes, sauvegardes, licence.
- Mot de passe oublié : **Mot de passe oublié ?** dans la fenêtre du mot de passe, puis le **code de secours**
  noté à l'installation (ou appeler le fournisseur).

---

## Exercice de fin de formation (15 min)

À faire par chacun, sur la vraie caisse, avant l'ouverture au public :

1. Se connecter avec son PIN, puis **Changer d'utilisateur**.
2. Serveur : une table, 2 boissons et 1 plat, **Envoyer** ; ajouter une tournée ; annuler un article (avec le
   gérant).
3. Cuisine : passer le bon en **Prêt** puis **Servi**.
4. Caisse : encaisser une partie en espèces (vérifier la monnaie), le reste en Mobile Money (référence d'essai) ;
   imprimer le bon de sortie.
5. Sortie : contrôler ce bon (**PAYÉ**), puis le contrôler une seconde fois (**DÉJÀ PRÉSENTÉ**).
6. Gérant : clôturer la caisse d'essai et lire le rapport Z.

Les ventes d'essai restent dans l'historique : les faire sur une journée d'essai, avant l'ouverture, et le noter
au gérant.
