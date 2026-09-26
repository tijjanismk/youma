# 0025 — Mise à jour de l'interface : React 19, React Router 7, Vite 8

## Contexte
Le porteur de projet veut les versions récentes de React et de Tauri (26/09/2026). L'interface était en React 18.3,
React Router 6.30, Vite 5.4, Vitest 2.1 ; `npm audit` signalait 7 vulnérabilités (outils de développement).

## Décision
* **React 19.3**, **React Router 7.18** (paquet `react-router` : `react-router-dom` n'est plus nécessaire ; les
  options `future` de transition vers la v7 sont retirées, elles sont devenues le comportement par défaut).
* **Vite 8.3** (construction par Rolldown) et `@vitejs/plugin-react` 6 ; **Vitest 5**, jsdom 30, Playwright 1.63,
  jest-dom 7. Le plugin du service worker (`vite.config.ts`) marche sans changement. `npm audit` : 0 vulnérabilité.
* `package-lock.json` régénéré (l'ancien arbre empêchait npm de résoudre les nouvelles versions).
* **Tauri reste en 2** : 2.11.6 est la dernière version stable ; Tauri 3 n'existe qu'en alpha. Le `Cargo.lock` de
  `apps/desktop` est remis à jour avec les dépendances actuelles de `youma-server`.
* **TypeScript reste en 5.9** : TypeScript 7 (réécriture complète du compilateur) ne sert qu'à la vérification des
  types, sans gain pour les restaurants ; à reprendre quand l'outillage (Vite, éditeurs) l'aura adopté.

## Alternatives écartées
* Migration pas à pas sur plusieurs pull requests : Vitest 2 exige Vite 5, les deux devaient changer ensemble.

## Conséquences
Le fichier JavaScript principal passe de 84 à 107 Ko compressés (React 19 et React Router 7) : sans effet notable
sur le Wi-Fi du restaurant. En local, un Chromium plus ancien que celui attendu par Playwright se choisit avec
`PLAYWRIGHT_CHROMIUM=/chemin/vers/chrome npx playwright test` (la CI télécharge le bon).
