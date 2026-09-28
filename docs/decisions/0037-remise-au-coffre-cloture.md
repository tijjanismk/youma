# 0037 — Remise au coffre à la clôture de caisse

## Contexte

Retour d'essai : « si je referme la caisse, l'ancien montant compte toujours ». Le tiroir est un compte dont le solde
vient des mouvements (règle non négociable) : clôturer, c'est compter, pas vider. L'argent remis au gérant restait
donc dans le tiroir pour Youma, et la réouverture réclamait un motif d'écart. Le transfert vers le coffre existait
(« Comptes et transferts ») mais personne ne pensait à le faire. C'est probablement le « bug de clôture » signalé
(fiche 0036, **[HYPOTHÈSE]** levée en partie).

## Décision

RG-CAI-15 : l'écran de clôture propose, cochée par défaut s'il existe un compte coffre actif, « Remettre l'argent au
coffre en gardant un fond pour la monnaie ». Fond gardé par défaut : le fond d'ouverture de la session. Le reste de
l'argent compté part au coffre dans la même transaction que la clôture : deux mouvements liés `remise_coffre`
(sortie du tiroir rattachée à la session, entrée au coffre). Le rapport Z affiche « Remis au coffre » et « Fond
laissé en caisse ». Champ `fond_garde` facultatif dans l'API : absent, rien ne change.

## Alternatives écartées

- Remettre le tiroir à zéro à chaque clôture : faux si un fond reste pour la monnaie, et contraire au calcul des
  soldes depuis les mouvements.
- Laisser le transfert manuel : c'est ce qui a produit le retour d'essai.

## Conséquences

- La session suivante n'attend que le fond laissé.
- Pas de permission supplémentaire : la remise est faite par le caissier qui clôture, tracée dans l'audit.
