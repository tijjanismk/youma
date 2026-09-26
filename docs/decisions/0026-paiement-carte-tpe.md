# 0026 — Paiement par carte sur un TPE non relié

## Contexte
Des restaurants ont un terminal de paiement (TPE) de leur banque. Le cœur acceptait déjà un paiement « carte »,
mais vers n'importe quel compte, sans référence, et l'écran d'encaissement n'avait pas de bouton pour ça.

## Décision
* **TPE non relié à Youma** : le caissier tape le montant sur le TPE, puis choisit « Carte (TPE) » dans Youma et
  recopie le **numéro d'autorisation** imprimé sur le ticket du TPE.
* **RG-CAI-15** : une part « carte » va sur un compte de type **banque** ; le numéro d'autorisation est obligatoire ;
  il ne peut pas servir deux fois **dans la même journée d'exploitation** sur le même compte (un ticket de TPE ne paie
  pas deux additions). Le montant n'entre pas dans le tiroir : il crédite le compte bancaire.
* Un bouton par compte bancaire actif (« Carte (TPE) », ou « Carte — nom du compte » s'il y en a plusieurs). Sans
  compte bancaire (Administration → Moyens de paiement → « Banque »), pas de bouton.
* **Migration 0008** : l'index unique sur (compte, référence) est réservé au Mobile Money (RG-CAI-05, unicité
  définitive). Les numéros d'autorisation (souvent 6 chiffres) finiraient sinon par se répéter au fil des mois et
  bloqueraient un paiement légitime.
* Démonstration : compte « Banque (TPE) ».

## Alternatives écartées
* **TPE relié** (protocole caisse du fabricant ou de la banque) : propre à chaque banque, agrément et matériel
  nécessaires pour l'essayer ; faible usage de la carte au Mali face au Mobile Money. À reprendre avec une banque
  partenaire.
* Numéro d'autorisation facultatif : sans lui, pas de rapprochement possible avec le relevé du TPE.

## Conséquences
Rapprochement de fin de journée : comparer les parts « carte » du rapport avec le ticket de clôture (télécollecte) du
TPE. **[HYPOTHÈSE]** Import du relevé du TPE (comme les relevés Mobile Money, fiche 0016) à faire quand un format
réel de banque malienne sera disponible.
