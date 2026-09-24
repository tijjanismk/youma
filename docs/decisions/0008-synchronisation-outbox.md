# 0008 — Synchronisation cloud par outbox (V2)

## Décision
* Chaque opération métier ajoute une ligne `outbox` dans sa transaction (entité, identifiant, opération).
* L'envoi au cloud (V2) sera idempotent (clé = id outbox) et reprendra après coupure.
* Données financières en ajout seul : aucune résolution de conflit nécessaire.
* Données de référence modifiées à distance : demande de modification appliquée par le poste central
  si la `version` correspond, sinon rejetée et signalée (le local gagne).

## État
Outbox alimentée et testée indirectement ; l'API cloud, le QR menu et la file de commandes distantes
ne sont **pas** implémentés.
