# Module : personnel et paie

Rôle : fiches employés, présences, avances, consommations employé, accès livreur, bulletins et paiement des salaires.

## Fichiers (`crates/youma-core/src/`)
- `employes.rs` (601 l.) — `enregistrer`, `pointer`, `presences`, `solde` (compte employé), `avance`, `imputer_commande`, `releve`, `definir_pin_livreur` (l. 569, RG-LIV-05 ; `Employe.acces_livreur` en lecture).
- `paie.rs` (477 l.) — `apercu`, `cloturer` (bulletin) et `payer` (enveloppes de `cloturer_op` l. 288 / `payer_op` l. 426), `payer_periode` (l. 259, RG-PAI-10 : arrête et paie dans une seule transaction), `a_payer`.

## Entrant
api.rs : `/employes*`, `/employes/{id}/acces-livreur`, `/paie/apercu|cloturer|payer|payer-periode|bulletins`. UI : `ui/src/pages/Paie.tsx` (« Payer », « Tout payer », périodes toutes faites), `FicheEmploye.tsx` (carte « Application Youma Livreur »).

## Base de données
Écrit : employes (dont `pin_livreur_hash`, migration 0013), presences, mouvements_employe, commandes (conso employé), bulletins, mouvements_tresorerie (paiement de salaire).

## Règles métier
- [CONFIRMÉ] Types de contrat contrôlés (RG-EMP-02) — `employes.rs:150` ; calcul de paie RG-PAI-03/07/08 — `paie.rs:88`.
- [CONFIRMÉ] Salaire jamais payé depuis le tiroir (RG-PAI-09) — `paie.rs::payer_op` ; refus = rien d'arrêté avec `payer_periode`.
