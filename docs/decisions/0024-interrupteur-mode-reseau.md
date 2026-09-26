# 0024 — Interrupteur « mode réseau » dans Administration

## Contexte
Le mode réseau (les téléphones des serveurs se connectent à la caisse) se choisissait au démarrage : option
`--reseau` de `youma-server`, ou `"reseau": true` dans `%PROGRAMDATA%\Youma\config.json` pour la coquille Windows.
Sur une caisse POS, il fallait donc éditer ce fichier au Bloc-notes en administrateur : fragile pour un technicien
et impossible pour un propriétaire.

## Décision
* Administration → **Téléphones et tablettes** : case « Mode réseau ». Elle appelle `PUT /api/reseau`
  (`{ "actif": true }`), qui exige `appareil.gerer` confirmé par le mot de passe d'administration (RG-AUT-06),
  trace le changement au journal d'audit (`mode_reseau`) et écrit `config.json` **dans la même transaction**
  (fichier non écrit ⇒ rien n'est tracé). Seule la clé `reseau` est changée : `port`, `demo`… sont gardés.
* Le changement s'applique **au prochain démarrage** de Youma (le serveur choisit son adresse d'écoute au démarrage) ;
  l'écran le dit tant que le choix enregistré diffère de l'état actuel. `GET /api/reseau` renvoie `au_redemarrage`.
* `youma-server` lit aussi `config.json` : `--reseau` **ou** le choix enregistré active le mode réseau.
* Module `crates/youma-server/src/poste.rs` ; la coquille Windows lisait déjà ce fichier et ne change pas.

## Alternatives écartées
* **Basculer sans redémarrer** (fermer puis rouvrir l'écoute sur toutes les interfaces) : plus de code risqué dans le
  serveur (connexions en cours, WebSocket, HTTPS) pour un réglage fait une fois à l'installation.
* **Réglage dans la base** : `config.json` est propre à la machine ; restaurer une sauvegarde sur un autre PC ne doit
  pas ouvrir celui-ci au réseau sans qu'on le décide.

## Conséquences
Le guide d'installation (`docs/guides/installation-caisse.md`) n'édite plus de fichier à la main. Sans le module
« réseau » de la licence, le mode s'active mais les téléphones sont refusés (message clair sur le téléphone).
