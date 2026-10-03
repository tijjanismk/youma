# Module : personnel et paie

Rôle : fiches employés, présences, avances, consommations employé, bulletins et paiement des salaires.

## Fichiers (`crates/youma-core/src/`)
- `employes.rs` (562 l.) — `enregistrer`, `pointer`, `presences`, `solde` (compte employé), `avance`, `imputer_commande`, `releve`.
- `paie.rs` (439 l.) — `apercu`, `cloturer` (bulletin), `payer`, `a_payer` ; historique et blocages affichés côté UI.

## Base de données
Écrit : employes, presences, mouvements_employe, commandes (conso employé), bulletins.

## Règles métier
- [CONFIRMÉ] Types de contrat contrôlés (RG-EMP-02) — `employes.rs:150` ; calcul de paie RG-PAI-03/07/08 — `paie.rs:88`.
