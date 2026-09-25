# 0008 — Synchronisation cloud par outbox (V2)

## Décision
* Chaque opération métier ajoute une ligne `outbox` dans sa transaction (entité, identifiant, opération).
* L'envoi au cloud (V2) sera idempotent (clé = id outbox) et reprendra après coupure.
* Données financières en ajout seul : aucune résolution de conflit nécessaire.
* Données de référence modifiées à distance : demande de modification appliquée par le poste central
  si la `version` correspond, sinon rejetée et signalée (le local gagne).

## État
Outbox alimentée. Le QR menu et les commandes distantes passent par le relais (fiche 0013) ; le cloud
(fiche 0018) reçoit des résumés de journée et des sauvegardes chiffrées plutôt qu'une réplication ligne à ligne :
la réplication complète par l'outbox reste possible plus tard, sans changer le poste.
