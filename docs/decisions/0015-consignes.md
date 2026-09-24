# 0015 — Consignes : bouteilles et casiers

## Contexte
Au Mali, bières et sodas arrivent en bouteilles et casiers consignés : le dépôt facture une consigne par emballage
plein livré, et la reprend contre les vides rendus. Le cahier des charges (§9, option V2) demande de suivre les
consignes versées, les vides en stock et les retours. Les pertes d'emballages coûtent cher sans qu'on les voie.

## Décision
* `emballages` (nom, consigne entière) ; un article de stock peut être lié à son emballage (une unité pleine le contient).
* Journal `mouvements_emballages` en ajout seul : `quantite` = effet sur les emballages **détenus** (pleins + vides
  présents), `montant` = effet sur la **consigne versée** au fournisseur (ce qu'il doit rendre).
* **Vides = détenus − pleins en stock** : rien à saisir à la vente, la bouteille reste au restaurant. Un casier
  (sans article lié) se compte directement.
* La réception d'un achat porte les emballages reçus et les vides rendus ; la consigne nette s'ajoute au total de
  l'achat, qui suit le chemin habituel (caisse ou dette fournisseur). Aucune écriture de trésorerie séparée.
* Casse, perte, bouteille emportée ou rapportée par un client : mouvements manuels motivés. Comptage des vides :
  mouvement d'écart. Vides rendus hors livraison : remboursement en caisse ou déduction de la dette.

## Alternatives écartées
* Mouvement d'emballage à chaque vente : inutile (détenus inchangés) et plus d'écritures.
* Consigne encaissée auprès des clients qui emportent la bouteille : pratique variable selon les maquis ; laissée à la
  saisie manuelle « emportée par un client » avec motif **[HYPOTHÈSE]**, à confirmer au pilote.

## Conséquences
Migration 0005 (dont `articles_stock.emballage_id`) ; règles RG-CON-01 à 06 ; onglet Stock → « Consignes » ;
section « Emballages consignés » dans la réception d'achat. Démonstration : bouteille bière 65 cl (150 FCFA),
casier de 12 (2 500 FCFA), reçus avec le stock initial.
