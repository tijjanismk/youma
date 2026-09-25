# 0019 — Refonte de l'interface : menu repliable, écrans téléphone, photos des plats

## Contexte
Les écrans du MVP étaient fonctionnels mais austères. Le propriétaire a fourni une référence visuelle (caisse de
restaurant « Zillo ») : menu latéral, cartes de plats avec visuel rond, prix en couleur, stock disponible, ticket à
droite. Sur téléphone, le serveur doit prendre une commande d'une main, sans chercher le ticket.

## Décision
* **Thème unique** en variables CSS (`--accent` orange, fond crème, cartes arrondies), police Poppins **embarquée**
  (`@fontsource/poppins`) et icônes `lucide-react` : rien n'est chargé depuis Internet (fonctionnement hors ligne).
* **PC : menu latéral repliable** (icônes seules, 92 px), choix mémorisé par poste (`localStorage`, confort seulement).
* **Téléphone** : barre du bas (Accueil + 3 écrans prioritaires selon les permissions + « Autres écrans ») ; en prise
  de commande, les plats occupent l'écran (2 colonnes) et la commande monte en tiroir via « Voir la commande (n) ».
* **Stock disponible sur la carte du plat** : `catalogue::disponibles` (article revendu = solde ; recette = nombre de
  portions réalisables, minimum sur les ingrédients). Calcul côté backend, depuis les mouvements.
* **Photo du plat** choisie dans l'administration, recadrée et compressée dans le navigateur (JPEG 320 px, ≈ 20–30 Ko),
  stockée dans le champ `photo` existant (URL `data:`). Sans photo : icône de la catégorie sur un cercle teinté.

## Alternatives écartées
* Polices et icônes via CDN : cassent hors ligne.
* Stockage des photos en fichiers servis par le poste central : sauvegarde et synchronisation plus complexes pour un
  gain faible à cette taille.
* Application mobile séparée : le même écran web adaptatif suffit et reste à jour avec le poste central.

## Conséquences
* Le catalogue renvoyé aux postes grossit avec les photos (≈ 25 Ko par plat photographié). **[HYPOTHÈSE]** acceptable
  sur le Wi-Fi local pour une carte de moins de 100 plats.
* Les libellés accessibles (aria-label) sont conservés : les tests de bout en bout restent valables ; un test vérifie
  le tiroir de commande sur téléphone.
