# 0011 — Mot de passe personnel pour l'administration

## Contexte
Le cahier (§19) demande un mot de passe pour les rôles d'administration ; le porteur de projet l'a confirmé.
Un PIN de 4 chiffres suffit au service mais se voit facilement par-dessus l'épaule au comptoir.

## Décision
* Le PIN reste la seule clé pour vendre, encaisser, cuisiner, gérer le stock et la paie.
* Les permissions d'administration (utilisateurs et rôles, paramètres, licence, sauvegardes/restauration,
  appareils du réseau) exigent une **session confirmée** par le mot de passe personnel (RG-AUT-06).
* Confirmation valable jusqu'à la fin de la session (verrouillage automatique après inactivité, RG-AUT-04).
* Pas d'autorisation ponctuelle par le PIN d'un tiers pour ces actions : c'est le propriétaire du compte qui tape son mot de passe.
* Mot de passe de 6 caractères au moins, haché (Argon2) ; les échecs comptent pour le verrouillage.
* Le propriétaire le choisit à l'installation ; un utilisateur sans mot de passe le crée au premier accès ;
  le propriétaire peut le réinitialiser pour un autre utilisateur.

## Alternatives écartées
* Mot de passe à chaque connexion : trop lent pour le service.
* PIN plus long pour les responsables : toujours visible au comptoir.

## Conséquences
Nouvelle colonne `sessions.eleve` (migration 0002). Code d'erreur `MOT_DE_PASSE_REQUIS` : l'interface demande
le mot de passe puis rejoue l'action.
