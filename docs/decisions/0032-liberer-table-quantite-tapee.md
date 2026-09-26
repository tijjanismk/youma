# 0032 — Libérer une table d'un geste ; quantité tapée au clavier

## Contexte
Le porteur de projet (26/09/2026) demande la libération rapide des tables, que faire d'une addition abandonnée, et
la saisie libre de la quantité dans la commande. Réponses : bouton « Libérer » dans la Salle ; une addition dont
des plats ont été envoyés doit rester **bloquée** « pour éviter les incohérences » ; toucher « 2× » pour taper la
quantité. Le formulaire d'article du stock convient tel quel.

Jusqu'ici, une table ouverte par erreur (addition vide) ne se libérait qu'en entrant dans l'addition puis
« Plus… » → « Abandonner l'addition vide » ; la quantité ne changeait que par − et +.

## Décision
* **RG-SAL-01** : bouton « Libérer » sous chaque table non libre (Salle). `POST /api/tables/{id}/liberer`
  (`salle::liberer_table`, une transaction) : l'addition ouverte est abandonnée par la même règle que
  « Abandonner l'addition vide » (`commandes::abandonner_op`, RG-CMD-04) ; si des articles ont été envoyés ou un
  paiement enregistré, **refus** avec la marche à suivre (encaisser, ou annuler les articles avec motif et
  responsable). Les marques « réservée » et « à nettoyer » sont retirées. Trace `table.liberer` au journal.
* **Quantité tapée** : sur un article pas encore envoyé, la quantité est un champ (clavier numérique sur
  téléphone) ; on la touche et on tape le nombre, validé par Entrée ou en quittant le champ. Entier de 1 à 999
  (limite déjà imposée par le poste central) ; 0 retire l'article ; toute autre saisie est ignorée. Les boutons
  − et + restent. Un article déjà envoyé ne change pas de quantité (annulation motivée, RG-CMD-04).

## Alternatives écartées
* **Annuler d'un coup une addition envoyée** (« client parti sans payer ») : écarté par le porteur de projet ;
  chaque annulation garde son motif et son responsable.
* **Libérer la table dès l'impression de l'addition** : l'addition resterait ouverte sans table, source d'oublis.
* **Pavé numérique avant le choix du plat** : un geste de plus pour le cas courant (1 ou 2 articles).

## Conséquences
Une table ouverte par erreur se libère sans entrer dans l'addition. Aucune donnée financière ne peut disparaître
par ce bouton : il ne touche qu'aux additions sans envoi ni paiement.
