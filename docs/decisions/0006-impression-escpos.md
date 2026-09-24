# 0006 — Impression ESC/POS avec file persistante

## Contexte
Le ticket cuisine imprimé est le cas principal ; une imprimante en panne ne doit pas faire perdre un envoi.

## Décision
* Le cœur produit le ticket en texte balisé (gros caractères, gras, trait) et l'écrit dans `impressions`
  dans la même transaction que l'envoi.
* Une tâche du serveur imprime la file toutes les 2 s : `tcp:IP:9100` (imprimantes réseau ESC/POS),
  `windows:NOM` (imprimante USB installée, envoi brut par le spouleur), `fichier:CHEMIN` (tests).
* Page de code PC858 pour les accents ; ouverture du tiroir-caisse par l'imprimante (`ESC p`).
* En erreur : 5 tentatives, signalement en caisse et en cuisine, réimpression vers une autre imprimante.

## Alternatives écartées
Impression via le dialogue du navigateur (lent, marges, pas de coupe) ; pilotes propriétaires.

## Conséquences
* **[HYPOTHÈSE]** Les imprimantes du pilote acceptent PC858 : à vérifier sur le matériel réel.
* L'impression Windows brute n'a pas été testée sur matériel (code conditionnel `cfg(windows)`).
