# CLAUDE.md — Youma

Logiciel de gestion de restaurant local-first pour le Mali.
Référence unique : `docs/CAHIER_DES_CHARGES.md`. Le lire avant toute tâche.

## Phase actuelle : RÉALISATION

- Conception livrée dans `docs/conception/` (phases 1 à 4) ; MVP 0 et MVP 1 codés (voir fiche 0001).
- Toute nouvelle décision technique : une fiche dans `docs/decisions/NNNN-titre.md`
  (contexte, décision, alternatives écartées, conséquences).
- Hypothèses marquées **[HYPOTHÈSE]**, contradictions du cahier des charges signalées, jamais tranchées en silence.
- Avant de pousser : `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`,
  `cd ui && npm test && npm run build && npx playwright test`.

## Règles non négociables (à respecter aussi une fois le code commencé)

- Aucune dépendance cloud pour vendre, encaisser, imprimer, gérer stock, employés, paie, rapports locaux.
- Montants en entiers FCFA. Jamais de flottants.
- Opérations financières et de stock : ajout seul, correction par contre-passation, jamais de suppression.
- Soldes (stock, caisse, dettes, compte employé) calculés depuis les mouvements.
- Toute opération métier multi-tables dans une seule transaction.
- Logique métier dans le backend Rust uniquement ; l'interface ne touche jamais la base.
- Ventes rattachées à une journée d'exploitation, pas à la date du PC.
- Une licence expirée ne bloque jamais les ventes.
- Interface en français, grands boutons, peu de clics.

## Conventions

- Règles métier numérotées `RG-<MODULE>-NN` (ex. `RG-CAI-03`), citées dans le code et les tests.
- Commits en français, à l'impératif, courts.
