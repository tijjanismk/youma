# Domaine — règles métier de Youma

Source de vérité des règles : `docs/conception/phase-2-donnees-regles.md` (énoncés RG-XXX-NN) et le code qui les
cite. Ici : où chaque famille est appliquée. Chemins relatifs à `crates/youma-core/src/` sauf mention.
[CONFIRMÉ] = lu dans le code à la ligne citée ; [DÉDUIT] = à revérifier.

## Invariants transverses

- [CONFIRMÉ] Toute écriture passe par `Db::executer` : transaction `BEGIN IMMEDIATE` (RG-SYS-02) et garde d'horloge
  (RG-SYS-01) — `db.rs:204`. Permissions par `op.exiger(perm::…)` — `db.rs:324`, liste dans `permissions.rs`.
- [CONFIRMÉ] Tables financières et de stock en ajout seul : un UPDATE/DELETE refusé par trigger remonte en
  RG-SYS-03 — `erreur.rs:75`. Correction = contre-passation (ex. RG-CAI-07 `caisse.rs:667`).
- [CONFIRMÉ] Numéros séquentiels sans trou dans la transaction (RG-SYS-04) — `db.rs:356`.
- [CONFIRMÉ] La licence ne conditionne que les modules, jamais la vente (RG-SYS-06) — `licence.rs:2`.
- [CONFIRMÉ] Ventes rattachées à la journée d'exploitation ouverte (RG-JOU-01) — `db.rs:414` ; blocages de clôture
  affichés (RG-JOU-04) — `journee.rs:102`.
- [CONFIRMÉ] Montants en `i64` FCFA ; soldes = somme des mouvements (ex. `fidelite.rs:14`, `contrats.rs:40`).
- [CONFIRMÉ] Migrations qui reconstruisent une table référencée : marqueur `-- youma:cles-etrangeres-coupees`
  (clés étrangères coupées puis `foreign_key_check`) — `db.rs:30`, `db.rs:121`.

## Familles de règles → lieu d'application (première citation)

| Famille | Sujet | Où |
| --- | --- | --- |
| AUT 01–08 | PIN Argon2, verrouillage 5 échecs (02), autorisation ponctuelle par PIN (03), dernier propriétaire (05), élévation par mot de passe (06), secours (07), PIN d'autorisation : 5 faux en 15 min bloquent le demandeur 5 min (08) | `auth.rs` (`autoriser_par_pin`), `db.rs::executer_interne`, `secours.rs` |
| SYS 01–06 | horloge, transaction, ajout seul, séquences, audit, licence | `db.rs`, `erreur.rs`, `licence.rs` |
| JOU 01–04 | journée d'exploitation, clôture | `journee.rs`, `db.rs:414` |
| CMD 01–12 | commandes, envoi par poste (03), article envoyé non modifiable (04), parts égales (12) | `commandes.rs:145,593,607,1092`, `entrantes.rs:512,664` |
| CAT 01–08 | catalogue, menu du jour (07), produit revendu → article de stock (06), ruptures levées à la nouvelle journée (08) | `catalogue.rs:149,155,276,359,452` |
| CAI 01–17 | encaissement multi-parts (01–06), annulation de paiement (07), sessions et billetage (08–14), TPE (15), remise au coffre à la clôture (16) ou à l'ouverture si le tiroir a été vidé (17) | `caisse.rs:350,485–556,667,755,798,849` |
| CLI 01–04 | crédit client, plafond, relevé, téléphone unique (04) | `clients.rs:21,100,193`, `caisse.rs:485` |
| FID 01–04 | points : réglages (01), gain au solde de l'addition (02), utilisation = remise (03), retrait à l'annulation (04) | `parametres.rs:50`, `caisse.rs:643,725`, `fidelite.rs:86,117` |
| CAD 01–05 | cartes cadeaux / bons d'avoir : code (02), expiration (03), avoir avec motif + accord gérant (04), recrédit à l'annulation (05) | `cartes.rs:80,193,222,236` |
| SOC 01–04 | contrats société : crédit autorisé (01), part plafonnée + employé nommé (02/03), relevé (04) | `contrats.rs:69,121,154`, `caisse.rs:457` |
| VIP 01–03 | client privilégié (01), tête de file des entrantes (02), priorité cuisine des commandes à distance (03) ; tables : ordre d'arrivée | `fidelite.rs:187`, `entrantes.rs:556`, `commandes.rs:1017` |
| CAN 01–08 | canaux QR / en ligne : QR inconnu (02), liste noire (03), numéro vérifié une fois par SMS ou WhatsApp (04, fiche 0046), paiement/plafond (05), modifications client (06), commande renvoyée une seule fois (07, fenêtre 5 min) | `entrantes.rs:126,230,320,368,458`, relais `youma-relais/src/lib.rs` |
| ZON 01–03 | zones à risque (quartier et/ou cercle GPS) | `zones_risque.rs:86`, `entrantes.rs:206`, `commandes.rs:462` |
| LIV 01–05 | livraison, remise du livreur (03), position pendant la course (04), accès du livreur par téléphone + PIN et courses publiées au relais (05, fiche 0047) | `livraison.rs` (`livreurs_relais`), `employes.rs::definir_pin_livreur`, `entrantes.rs:867`, relais `connexion_livreur` |
| STK 01–07, REC 01–04 | stock (sortie à l'envoi, retour à l'annulation, inventaire), recettes et coût matière | `stock.rs:180,225,279,395`, `recettes.rs:38` |
| ACH 01–05, CON 01–06 | achats (marché par défaut), consignes d'emballages | `achats.rs:88–209`, `consignes.rs` |
| EMP 01–07, PAI 01–09 | employés, présences, avances, paie | `employes.rs`, `paie.rs:88,155,240,386` |
| RMM 01–05 | relevés Mobile Money rapprochés | `releves_mm.rs:157,235,264` |
| PRO, STA, RAP | promotions, statistiques, rapports ; commandes non honorées (RAP-04) | `promotions.rs`, `rapports.rs` (`rapport_non_honorees`) |
| AVI 01–03 | avis des clients : note 1–5 par code de suivi, une fois, commande terminée, 7 jours (01) ; avis ≤ 2 à traiter par le gérant (02) ; rapport (03) | `avis.rs`, relais `enregistrer_avis` |
| SOR 01–03 | contrôle de sortie par code du ticket | `sortie.rs:39`, `impression.rs:399` |
| CLO 01–05 | cloud facultatif chiffré | `cloud.rs`, `youma-relais/src/cloud.rs:222` |

## Hypothèses en cours (fiche 0039)

- [CONFIRMÉ] Points gagnés sur les parts hors contrat société et hors bon d'avoir — `fidelite.rs:93–97`.
- [DÉDUIT] Une carte cadeau payée par une autre carte cadeau rapporte des points (noté [HYPOTHÈSE] dans la fiche 0039).
