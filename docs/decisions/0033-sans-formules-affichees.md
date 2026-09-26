# 0033 — Plus de formules de calcul affichées dans l'application

## Contexte
Le porteur de projet (26/09/2026) demande d'enlever « Comment est-ce calculé ? » du tableau de bord, puis « les
formules de calcul dans toute l'app ».

**Contradiction avec le cahier des charges**, signalée ici : § « Tableau de bord » (« bénéfice estimé, formule
affichée ») et § 18 (« Définir chaque indicateur par une formule écrite, affichée dans le rapport […] Un
propriétaire qui ne sait pas comment un chiffre est calculé finit par ne plus le croire »), repris par RG-RAP-01.
Le porteur de projet tranche : les écrans restent sans formule.

## Décision
* Retirés de l'interface : la formule sous chaque carte de « Ma journée » et des Rapports (indicateurs, tableaux,
  coût matière), dans la fiche Recette, les Consignes, les Achats (consigne), « Comment la paie est-elle
  calculée ? » (Paie) et « quantité × dernier prix d'achat » (valeur du stock). Les cartes d'indicateurs des
  rapports ne s'ouvrent plus.
* Export CSV des rapports : deux colonnes (libellé ; valeur), sans formule.
* Le poste central **garde** la formule écrite de chaque indicateur (`rapports.rs`, champ `formule` de l'API) :
  c'est la définition du calcul, utile au support et pour la remettre à l'écran si un restaurant la demande.
  RG-RAP-01 est reformulée en ce sens.

## Alternatives écartées
* **Supprimer aussi les formules du code et de l'API** : on perd la définition écrite des indicateurs, qui sert à
  répondre à un propriétaire qui conteste un chiffre.
* **Réglage par restaurant (afficher / masquer)** : non demandé ; à reprendre si des propriétaires réclament les
  formules.

## Conséquences
Écrans plus courts. En cas de doute sur un chiffre, l'explication vient du fournisseur (formules dans le code) ou
du détail des opérations (journal, historique du stock, relevés), plus de l'écran lui-même.
