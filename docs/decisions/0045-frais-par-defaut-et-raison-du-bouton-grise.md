# 0045 — Frais de livraison par défaut, raison du bouton « Envoyer la commande » grisé

## Contexte

Essai du porteur de projet sur le relais (04/10/2026) : « Envoyer la commande » restait grisé alors que la position
était à 20 m et le point de repère rempli. Le bouton se grise tant qu'il manque une donnée (téléphone de 8 chiffres,
code SMS si la vérification est active, quartier, repère ou position précise, référence Mobile Money en paiement
d'avance), mais rien ne disait laquelle. Il demande aussi des frais de livraison par défaut de 1 000 F.

Avant ce changement, un quartier absent de la liste était livré gratuitement (0 F) par la caisse, et un client en
ligne ne pouvait pas choisir un quartier hors liste.

## Décision

- Sous le bouton grisé : « Pour envoyer, il manque : … », avec chaque donnée manquante. Pour le code SMS, le message
  rappelle de toucher « Recevoir un code par SMS ».
- Nouveau paramètre `frais_livraison_defaut` (1 000 F au départ, Administration → Restaurant et règles → Livraison).
  `Parametres::frais_livraison` donne les frais du quartier (sans tenir compte des majuscules ni des espaces), sinon ces
  frais par défaut. Il sert pour les commandes en ligne et pour celles saisies par le personnel. Un quartier ajouté à la
  liste part de ce montant.
- Menu en ligne : option « Autre quartier (livraison 1 000 FCFA) », puis saisie du quartier.

## Alternatives écartées

- Bouton toujours actif avec message d'erreur après l'envoi : un aller-retour inutile, et le client ne sait pas quoi
  corriger avant d'avoir essayé.
- Refuser les quartiers hors liste : des clients proches mais non listés seraient perdus. La zone à risque reste
  l'outil pour refuser un quartier.

## Conséquences

- Une commande d'un quartier non listé coûte désormais les frais par défaut, au lieu de 0 F. Un restaurant qui livre
  gratuitement met ces frais à 0.
