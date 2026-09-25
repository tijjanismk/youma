# 0016 — Rapprochement Mobile Money par relevé d'opérateur

## Contexte
Les faux SMS et les captures d'écran réutilisées sont la fraude Mobile Money la plus courante. La vérification une à
une (écran « À vérifier ») est lente. Les espaces marchands (Orange Money, Moov Money, Wave, Sama Money) permettent
d'exporter un relevé ; le cahier des charges (§12, V2) demande de rapprocher les paiements avec ce relevé.

## Décision
* Import d'un fichier CSV par compte Mobile Money, **hors ligne** (fichier téléchargé puis chargé sur le poste).
* Lecture tolérante : séparateur deviné (`;`, `,`, tabulation), colonnes reconnues par leur nom (français ou anglais),
  débits ignorés, montants ramenés en FCFA entiers, dates au format jj/mm/aaaa ou aaaa-mm-jj (heure du Mali = UTC).
* Lignes du relevé conservées en ajout seul, uniques par (compte, référence) : un relevé rechargé ou des relevés qui
  se chevauchent ne comptent rien deux fois.
* Rapprochement par référence ; le montant doit être identique pour vérifier. Le bilan donne les vérifiés, les
  écarts de montant, les lignes inconnues en caisse et les paiements absents du relevé.

## Alternatives écartées
* API des opérateurs en direct : contrats marchands nécessaires, Internet obligatoire ; à envisager via le relais.
* Rapprochement sur montant et heure sans référence : trop d'homonymes (beaucoup de paiements de même montant).

## Conséquences
Migration 0006 ; règles RG-RMM-01 à 05 ; onglet « Relevé de l'opérateur » dans l'écran Mobile Money.
**[HYPOTHÈSE]** Noms de colonnes des relevés réels à confirmer sur des exports Orange Money, Moov et Wave ; la liste
des noms reconnus s'étend sans changer le reste.
