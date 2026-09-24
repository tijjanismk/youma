# 0005 — Soldes calculés depuis des journaux en ajout seul

## Contexte
Cahier §4 : stock, caisse, dettes et compte employé ne sont jamais saisis ; les opérations financières
ne sont jamais modifiées ni supprimées.

## Décision
* Tables de mouvements : `mouvements_tresorerie`, `mouvements_stock`, `mouvements_client`,
  `mouvements_fournisseur`, `mouvements_employe`, plus `paiements`, `parts_paiement`, `remises`,
  `annulations`, `depenses`, `bulletins`, `journal_audit`.
* Solde = `SUM(montant)` ou `SUM(quantite)`. Aucun solde stocké.
* Correction = contre-passation liée (paiement négatif, dépense d'annulation, mouvement opposé).
* Vérification de Mobile Money : table `verifications_mm` (le dernier statut fait foi) plutôt qu'une mise à jour.

## Conséquences
* Les rapports sont toujours recalculables. Pas de cache de solde pour l'instant : les volumes le permettent.
* La paie devient simple : le net à payer est le solde du compte employé (le report du net négatif est naturel).
