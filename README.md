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
| Application installable (PWA) : écran d'accueil, plein écran, ouverture sans Wi-Fi | ✅ |
| Commandes par téléphone, QR sur la table, en ligne ; file de validation | ✅ (réseau local) |
| Zones à risque (quartier ou cercle GPS × heure × jours), liste noire de numéros | ✅ |
| Suivi en direct pour le client, position du livreur | ✅ (position : via relais HTTPS) |
| Serveur relais Internet facultatif (menu en ligne, code SMS Orange Mali ou simulé, suivi et position du livreur en HTTPS) | ✅ (SMS simulé tant que le contrat Orange n'est pas signé) |
| Installateur Windows (Tauri) | code prêt, à construire sous Windows |
| Recettes (ingrédients en g/ml/pièce), consommation théorique, coût matière | ✅ |
| Consignes : bouteilles et casiers, vides, consigne versée au dépôt, retours | ✅ |
| Rapprochement Mobile Money par relevé d'opérateur (CSV) | ✅ |
| Promotions et happy hours (prix fixe ou %, horaires, jours, dates) | ✅ |
| Statistiques : panier moyen, ventes par heure et par jour, serveurs, comparaison de périodes | ✅ |
| Cloud facultatif : résumé SMS de clôture, sauvegardes chiffrées, espace propriétaire multi-restaurants | ✅ |

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
#   → aussi https://<IP du PC>:7879 (HTTPS local, pour installer l'application ; --port-https 0 pour le couper)
```

### Installer Youma sur les téléphones (PWA)

Les navigateurs n'installent une application web qu'en HTTPS. En mode réseau, le poste central crée
son autorité de certification (conservée dans la base, limitée aux adresses du réseau local) et sert
l'application en HTTPS sur le port suivant. Pour chaque téléphone, **une seule fois** :

1. Administration → Appareils : scanner le QR « certificat du restaurant » et installer le certificat
   (Android : Paramètres → Sécurité → Installer un certificat → Certificat CA ; iPhone : Réglages →
   Profil téléchargé, puis Général → Informations → Réglages des certificats).
2. Scanner le QR d'appairage (adresse `https://…:7879`), puis « Installer l'application » sur l'accueil.

L'espace propriétaire (`/proprietaire` sur le relais, déjà en HTTPS) s'installe de la même façon, avec
sa propre icône. Icônes : `ui/public/icone.svg`, PNG produits par `cd ui && node outils/icones.mjs`.

### Relais Internet (facultatif)

Sans relais, le menu QR et les commandes fonctionnent sur le Wi-Fi du restaurant. Pour publier le menu
sur Internet, installer le relais sur un petit serveur (VPS) derrière un proxy HTTPS (Caddy, nginx) :

```bash
YOUMA_RELAIS_CLE=une-longue-cle-secrete cargo run --release -p youma-relais -- --port 8080 --ui ui/dist --derriere-proxy
```

Puis, sur le poste central : Administration → Commandes à distance → « Serveur relais Internet » :
adresse (`https://commande.exemple.ml`) et même clé. Le poste central appelle le relais toutes les 10 s ;
s'il se tait, le menu en ligne s'affiche « fermé ».

Code SMS de vérification du numéro : **Orange Mali** (API SMS d'Orange Developer). Identifiants à fournir
au relais seulement :

```bash
YOUMA_ORANGE_CLIENT_ID=… YOUMA_ORANGE_CLIENT_SECRET=… YOUMA_ORANGE_EXPEDITEUR=+223XXXXXXXX \
YOUMA_ORANGE_NOM_EXPEDITEUR=Baobab   # facultatif, nom validé par Orange
```

Sans ces identifiants, le relais **simule** l'envoi : le code s'affiche sur la page du client (essais, démonstration).

### Cloud (facultatif)

Le même serveur Internet sert de cloud pour plusieurs restaurants. Le fournisseur inscrit chaque restaurant :

```bash
cargo run --release -p youma-relais -- --ajouter-restaurant "Maquis Le Baobab" --donnees ./donnees-relais
# → affiche la clé à saisir sur le poste central (Administration → Cloud)
```

Sur le poste : adresse, clé, téléphone du propriétaire, phrase de chiffrement (à noter sur papier) et mot de passe
de l'espace propriétaire. Le propriétaire consulte ensuite tous ses restaurants sur `https://…/proprietaire`.

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
