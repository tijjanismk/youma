# 0013 — Canaux de commande, zones à risque et suivi en direct

## Contexte
Le porteur de projet veut ajouter les commandes en ligne et un suivi en direct, et définir des zones à risque
(« une commande à telle heure à tel endroit sera bloquée »), sans rien imposer : le QR en salle, le menu papier
et la commande en ligne doivent rester indépendants. Réponses du porteur : relais Internet **oui mais facultatif** ;
zones **par quartier et cercle GPS** ; paiement **d'avance et à la livraison** (les deux, au choix du restaurant) ;
vérification du numéro **oui** (SMS si un fournisseur est configuré, sinon rappel par la caisse).

## Décision
* **Canaux indépendants** (`commandes.canal`) : `serveur` (menu papier, toujours actif), `telephone` (saisi par
  le personnel), `qr_table`, `en_ligne`. Chacun s'active dans Administration → Commandes à distance.
* **File de validation** : une commande QR ou en ligne est créée « en attente » (`validation`), hors du plan de
  salle, de la cuisine et de la liste des additions. Un membre du personnel l'accepte (envoi en cuisine, ou
  encaissement Mobile Money d'avance puis envoi) ou la refuse avec motif. Le client ne commande jamais seul
  jusqu'à la cuisine : c'est la parade principale contre les fausses commandes.
* **QR des tables** : code secret aléatoire par table (`tables_salle.code_qr`), régénérable. L'acceptation rattache
  la commande à la table, ou à son addition déjà ouverte.
* **Zones à risque** (`zones_risque`) : quartier et/ou cercle GPS, plage horaire (minuit possible), jours, action
  bloquer / paiement d'avance / accord d'un responsable ; la plus stricte l'emporte. Saisie du personnel :
  PIN d'un responsable (`zone.outrepasser`). Distances par haversine sur des microdegrés entiers (pas de montant :
  le calcul flottant est admis).
* **Liste noire** (`numeros_bloques`) sur le numéro normalisé (8 chiffres, sans +223).
* **Suivi** : code de suivi public (8 caractères) pour le client ; lien livreur secret distinct (12 caractères) pour
  envoyer la position, seulement pendant la course (`positions_livreur`, ajout seul).
* **Routes publiques** `/api/public/*` : sans connexion ni appairage, mais refusées si le canal est inactif ;
  le poste central ignore `telephone_verifie` et `origine_id` venant du réseau local (réservés au relais).
* **Relais Internet facultatif** (étape suivante) : le poste central garde la main ; le relais publie le menu,
  reçoit les commandes (identifiant d'origine pour l'idempotence), envoie les SMS, sert la page de suivi en HTTPS.

## Alternatives écartées
* Commande client envoyée directement en cuisine : trop exposée aux fausses commandes et aux erreurs.
* Zones par carte dessinée (polygones) : trop complexe à saisir ; quartier + cercle couvrent le besoin.
* Carte intégrée (tuiles) dans l'application : dépend d'Internet ; lien vers OpenStreetMap et distance affichée.
* Suivi par le même code pour le client et le livreur : le client pourrait falsifier la position.

## Conséquences
Migration 0003 ; permissions `commande.valider_entrante` et `zone.outrepasser` ; règles RG-CAN-01 à 05,
RG-ZON-01 à 03, RG-LIV-04. **[HYPOTHÈSE]** La géolocalisation du navigateur exige HTTPS : sur le Wi-Fi du
restaurant (http), la page livreur ne peut pas partager sa position ; le suivi en direct complet passe par le relais.
**[HYPOTHÈSE]** Pas de limitation de débit sur les routes publiques du réseau local (Wi-Fi du restaurant) ;
le relais Internet devra en avoir une.
