# 0042 — Codes publics limités à 15 essais, clé des licences exigée en production

## Contexte

Suites de la revue du 30/09 (`AI_CONTEXT/ETUDE-MOBILE.md`) ; réponses du porteur de projet (03/10/2026) : « C3 :
15 tentatives » ; pas de compte Apple pour le moment ; « comment je vais générer mes licences ».

- **C3** : `/public/suivi/{code}` et `/public/position/{code}` (poste central et relais) acceptaient des essais sans
  limite : on pouvait chercher des codes de suivi, ou des liens de livreur pour envoyer de fausses positions.
- **C2** : sans `YOUMA_CLE_PUBLIQUE` à la compilation, la caisse accepte les licences signées avec la clé de
  développement, publiée dans le dépôt (`outils/cle-dev.txt`). Rien n'empêchait de compiler ainsi une version de
  production.

## Décision

- **RG-CAN-08** : depuis une même adresse, **15 codes inconnus en 15 minutes** (réponse « introuvable ») bloquent les
  essais suivants sur ces deux routes, jusqu'à ce que les plus anciens aient 15 minutes. Les réponses justes ne
  comptent pas : un client qui suit sa commande ou un livreur en course ne sont jamais gênés. Poste : refus 403
  (`Etat::essais_epuises`, mémoire du serveur) ; relais : 429 `TROP_D_ESSAIS` (même table que ses limites anti-abus).
  Les pages de suivi et du livreur arrêtent de réessayer un code inconnu, pour ne pas se bloquer elles-mêmes.
- **C2** : `crates/youma-server/build.rs` refuse une compilation `release` sans `YOUMA_CLE_PUBLIQUE`, sauf accord
  explicite `YOUMA_CLE_DEV=1` (le workflow Windows le donne pour ses versions d'essai `-dev`, et refuse toujours une
  étiquette `v*` sans le secret). La coquille Tauri dépend du serveur : couverte aussi.
- **`youma-licence`** : clé privée lue dans un fichier (`--cle-privee-fichier`) ou `YOUMA_CLE_PRIVEE` plutôt qu'en
  argument (historique du terminal) ; `generer-cles --fichier` n'écrase jamais une clé existante. Mode d'emploi :
  `docs/guides/licences.md`.
- **iPhone** : pas de compte Apple pour le moment ; le projet iOS reste prêt et compilé par la CI (fiche 0041).

## Alternatives écartées

- Limiter toutes les requêtes publiques (réussies comprises) à 15 : le suivi interroge toutes les 10 à 20 s et le
  livreur envoie sa position toutes les 15 s ; ils seraient bloqués en quelques minutes.
- Bloquer par code plutôt que par adresse : l'énumération essaie justement des codes différents.
- Échec de compilation dans `youma-core` : le relais (Railway) compile aussi le cœur en `release` sans avoir besoin
  des licences ; la règle est posée sur le binaire qui les vérifie, le poste central.

## Conséquences

- Derrière le même accès Internet partagé (box d'un maquis, NAT de l'opérateur), plusieurs clients partagent une
  adresse : 15 codes faux en 15 minutes restent très au-delà d'un usage normal. **[HYPOTHÈSE]** à surveiller au pilote.
- Le compteur du poste vit en mémoire : un redémarrage le remet à zéro (sans importance pour cet usage).
- Construire soi-même un installateur ou un poste `release` demande désormais la clé publique (ou `YOUMA_CLE_DEV=1`).
