# Module : caisse et clients (paiements, crédit, fidélité, cartes cadeaux, sociétés, Mobile Money)

Rôle : encaisser en plusieurs parts, sessions de caisse et billetage, trésorerie, comptes clients, fidélité, cartes, contrats société.

## Fichiers (`crates/youma-core/src/`)
- `caisse.rs` (1116 l.) — `encaisser(_op)` (parts : especes, mobile_money, tpe, credit, carte_cadeau ; `PartSaisie.contrat_id`), `annuler_paiement` (RG-CAI-07, recrédite cartes, retire points), sessions `ouvrir_session`/`cloturer_session` (`fond_garde`, remise au coffre RG-CAI-16 l. 350), `billetage`, `mouvement`, `transferer`, dépenses, `verifier_mobile_money`.
- `clients.rs` (313 l.) — `Client` (vip, vip_jusqu_au, points), `vente_credit`, `contrepasser_credit`, `regler`, `releve`, `dettes_par_anciennete`.
- `fidelite.rs` (224 l.) — `solde`, `etat`, `gagner_op` (86), `annuler_gain_op` (110), `utiliser` (119, crée une remise), `definir_privilege` (187), `SQL_COMMANDE_VIP`, `est_vip`.
- `cartes.rs` (255 l.) — `vendre` (espèces/MM, hors CA), `offrir_avoir` (perm COMMANDE_OFFRIR, motif), `debiter_op`, `recrediter_op`, `par_code`, `code_lisible`.
- `contrats.rs` (171 l.) — `enregistrer` (perm CLIENT_DEPASSER_LIMITE), `part_societe`, `controler_part_op` (121), `releve` (154).
- `releves_mm.rs` (334 l.) — import CSV d'un relevé Mobile Money, rapprochement (RG-RMM-02/03 l. 157).

## Entrant
`caisse.rs` importé par 21 (achats, cartes, clients, employes, livraison…) ; `fidelite`/`cartes`/`contrats` ← caisse ; tout ← api.rs.

## Base de données
Écrit : paiements, parts_paiement, sessions_caisse, billetages, mouvements_tresorerie, comptes_tresorerie, depenses, categories_depense, verifications_mm, commandes, clients, mouvements_client, mouvements_fidelite, remises, cartes_cadeaux, mouvements_carte, contrats_societe, releves_mm, lignes_releve_mm.

## Règles métier
- [CONFIRMÉ] Part société = vente à crédit sur le compte de la société, employé nommé (RG-SOC-02) — `caisse.rs:457`, `contrats.rs:121`.
- [CONFIRMÉ] Points : ni part société ni bon d'avoir — `fidelite.rs:93–97`.
- [CONFIRMÉ] Bon d'avoir : motif obligatoire, accord gérant (RG-CAD-04) — `cartes.rs:193`.
- [DÉDUIT] `parts_paiement` est écrite seulement par `caisse.rs` (hors tests) : seul point d'entrée des paiements.
