# Phase 1 — Vision et architecture

## 1. Vision produit

Youma est la caisse et le carnet de gestion d'un restaurant malien : maquis, fast-food, café,
petit restaurant. Il tourne sur **un PC Windows sans Internet**, résiste aux coupures, parle
en **FCFA entiers**, connaît les espèces **et** le Mobile Money, les ardoises, les tournées,
les avances sur salaire et les employés **sans contrat écrit**.

Promesse au propriétaire : *« Vous voyez chaque soir ce qui a été vendu, encaissé, dépensé,
offert, annulé, et où est parti le stock. »* Sans en faire un outil policier : les contrôles
(motif, PIN du gérant) ne se déclenchent que sur les actions à risque.

Principes directeurs :

1. **Le local d'abord.** Tout ce qui sert au service fonctionne sans cloud et sans licence valide.
2. **Le journal, pas le solde.** Stock, caisse, dettes et compte employé sont des sommes de mouvements.
3. **Rien ne disparaît.** Une erreur se corrige par contre-passation.
4. **La journée d'exploitation**, pas la date du PC.
5. **Flexibilité RH.** La plupart des employés n'ont ni contrat écrit, ni INPS, ni AMO : ces champs
   sont facultatifs, et les cotisations sont désactivées par défaut (voir `RG-EMP-*`, `RG-PAI-*`).

## 2. Phasage retenu

| Lot | Contenu | État dans ce dépôt |
|---|---|---|
| MVP 0 | utilisateurs/rôles/PIN, catalogue, salle et comptoir, commandes et tournées, envois cuisine/bar imprimés, caisse (sessions, paiement mixte, Mobile Money), dépenses, stock des articles revendus + inventaire, rapport Z et journalier, sauvegarde/intégrité, licence hors ligne | implémenté |
| MVP 1 | clients et crédit, fournisseurs/achats/dettes, employés/présences/avances/paie, mode réseau local (navigateur), rapports + export CSV | implémenté |
| V2 | livraison et remise livreur | implémenté (base) |
| V2 | recettes, consignes, QR menu, commandes en ligne, cloud, rapprochement MM par relevé, promotions, multi-établissements | conçu (outbox en place), non implémenté |

**[HYPOTHÈSE]** Équipe réduite (1–2 développeurs) : on privilégie un seul binaire et un seul
langage métier (Rust) plutôt qu'une pile plus large.

## 3. Architecture

```
┌──────────────────────── Poste central (Windows) ─────────────────────────┐
│  Coquille Tauri 2 (fenêtre)  ──HTTP──►  youma-server (axum, HTTP + WS)   │
│                                           │                             │
│                                           ▼                             │
│                                     youma-core (Rust)                   │
│                              règles métier + SQLite (WAL)               │
└──────────────────────────────────────────────────────────────────────────┘
          ▲  HTTP/WS sur le réseau local (mode B uniquement)
          │
  téléphones serveurs, tablette cuisine : navigateur, même interface React
```

* **Un seul cœur, toujours client-serveur** (fiche 0002). Mode A = le serveur écoute sur
  `127.0.0.1`. Mode B = il écoute sur le réseau local (module `reseau` de la licence).
* **SQLite sur le poste central**, un seul processus écrivain (fiche 0003).
* **L'interface ne touche jamais la base** : elle appelle l'API HTTP. La fenêtre Tauri charge
  simplement `http://127.0.0.1:<port>` (fiche 0002).
* **Cloud** (V2) : API séparée ; le poste central pousse son **outbox** (fiche 0008).

## 4. Structure du projet

```
Cargo.toml                 espace de travail Rust
crates/youma-core/         domaine : règles métier, migrations SQLite, rapports, impression, licence
  src/db.rs                ouverture, PRAGMA, migrations, transaction métier
  src/horloge.rs           garde d'horloge (RG-SYS-01)
  src/auth.rs              utilisateurs, rôles, PIN, sessions, autorisation ponctuelle
  src/journee.rs           journée d'exploitation
  src/catalogue.rs         catégories, produits, options, prix par zone, postes
  src/salle.rs             zones, tables
  src/commandes.rs         additions, lignes, envois, annulations, remises
  src/caisse.rs            comptes, sessions, paiements, mouvements, dépenses
  src/stock.rs             articles, conditionnements, mouvements, inventaire
  src/clients.rs           clients et crédit
  src/achats.rs            fournisseurs, achats, règlements
  src/employes.rs          employés, présences, compte employé
  src/paie.rs              bulletins, cotisations facultatives
  src/livraison.rs         livraisons, remise livreur
  src/rapports.rs          indicateurs avec formules
  src/impression.rs        rendu ticket 80 mm + ESC/POS, file d'impression
  src/sauvegarde.rs        sauvegarde, rotation, intégrité, restauration
  src/licence.rs           licence signée Ed25519, code machine
  src/demo.rs              base de démonstration
crates/youma-server/       API HTTP + WebSocket, service de l'interface, imprimantes
crates/youma-licence/      outil fournisseur : clés et licences (hors poste client)
apps/desktop/              coquille Tauri 2 (Windows)
ui/                        React + TypeScript (Vite), tests Vitest et Playwright
docs/                      cahier des charges, conception, décisions
```

## 5. Hors ligne et résilience

* `journal_mode=WAL`, `synchronous=FULL`, `foreign_keys=ON` (fiche 0004) : un encaissement
  validé survit à une coupure.
* Chaque opération métier = **une transaction `BEGIN IMMEDIATE`** (`Db::executer`).
* `PRAGMA quick_check` au démarrage, `integrity_check` à la demande ; échec → écran de restauration.
* Récupération après coupure : additions, envois non imprimés (file d'impression persistée) et
  session de caisse sont en base, donc retrouvés tels quels.
* Sauvegarde `VACUUM INTO` (copie cohérente) : à la clôture de journée, toutes les 30 min en
  service, avant migration ; rotation 7 j / 4 sem / 12 mois ; copie vers un second dossier (clé USB).
* Horloge : refus d'écrire si l'heure du PC est antérieure au dernier événement (RG-SYS-01).
* Côté navigateur (mode B) : saisie en cours conservée localement, bandeau « poste central
  injoignable », reconnexion automatique du WebSocket.

## 6. Synchronisation (V2, fondations posées)

* Un seul maître : le poste central. Identifiants UUIDv7, `cree_le`, `modifie_le`, `version`.
* Toute écriture métier ajoute une ligne dans `outbox` (même transaction) ; l'envoi au cloud
  sera idempotent (clé = id de la ligne outbox).
* Données financières en ajout seul → pas de conflit.
* Données de référence modifiées depuis le cloud : **demande de modification** appliquée par le
  poste central si `version` correspond, sinon rejetée et signalée (dernier écrit local gagne).

## 7. Hypothèses et questions ouvertes

* **[HYPOTHÈSE]** Heure de bascule par défaut : 6 h.
* **[HYPOTHÈSE]** Quantités de stock entières dans l'unité de base (bouteille, pièce, g, ml).
* Taux INPS/AMO : **saisis à la main** par le restaurateur, aucun taux pré-rempli (décision du porteur de projet).
* **[HYPOTHÈSE]** Mois de paie = 26 jours ouvrables pour la déduction d'absence (configurable).
* Pas de factures distinctes : le ticket de caisse payé sert de bon de sortie (fiche 0012).
* Opérateurs Mobile Money : tous proposés (Orange Money, Moov Money, Wave, Sama Money), d'autres ajoutables.
* Question : impression USB par le spouleur Windows à valider sur le matériel du pilote.
