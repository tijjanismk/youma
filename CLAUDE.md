# CLAUDE.md — Youma

Logiciel de gestion de restaurant local-first pour le Mali.
Référence unique : `docs/CAHIER_DES_CHARGES.md`. Le lire avant toute tâche.
Après une pause : lire `docs/REPRISE.md` (état, ce qui reste, questions ouvertes) et le tenir à jour.

## Phase actuelle : RÉALISATION

- Conception livrée dans `docs/conception/` (phases 1 à 4) ; MVP 0, MVP 1 et V2 codés (voir fiche 0001 et le
  tableau de `README.md`) : canaux à distance, recettes, consignes, relevés Mobile Money, promotions,
  statistiques, cloud facultatif, refonte de l'interface (0019), application installable (0020).
- Toute nouvelle décision technique : une fiche dans `docs/decisions/NNNN-titre.md`
  (contexte, décision, alternatives écartées, conséquences). Dernière fiche : 0025.
- Hypothèses marquées **[HYPOTHÈSE]**, contradictions du cahier des charges signalées, jamais tranchées en silence.
- Avant de pousser (comme la CI, `.github/workflows/ci.yml`) : `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace`, `cd ui && npm test && npm run build && npx playwright test`
  (Playwright lance `target/debug/youma-server` : faire `cargo build -p youma-server` avant ; Chromium local plus ancien :
  `PLAYWRIGHT_CHROMIUM=/opt/pw-browsers/chromium-1194/chrome-linux/chrome`).

## Règles non négociables

- Aucune dépendance cloud pour vendre, encaisser, imprimer, gérer stock, employés, paie, rapports locaux.
- Montants en entiers FCFA. Jamais de flottants.
- Opérations financières et de stock : ajout seul, correction par contre-passation, jamais de suppression.
- Soldes (stock, caisse, dettes, compte employé) calculés depuis les mouvements.
- Toute opération métier multi-tables dans une seule transaction.
- Logique métier dans le backend Rust uniquement ; l'interface ne touche jamais la base.
- Ventes rattachées à une journée d'exploitation, pas à la date du PC.
- Une licence expirée ne bloque jamais les ventes.
- Interface en français, grands boutons, peu de clics.
- Rien n'est chargé depuis Internet par l'interface (polices, icônes, scripts embarqués) : elle doit marcher hors ligne.

## Carte du code

- `crates/youma-core` : tout le métier (rusqlite). Migrations `migrations/NNNN_*.sql` par `PRAGMA user_version` (0001 → 0007, le test
  `regles.rs` vérifie la version). Écritures via `Db::executer(acteur, |op| …)` (transaction + audit +
  permissions `op.exiger(perm::…)`). Horloge injectable (`HorlogeFixe` en test), jamais l'heure système en dur.
  Valeurs techniques : `db::valeur_systeme` / `definir_valeur_systeme` (table `systeme`, incluse dans les sauvegardes).
- `crates/youma-server` : poste central axum (API `/api`, WebSocket, sert `ui/dist`). Macros `ecrire!`/`lire!`,
  extracteurs `Auth`/`Poste`. `tls.rs` : HTTPS du réseau local (autorité du restaurant bridée aux adresses
  privées, port HTTP + 1, `--port-https 0` pour couper). `relais.rs`/`cloud.rs` : tâches de fond facultatives.
- `crates/youma-relais` : serveur Internet facultatif (relais des commandes en ligne + cloud multi-restaurants,
  SMS Orange Mali ou simulation). `crates/youma-licence` : licences hors ligne.
- `docs/guides/` : installation sur la caisse (`installation-caisse.md`), mise en ligne du relais
  (`relais-en-ligne.md`, fichiers dans `deploiement/relais/`).
- `apps/desktop` : coquille Tauri (hors workspace, construite à part) ; elle construit `youma_server::Config`
  elle aussi : tout nouveau champ de `Config` s'y ajoute.
- `ui/` : React 19 + TypeScript (Vite 8, React Router 7 : importer depuis `react-router`, fiche 0025). `src/pages` (écrans du personnel), `src/public` (menu client, suivi,
  livreur, propriétaire : sans connexion), `src/composants`, `src/contexte.tsx` (`useApp().agir` : PIN/mot de
  passe redemandés automatiquement), `src/api.ts`, `src/pwa.ts`.

## Interface (fiches 0019, 0020, 0022)

- Design system « Mali vivant » (fiche 0022) : jetons CSS dans `ui/src/styles.css`, thème clair dans `:root`, sombre sous
  `[data-theme="sombre"]` (choix par poste, `src/theme.ts`). **Jamais de couleur en dur** : `--accent`, `--carte`, `--sur-etat`… ; le bloc
  « Téléphone » (`@media (max-width: 900px)`) vient juste avant la couche « Mali vivant » (fin de fichier) : y reporter toute grille
  modifiée (utiliser `minmax(0, 1fr)` pour ne pas faire déborder l'écran).
- Icônes `lucide-react`, police Poppins embarquée (`@fontsource/poppins`). Pas de CDN.
- PC : menu latéral repliable. Téléphone : barre du bas, prise de commande avec la commande en tiroir
  (« Voir la commande (n) »). Écrans secondaires : chiffres clés `composants/Chiffres.tsx` ; `TableauDonnees`
  devient des cartes sur téléphone (`data-label`). Photos des plats en URL `data:` compressées côté navigateur (`composants/Plat.tsx`).
- Les tests e2e ciblent les rôles et `aria-label` (« Menu », « Autres écrans », « Changer d'utilisateur »,
  « Ajouter un X »…) : ne pas les renommer sans mettre à jour `ui/e2e/`.
- PWA : `ui/public/manifest*.webmanifest`, icônes produites par `cd ui && node outils/icones.mjs`, service worker
  généré par le plugin de `vite.config.ts` depuis `ui/sw/sw.js`. Règle : **`/api/` ne passe jamais par le cache**.
  Playwright bloque le service worker (`serviceWorkers: "block"`) sauf dans `e2e/pwa.spec.ts`.
- Mise en forme TypeScript : `npx prettier --print-width 160`. Pas de `cargo fmt` (le code n'a jamais été formaté ainsi).

## Pièges connus

- Tests e2e en série sur une même base de démonstration (port 7979) : un nouveau test ne doit pas dépendre
  d'une table libre (préférer une vente à emporter) ni laisser d'état gênant pour les suivants.
- Base de démonstration : PIN propriétaire 1234, gérant 2222, caisse 3333, serveuse 4444, grill 5555 ;
  mots de passe d'administration « baobab123 » (Mariam), « adama123 » (Adama).
- Arrêter un serveur lancé à la main : `pgrep -x youma-server | xargs -r kill` (pas `pkill -f`, qui tue le shell).
- Disque plein pendant `cargo build` : `cargo clean` (le dossier `target` grossit vite, plus de 10 Go).

## Conventions

- Règles métier numérotées `RG-<MODULE>-NN` (ex. `RG-CAI-03`), citées dans le code et les tests.
- Commits en français, à l'impératif, courts.
