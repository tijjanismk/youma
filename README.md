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
| Commandes par téléphone, QR sur la table, en ligne ; file de validation | ✅ (réseau local) |
| Zones à risque (quartier ou cercle GPS × heure × jours), liste noire de numéros | ✅ |
| Suivi en direct pour le client, position du livreur | ✅ (position : via relais HTTPS) |
| Serveur relais Internet facultatif (menu en ligne, code SMS, suivi et position du livreur en HTTPS) | ✅ |
| Installateur Windows (Tauri) | code prêt, à construire sous Windows |
| Recettes, consignes, cloud, multi-établissements | V2, non commencé |

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

### Relais Internet (facultatif)

Sans relais, le menu QR et les commandes fonctionnent sur le Wi-Fi du restaurant. Pour publier le menu
sur Internet, installer le relais sur un petit serveur (VPS) derrière un proxy HTTPS (Caddy, nginx) :

```bash
YOUMA_RELAIS_CLE=une-longue-cle-secrete cargo run --release -p youma-relais -- --port 8080 --ui ui/dist --derriere-proxy
```

Puis, sur le poste central : Administration → Commandes à distance → « Serveur relais Internet » :
adresse (`https://commande.exemple.ml`) et même clé. Le poste central appelle le relais toutes les 10 s ;
s'il se tait, le menu en ligne s'affiche « fermé ». Code SMS : modèle d'adresse du fournisseur, par exemple
`https://api.fournisseur.ml/send?to={numero}&text={message}`.

## Tests

```bash
cargo test --workspace                       # règles métier, 15 scénarios d'acceptation, canaux, API, impression
cd ui && npm test                            # logique et composants (Vitest)
cd ui && npm run build && npx playwright test   # parcours complets dans Chromium sur un vrai serveur
```

## Organisation

```
crates/youma-core     règles métier (RG-*), SQLite, rapports, impression, licence
crates/youma-server   API HTTP + WebSocket, imprimantes, tâches de fond
crates/youma-licence  outil fournisseur (clés, licences)
crates/youma-relais   serveur relais Internet facultatif (commandes en ligne, SMS, suivi)
apps/desktop          coquille Windows (Tauri 2)
ui                    interface React + TypeScript
docs/                 cahier des charges, conception (phases 1 à 4), fiches de décision
```

## Documents

- [`docs/CAHIER_DES_CHARGES.md`](docs/CAHIER_DES_CHARGES.md) — cahier des charges en vigueur (v2)
- [`docs/conception/`](docs/conception/) — vision, architecture, données, règles, écrans, API, plan
- [`docs/decisions/`](docs/decisions/) — fiches de décision
- [`docs/cahier-des-charges-v1.md`](docs/cahier-des-charges-v1.md) — version initiale, pour mémoire
