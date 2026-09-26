# Reprise du projet — où on en est (25/09/2026, mis à jour après la refonte des écrans secondaires)

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
- **Interface à jour** (fiche 0025) : React 19, React Router 7, Vite 8 ; Tauri 2.11 (dernière stable).
- **Paie indépendante des caisses** (fiche 0027) : salaires payés depuis le coffre, la banque ou le Mobile Money ;
  avances depuis le tiroir ou hors caisse, au choix.
- **WhatsApp par liens wa.me** (ticket, suivi, résumé au propriétaire) et **ticket seul à l'impression navigateur** (fiche 0028).
- **Photos des plats entières** : import sans recadrage (480 px max, proportions gardées), affichage sans rognure (fiche 0029).
- **Paiement par carte** sur un TPE non relié (fiche 0026) : numéro d'autorisation obligatoire, une fois par jour.
- **Design system « Mali vivant »** (fiche 0022) : palette indigo et mangue, thème clair / sombre au choix du poste.
- **Photos au menu client, listes de villes et quartiers** (fiche 0021) : le menu n'est renvoyé au relais que
  s'il a changé (les photos ne repartent plus toutes les 10 s).

Détail : tableau de `README.md` et `docs/conception/phase-4-plan-realisation.md`.

## Ce qui reste (par ordre conseillé)

1. **Installateur Windows** : construit par la CI (`.github/workflows/installateur-windows.yml`, artefact dans
   l'onglet Actions). Reste : ajouter le secret `YOUMA_CLE_PUBLIQUE` (clé de production des licences), puis
   installer sur une vraie caisse POS et vérifier démarrage, pare-feu et impression.
2. **Guide d'installation sur la caisse** : écrit, `docs/guides/installation-caisse.md`. Reste : captures d'écran
   lors de la première installation réelle, et un guide de formation du personnel (ouverture/clôture, service).
   Le mode réseau s'active dans Administration → Téléphones et tablettes (fiche 0024).
3. **Essais sur vrais appareils** :
   - téléphones Android et iPhone sur le Wi-Fi du restaurant, en service ;
   - imprimantes thermiques chinoises : accents (PC858 après `FS .`), 58 et 80 mm, imprimante intégrée à la caisse
     POS (`windows:NOM`), tiroir-caisse.
4. **Pilote réel d'une semaine** dans un restaurant (mono-poste + imprimante cuisine), puis mode réseau.
   Noter les retours, corriger.
5. **Valider avec des données réelles** : relevés Orange Money, Moov, Wave (formats d'import) ;
   identifiants Orange Developer pour les vrais SMS (aujourd'hui : simulation).
6. **Mettre le relais/cloud en ligne** (facultatif) : guide prêt, `docs/guides/relais-en-ligne.md`
   (VPS, Caddy, service `deploiement/relais/`). Reste : louer le serveur, le nom de domaine, et le faire.
7. ~~Finir la refonte des écrans secondaires~~ : fait (tableau de bord, caisse, stock, paie ; tableaux en
   cartes sur téléphone). Reste éventuellement : achats, clients, rapports, administration (ils profitent déjà
   des tableaux en cartes et du thème).
8. **Plus tard** : relais partagé par plusieurs restaurants pour les commandes en ligne (base du SaaS) ;
   application Android si le raccourci du navigateur ne suffit pas (fiche 0023) ; interface en bambara
   (`ui/src/i18n.ts` est prêt pour la traduction).
9. **PI-SPI** (paiement instantané interopérable de la BCEAO, QR ou numéro de téléphone) :
   - sans code, dès qu'un restaurant a son QR marchand PI-SPI : moyen de paiement « PI-SPI » créé comme un compte
     Mobile Money (référence saisie, unicité RG-CAI-05, rapprochement avec le relevé) ;
   - plus tard, avec une banque ou un opérateur partenaire (seuls les participants accèdent au PI-SPI) : QR avec le
     montant de l'addition et confirmation automatique reçue par le relais. **[HYPOTHÈSE]** Détails à vérifier
     auprès de la BCEAO et du partenaire.

## Réponses reçues (26/09/2026, fiche 0023)

- Imprimantes : **thermiques chinoises** (ESC/POS, 58 ou 80 mm).
- Poste central : le plus souvent une **caisse POS sous Windows**, sans IP publique (inutile : le poste appelle le relais).
- Certificat sur les téléphones : **refusé** → téléphones en HTTP sur le Wi-Fi du restaurant.
- Quartiers : la liste convient, puisqu'on la modifie (quartiers de livraison dans Administration, « Autre… »
  partout ; suggestions dans `ui/src/quartiers.ts`).

## Questions ouvertes (réponses du porteur de projet attendues)


- Modèles précis des imprimantes et des caisses POS du pack (pour les essais).
- Version de Windows des caisses POS (Windows confirmé ; **[HYPOTHÈSE]** 10 ou 11, 64 bits : Windows 7 et 8 ne conviennent pas).
- Restaurant pilote et date de démarrage.
- **[HYPOTHÈSE]** l'adresse locale de la caisse est fixée (réglage sur la caisse ou dans la box).
- Plusieurs restaurants d'un même propriétaire : aujourd'hui regroupés dans l'espace propriétaire (fiche 0018) et
  une instance du relais par restaurant pour les commandes en ligne (fiche 0013). Une page en ligne commune reste à décider.

## Rappels techniques

- Démonstration : `cd ui && npm ci && npm run build && cd .. && cargo run -p youma-server -- --demo --donnees ./donnees --ui ui/dist`
  puis http://127.0.0.1:7878 (PIN propriétaire 1234, mot de passe d'administration « baobab123 »).
- Contrôles avant de pousser et pièges connus : voir `CLAUDE.md`.
