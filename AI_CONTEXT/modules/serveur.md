# Module : serveur (poste central, `crates/youma-server`)

Rôle : exposer le métier en HTTP/WebSocket, servir `ui/dist`, imprimer, synchroniser relais et cloud.

## Fichiers (`crates/youma-server/src/`)
- `api.rs` (1616 l., 164 routes) — `routeur` ; macros `ecrire!`/`lire!` ; routes publiques `/public/suivi/{code}`, `/public/position/{code}` (l. 256–258, sans limiteur) ; sauvegardes `exporter`/`restaurer` avec chemin fourni (l. 235–236, 1341–1350).
- `lib.rs` (238 l.) — `Config` (`reseau`, `reseau_sans_licence`, `demo`…), extracteurs `Auth` (l. 145, `X-Autorisation-Pin` l. 106) et `Poste`, `est_local`, `demarrer`, `lien_telephones`.
- `ws.rs` (51 l.) — WebSocket `/api/ws?jeton=…` (diffuse des identifiants).
- `tls.rs` (163 l.) — autorité locale bridée aux adresses privées, HTTPS port + 1.
- `relais.rs` (139 l.), `cloud.rs` (186 l.), `taches.rs` (78 l.), `imprimantes.rs` (122 l., `unsafe` l. 60), `poste.rs`, `erreurs.rs`.

## Entrant
`main.rs`, `apps/desktop/src-tauri/src/main.rs` (construit `Config`), tests `tests/api.rs`.

## Notes
- [CONFIRMÉ] Aucun CORS sur `/api` (grep vide) : nécessaire seulement pour une coquille Capacitor (option 2 de l'étude mobile).
- [CONFIRMÉ] Appel bloquant en async signalé par la carte : `api.rs:1310` (`std::fs::metadata`).
