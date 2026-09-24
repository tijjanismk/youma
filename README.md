# Youma

Logiciel de gestion de restaurant local-first pour les restaurants, maquis, fast-foods et cafés au Mali.

Fonctionne sans Internet, résiste aux coupures d'électricité, gère espèces et Mobile Money en FCFA,
les ardoises, les tournées, et des employés souvent sans contrat écrit, ni INPS ni AMO.

## État

MVP 0 et MVP 1 réalisés (plus la livraison de base), avec des tests automatisés. Le pilote en conditions
réelles reste à faire. Voir [`docs/conception/phase-4-plan-realisation.md`](docs/conception/phase-4-plan-realisation.md).

| Fonction | État |
|---|---|
| Utilisateurs, rôles, PIN, autorisation ponctuelle par un responsable | ✅ |
| Catalogue (options, prix par zone, rupture du jour, import CSV) | ✅ |
| Salle, tables, tournées, transfert, fusion, comptoir, à emporter | ✅ |
| Envois cuisine/bar par poste, tickets ESC/POS, écran cuisine | ✅ |
| Caisse : sessions, billetage, paiements mixtes, Mobile Money à vérifier, retraits propriétaire, dépenses, rapport Z | ✅ |
| Stock des boissons, achats au marché, conditionnements, inventaire | ✅ |
| Clients et crédit, fournisseurs et dettes | ✅ |
| Employés (contrat, INPS et AMO facultatifs), présences, avances, paie, bulletins | ✅ |
| Livraison et remise livreur | ✅ (base) |
| Rapports avec formules, export CSV, journal d'audit | ✅ |
| Ticket de caisse = bon de sortie (code de contrôle, écran de contrôle) | ✅ |
| Mot de passe personnel pour l'administration (en plus du PIN) | ✅ |
| Sauvegardes, intégrité, restauration, licence hors ligne, diagnostic | ✅ |
| Mode réseau (téléphones via navigateur, appairage par QR) | ✅ |
| Installateur Windows (Tauri) | code prêt, à construire sous Windows |
| Recettes, consignes, QR menu, cloud, multi-établissements | V2, non commencé |

## Démarrer

```bash
# Interface
cd ui && npm ci && npm run build && cd ..
# Poste central avec la base de démonstration
#   PIN : propriétaire 1234, gérant 2222, caisse 3333, serveuse 4444, grill 5555
#   Mot de passe d'administration : propriétaire « baobab123 », gérant « adama123 »
cargo run -p youma-server -- --demo --donnees ./donnees --ui ui/dist
# → http://127.0.0.1:7878
# Mode réseau (téléphones des serveurs) : ajouter --reseau
```

## Tests

```bash
cargo test --workspace                       # règles métier, 15 scénarios d'acceptation, API, impression
cd ui && npm test                            # logique et composants (Vitest)
cd ui && npm run build && npx playwright test   # parcours complets dans Chromium sur un vrai serveur
```

## Organisation

```
crates/youma-core     règles métier (RG-*), SQLite, rapports, impression, licence
crates/youma-server   API HTTP + WebSocket, imprimantes, tâches de fond
crates/youma-licence  outil fournisseur (clés, licences)
apps/desktop          coquille Windows (Tauri 2)
ui                    interface React + TypeScript
docs/                 cahier des charges, conception (phases 1 à 4), fiches de décision
```

## Documents

- [`docs/CAHIER_DES_CHARGES.md`](docs/CAHIER_DES_CHARGES.md) — cahier des charges en vigueur (v2)
- [`docs/conception/`](docs/conception/) — vision, architecture, données, règles, écrans, API, plan
- [`docs/decisions/`](docs/decisions/) — fiches de décision
- [`docs/cahier-des-charges-v1.md`](docs/cahier-des-charges-v1.md) — version initiale, pour mémoire
