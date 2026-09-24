# 0002 — Un seul cœur, toujours en client-serveur (même en mono-poste)

## Contexte
Trois modes d'installation (mono-poste, réseau local, local + cloud) doivent partager le même code et
la même base ; passer du mode A au mode B ne doit pas demander de migration.

## Décision
* `youma-core` (Rust) contient toute la logique métier et la persistance.
* `youma-server` (axum) expose une API HTTP + WebSocket et sert l'interface React compilée.
* La fenêtre Tauri du poste central charge `http://127.0.0.1:<port>` comme n'importe quel navigateur.
* Mode A : écoute sur `127.0.0.1`. Mode B : écoute sur le réseau local (`--reseau`), appareils appairés.

## Alternatives écartées
* Commandes Tauri (IPC) pour le poste central + HTTP pour les autres : deux chemins d'appel à maintenir.
* Application mobile native pour les serveurs : installation impossible à imposer sur leurs téléphones.

## Conséquences
* Le poste central et les téléphones exécutent exactement la même interface et les mêmes contrôles.
* L'interface ne touche jamais la base : toute règle est vérifiée côté serveur, même si l'interface la pré-vérifie.
* Le serveur se teste sans fenêtre (tests d'intégration HTTP, Playwright).
