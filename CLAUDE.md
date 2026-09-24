# CLAUDE.md — Youma

Logiciel de gestion de restaurant local-first pour le Mali.
Référence unique : `docs/CAHIER_DES_CHARGES.md`. Le lire avant toute tâche.

## Phase actuelle : CONCEPTION

- Ne pas écrire de code applicatif tant que l'architecture n'est pas validée.
- Livrer la conception par phases (section 30 du cahier des charges) dans `docs/conception/`.
- S'arrêter à la fin de chaque phase et attendre la validation.
- Décisions techniques : une fiche par décision dans `docs/decisions/NNNN-titre.md`
  (contexte, décision, alternatives écartées, conséquences).
- Hypothèses marquées **[HYPOTHÈSE]**, contradictions du cahier des charges signalées, jamais tranchées en silence.

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
