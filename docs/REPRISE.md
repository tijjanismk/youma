# Reprise du projet — où on en est (25/09/2026)

Fiche à lire en premier pour reprendre après une pause. Tout le code est sur `main` (PR #1 et #2 fusionnées),
tous les tests passent (CI verte).

## Pour reprendre avec Claude

Ouvrir une session sur le dépôt `tijjanismk/youma` et coller :

> Lis `CLAUDE.md` et `docs/REPRISE.md`, puis continue avec le point N de « Ce qui reste ».

## Ce qui est fait

- **MVP 0 et MVP 1** : ventes, salle, cuisine, caisse, Mobile Money, stock, clients et crédit, employés et paie,
  rapports, sauvegardes, licence, mode réseau, bon de sortie.
- **V2** : commandes à distance (QR, en ligne, relais Internet facultatif, SMS Orange Mali en simulation),
  recettes et coût matière, consignes, relevés Mobile Money, promotions, statistiques, cloud facultatif
  (résumés, sauvegardes chiffrées, espace propriétaire).
- **Interface refaite** (fiche 0019) : thème, menu repliable sur PC, écrans téléphone, photos des plats.
- **Application installable (PWA)** (fiche 0020) : icône sur le téléphone, HTTPS du réseau local.

Détail : tableau de `README.md` et `docs/conception/phase-4-plan-realisation.md`.

## Ce qui reste (par ordre conseillé)

1. **Installateur Windows** : construire la coquille Tauri (`apps/desktop`) sur un PC Windows et produire
   l'installateur hors ligne ; ajouter si possible un job Windows à la CI.
2. **Guide d'installation et de formation** (français simple, avec captures) : installation du PC, IP fixe
   dans la box, imprimantes, appairage des téléphones, installation du certificat et de l'application,
   ouverture/clôture de journée.
3. **Essais sur vrais appareils** :
   - téléphones Android et iPhone : installation du certificat et de la PWA, usage en service ;
   - imprimantes du pack matériel : page de code PC858, impression USB sous Windows.
4. **Pilote réel d'une semaine** dans un restaurant (mono-poste + imprimante cuisine), puis mode réseau.
   Noter les retours, corriger.
5. **Valider avec des données réelles** : relevés Orange Money, Moov, Wave (formats d'import) ;
   identifiants Orange Developer pour les vrais SMS (aujourd'hui : simulation).
6. **Mettre le relais/cloud en ligne** (facultatif) : petit serveur (VPS) derrière Caddy en HTTPS,
   inscription des restaurants (`youma-relais --ajouter-restaurant`).
7. **Finir la refonte des écrans secondaires** : tableau de bord, caisse, stock, paie ont le nouveau thème
   mais pas encore de mise en page dédiée comme la prise de commande.
8. **Plus tard** : interface en bambara (`ui/src/i18n.ts` est prêt pour la traduction).

## Questions ouvertes (réponses du porteur de projet attendues)

- Modèles d'imprimantes du pack matériel.
- Restaurant pilote et date de démarrage.
- **[HYPOTHÈSE]** installer un certificat sur chaque téléphone est acceptable (fiche 0020).
- **[HYPOTHÈSE]** le PC du restaurant aura une IP fixe (bail DHCP réservé).

## Rappels techniques

- Démonstration : `cd ui && npm ci && npm run build && cd .. && cargo run -p youma-server -- --demo --donnees ./donnees --ui ui/dist`
  puis http://127.0.0.1:7878 (PIN propriétaire 1234, mot de passe d'administration « baobab123 »).
- Contrôles avant de pousser et pièges connus : voir `CLAUDE.md`.
