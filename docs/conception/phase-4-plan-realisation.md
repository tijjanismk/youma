# Phase 4 — Plan de réalisation, tests, risques

## 1. État de réalisation

| Étape | Contenu | Testable en réel | État |
|---|---|---|---|
| 1 | Cœur, schéma, journée, auth PIN, catalogue, salle | démo en mono-poste | ✅ |
| 2 | Commandes, tournées, envois, tickets ESC/POS, cuisine | service réel avec imprimante réseau | ✅ (impression USB Windows non testée sur matériel) |
| 3 | Caisse : sessions, paiements mixtes, Mobile Money, dépenses, clôture Z | une journée complète au pilote | ✅ |
| 4 | Stock des boissons, achats au marché, inventaire du soir | inventaire réel des boissons | ✅ |
| 5 | Clients et crédit, fournisseurs et dettes | ardoises réelles | ✅ |
| 6 | Employés sans contrat, présences, avances, paie, bulletins | une paie de fin de mois | ✅ |
| 7 | Mode réseau : appairage des téléphones, WebSocket, écran cuisine | 2 téléphones + 1 tablette | ✅ (à éprouver sur le Wi-Fi du pilote) |
| 8 | Rapports, CSV, sauvegarde/restauration, licence, diagnostic | clôture + restauration sur un autre PC | ✅ |
| 9 | Livraison et remise livreur | livraisons en moto | ✅ (base) |
| 10 | Coquille Windows, installateur hors ligne | installation chez le pilote | ⚠️ code prêt, installateur à construire sur Windows |
| 11 | Canaux de commande : téléphone, QR sur la table, en ligne ; file de validation, zones à risque, liste noire, suivi en direct (fiche 0013) | QR collés sur les tables, 1 journée | ✅ réseau local et relais Internet facultatif (`youma-relais`) |
| V2 | Recettes, consignes, cloud, relevés MM, promotions, multi-sites | — | ❌ conçu, non implémenté |

Prochaine étape recommandée : **pilote réel d'une semaine** (mono-poste + imprimante cuisine), puis mode réseau.

## 2. Stratégie de tests

| Niveau | Outil | Où | Contenu |
|---|---|---|---|
| Règles métier | `cargo test` | `crates/youma-core/tests/regles.rs` | une ou plusieurs assertions par `RG-*` (27 tests) |
| Scénarios §27 | `cargo test` | `crates/youma-core/tests/scenarios.rs` | les 15 scénarios d'acceptation (16 tests) |
| Coupure | `cargo test` | `s01_coupure_processus_tue` | processus **tué** en pleine transaction, base rouverte : intègre, rien d'écrit à moitié |
| Unitaires | `cargo test` | modules du cœur | journée après minuit, arrondis, division, format FCFA, ESC/POS |
| API | `cargo test` + reqwest | `crates/youma-server/tests/api.rs` | vrai serveur : parcours de service, erreurs, PIN responsable, WebSocket, appairage, paie, commandes QR/en ligne, poste central ↔ relais de bout en bout |
| Relais | `cargo test` | `crates/youma-relais/tests/relais.rs` | synchronisation, code SMS (faux fournisseur), suivi, position du livreur, anti-abus |
| Impression | `cargo test` | `youma-server/src/imprimantes.rs` | fichier, TCP ESC/POS (faux serveur 9100), ouverture du tiroir |
| Interface (logique) | Vitest | `ui/src/logique.test.ts` | panier local, paiement mixte, rendu, formats |
| Interface (composants) | Vitest + Testing Library | `ui/src/composants.test.tsx` | PinPad, connexion, rejeu avec PIN du responsable, hors ligne |
| Canaux, zones | `cargo test` | `crates/youma-core/tests/canaux.rs` | RG-CAN-01 à 05, RG-ZON-01 à 03, RG-LIV-04 (12 tests) |
| Bout en bout | Playwright (Chromium) | `ui/e2e/service.spec.ts` | 12 parcours réels : journée, service à table, cuisine, annulation avec PIN, coupure du poste central, employé sans contrat, INPS/AMO, tableau de bord, clôture Z, téléphone, commande QR et en ligne avec zone à risque et suivi |

Commandes :

```bash
cargo test --workspace            # cœur + serveur
cd ui && npm test                 # Vitest
cd ui && npm run build && npx playwright test   # bout en bout (démarre youma-server --demo)
```

À ajouter avant la mise en production : tests de coupure sur PC réel (débrancher pendant une clôture),
tests de charge (300 commandes/jour sur 4 Go de RAM et disque mécanique), tests sur imprimantes du pack matériel.

## 3. Risques principaux et parades

| Risque | Parade |
|---|---|
| Coupure pendant une écriture | WAL + `synchronous=FULL`, transactions uniques, test de processus tué, sauvegardes automatiques |
| Horloge BIOS remise à zéro | garde d'horloge RG-SYS-01, journée d'exploitation explicite |
| Disque plein / antivirus | alerte disque, rotation des sauvegardes, dossier de données hors `Program Files` |
| Fraude Mobile Money (faux SMS, capture réutilisée) | référence obligatoire et unique (RG-CAI-05), liste « à vérifier » |
| Détournements (annulations, remises, offerts) | motifs, PIN du responsable, journal d'audit, rapport des annulations |
| Imprimante incompatible (page de code) | file persistante, réimpression, [HYPOTHÈSE] PC858 à valider sur le pack matériel |
| Wi-Fi du restaurant instable (mode B) | panier local conservé, reconnexion WebSocket, bandeau d'alerte |
| Paie contestée | journal du compte employé, bulletins figés, régularisations tracées |
| Adoption par un personnel peu à l'aise avec l'écrit | grands boutons, couleurs et icônes de catégories, PIN au lieu de mots de passe |
| Équipe réduite | un seul langage métier (Rust), un seul chemin d'appel (HTTP), tests automatisés |

## 4. Questions tranchées par le porteur de projet

| Question | Réponse | Mise en œuvre |
|---|---|---|
| Opérateurs Mobile Money | tous | Orange Money, Moov Money, Wave, Sama Money par défaut ; ajout/désactivation dans Administration → Moyens de paiement |
| Taux INPS/AMO | saisis à la main | aucun taux pré-rempli, activation impossible sans taux (RG-PAI-07) |
| Mot de passe pour l'administration | oui | RG-AUT-06, fiche 0011 |
| Factures distinctes des tickets | non : bon de sortie | ticket de caisse payé = bon de sortie avec code de contrôle, écran « Contrôle de sortie » (RG-SOR-*, fiche 0012) |
| Bambara | plus tard | architecture de traduction conservée (`ui/src/i18n.ts`) |
| Commandes en ligne, suivi, zones à risque | oui, relais Internet facultatif ; zones par quartier et cercle GPS ; paiement d'avance et à la livraison ; vérification du numéro | fiche 0013 |

## 5. Questions encore ouvertes

1. Modèles d'imprimantes du pack matériel (pour valider la page de code PC858 et l'impression USB).
2. Restaurant pilote et date de démarrage.
