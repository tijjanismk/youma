# Étude mobile et revue de sécurité du 30/09/2026 — confrontées au code du 03/10/2026

Sources (livrable du porteur, non versionné dans le dépôt) : `rapport-discussion.md`,
`architecture-mobile-possibilites.md`, `architecture-native-flutter-react-native.md`, `LISEZMOI.md`.
Les schémas SVG cités (`schemas/*.svg`) ne faisaient pas partie des fichiers reçus. **Aucun code écrit pour cette étude.**

## 1. Résumé de l'étude mobile

| Option | Contenu | Effort annoncé | Verdict de l'étude |
| --- | --- | --- | --- |
| 1. Statu quo | Navigateur + raccourci (fiches 0020, 0023), HTTP local | 0 | Garder si le pilote montre que ça suffit |
| 2a. Coquille Capacitor, personnel | `ui/dist` embarqué, plein écran, Retour maîtrisé, écran allumé ; APK servi par la caisse | 2–3 j [HYPOTHÈSE] | **Recommandée** si une app est voulue |
| 2b. + livreur | Position en arrière-plan (service de premier plan) | +1–2 j [HYPOTHÈSE] | Si le pilote montre des coupures |
| 3. Réécriture native | React Native/Expo (réutilise types, `api.ts`, logique d'état) ou Flutter | plusieurs mois | Aucun besoin ne la justifie |
| 4. Mixte | Web pour le personnel, mini-app livreur | 1–2 j | Cible le seul besoin hors web (L4) |

Écartées : TWA/Bubblewrap (HTTPS public requis), ventes hors ligne dans le téléphone (contraire à la fiche 0003),
ressources chargées depuis Internet.

Invariants rappelés (tous vérifiés dans `CLAUDE.md` / code) : un seul écrivain (poste central), aucune
installation obligatoire pour prendre une commande, appairage `X-Appareil` comme contrôle d'accès, HTTP sur le
Wi-Fi, pages publiques par le relais HTTPS.

## 2. Ce que la coquille (option 2) demanderait — vérifié dans le code

| Point de l'étude | État actuel | Où |
| --- | --- | --- |
| Adresse API configurable | Appels relatifs `/api…` | `ui/src/api.ts:105` |
| WebSocket hors `location.host` | Construit sur `location.host`, jeton en paramètre d'URL | `ui/src/contexte.tsx:101–102` |
| Appairage par saisie d'adresse + code | Code lu dans l'URL `?appairage=` | `ui/src/App.tsx:108–116` |
| CORS sur `/api` en mode réseau | Absent (aucun `cors` dans `youma-server`) | `crates/youma-server/src` |
| Service worker inactif en natif | Déjà inactif hors contexte sécurisé | `ui/src/pwa.ts:29` |
| Export CSV par partage natif | `fetch` + lien de téléchargement | `ui/src/api.ts:140` |
| Version du serveur exposée | `/api/etat` renvoie `version` (`CARGO_PKG_VERSION`) | `crates/youma-server/src/api.rs:80,285` |

À décider avant tout APK (étude § 7) : politique de versions interface/serveur, garde de la clé de signature,
canal de distribution (caisse / WhatsApp / Play Store), iPhone oui/non, CORS restreint, HTTP clair limité aux
plages privées. Numérotation : l'étude propose la fiche **0039**, déjà prise (fidélité) → ce serait **0040**.

## 3. Constats de sécurité — état au 03/10

| Id | Constat | État | Preuve |
| --- | --- | --- | --- |
| C1 | PIN d'autorisation ponctuelle sans verrouillage (force brute de 10 000 PIN par un employé connecté) | **Ouvert** — seule faille jugée exploitable | `crates/youma-core/src/auth.rs:107` n'utilise ni `echecs_pin` ni `verrouille_jusqu_a` (utilisés l. 323–346 et 416–437) |
| C2 | Repli sur `CLE_DEV` si `YOUMA_CLE_PUBLIQUE` absente ; `outils/cle-dev.txt` dans le dépôt | **Atténué** : le workflow Windows échoue pour une version `v*` sans le secret ; une compilation release locale retombe encore sur la clé de dev | `licence.rs:18–21`, `.github/workflows/installateur-windows.yml:35–48` |
| C3 | `/public/suivi/{code}` et `/public/position/{code}` sans limiteur sur le poste | **Ouvert** (risque faible) | `crates/youma-server/src/api.rs:256–258` |
| C4 | Export/restauration de sauvegarde avec chemin libre fourni par le client | **Ouvert** (réservé SAUVEGARDE_GERER + session élevée) | `api.rs:1341–1350` |
| C5 | Clé privée de l'autorité TLS en clair dans la base | Compromis assumé (fiche 0020) | — |
| — | `cargo audit` absent de la CI | **Ouvert** | `.github/workflows/ci.yml` |

Observation ajoutée (pas dans le rapport) : le jeton de session passe en paramètre d'URL du WebSocket
(`contexte.tsx:102`), donc visible dans les journaux d'un proxy éventuel. Mineur sur le Wi-Fi local.

Alertes du script de carte (`ALERTES.md`, heuristiques à juger) : `format!` dans du SQL (interpolations de
constantes selon le rapport — à confirmer au cas par cas), `unsafe` sans `// SAFETY:` (`imprimantes.rs:60`),
appels bloquants en async (`api.rs:1310`, `server/cloud.rs:124,177`, `relais/cloud.rs:139,182`), `unwrap` hors tests
(`auth.rs:21`, `horloge.rs:54`, `apps/desktop/.../main.rs:56`). Dépendance `hyper` déclarée non utilisée dans `youma-server`.

## 4. Localisation des commandes à distance — état au 03/10

| Id | Problème | État | Preuve |
| --- | --- | --- | --- |
| L1 | Page en HTTP : géolocalisation refusée, message générique trompeur | **Ouvert** : `MenuClient.tsx` ne teste pas `isSecureContext` (le livreur oui) | `ui/src/public/MenuClient.tsx:376`, `Livreur.tsx:15` |
| L2 | Permission refusée mémorisée, même message pour toutes les erreurs | **Ouvert** : le gestionnaire d'erreur ignore le code (`PERMISSION_DENIED`/`TIMEOUT`…) | `MenuClient.tsx:374–378` |
| L3 | Position imprécise | **En partie traité** (fiche 0038) : GPS haute précision, meilleure mesure sur 30 s, arrêt ≤ 25 m, précision affichée, repère facultatif si ≤ 100 m. **Reste** : précision non envoyée au poste ; `zones_risque::evaluer` applique le cercle GPS sans elle | `MenuClient.tsx:355–390`, `zones_risque.rs` |
| L4 | Position du livreur coupée écran verrouillé | Limite du navigateur ; relève des options 2b/4 | — |

Proposition déjà faite au porteur et non confirmée : cacher « Partager ma position » sur une page HTTP et demander
directement le repère (correspond au correctif L1).

## 5. Questions ouvertes pour le porteur (reprises de l'étude)

1. Le raccourci navigateur a-t-il été essayé par du personnel réel ? Qu'est-ce qui gêne ?
2. Nombre de téléphones par restaurant, versions d'Android, iPhones en service ?
3. Livreur salarié (app imposable) ou occasionnel (web) ?
4. Impression depuis un téléphone demandée (Bluetooth) ?
5. C1 : verrouiller la session/l'appareil appelant (recommandé) ou l'utilisateur cible ?
6. L3 : transmettre la précision au poste (nouveau champ `livraison.precision_m`) ?
7. Option 3 si jamais retenue : React Native/Expo (recommandé, même langage) ou Flutter ?

## 6. Ordre suggéré (si le porteur valide)

1. C1 (fiche + test dans `regles.rs`), L1 + L2 (faible coût, interface seulement).
2. C3, C4, `cargo audit` en CI, échec de compilation release sans clé (C2).
3. Décision mobile (fiche 0040) après retour du pilote ; L3 avec la décision sur la précision.
