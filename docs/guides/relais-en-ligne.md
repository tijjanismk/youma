# Mettre le relais Youma en ligne

Le relais est **facultatif**. Sans lui, le restaurant vend, encaisse, imprime et prend les commandes par QR sur
le Wi-Fi du restaurant. Il ne sert qu'à :

- la **commande en ligne** (livraison, à emporter) depuis le téléphone du client, n'importe où ;
- le **suivi de la commande** par le client et la **position du livreur** ;
- le **cloud** : sauvegardes chiffrées, espace propriétaire (`/proprietaire`), résumé SMS de fin de journée.

Le poste du restaurant n'a pas besoin d'adresse publique : **c'est lui qui appelle le relais** (toutes les 10 s
pour les commandes, toutes les 5 min pour le cloud). Si Internet coupe au restaurant, le relais affiche
« restaurant fermé » aux clients et garde les commandes déjà passées. Voir les fiches 0013 et 0018.

Deux façons de l'héberger :

- **Railway** (plus simple, aucun serveur à administrer) : section « Héberger sur Railway » ci-dessous ;
- **un VPS** (moins cher, plus de travail) : sections 1 à 4.

**Pas Vercel ni Netlify** : leurs fonctions s'arrêtent entre deux appels et n'ont pas de disque, alors que le
relais tourne en permanence et garde les commandes et les sauvegardes sur disque.

## Héberger sur Railway

Railway construit l'image du relais depuis le dépôt (`railway.toml` et `deploiement/railway/Dockerfile`), fournit
le HTTPS et un disque (volume). Compter environ 5 $ par mois (offre Hobby) pour un relais peu chargé.

1. **Projet** : sur https://railway.com, « New Project » → « Deploy from GitHub repo » → le dépôt Youma.
   Railway lit `railway.toml` : construction par le Dockerfile, contrôle de santé sur `/api/etat`.
2. **Variables** (onglet « Variables » du service) :
   - `YOUMA_RELAIS_CLE` = une clé tirée au hasard (`openssl rand -hex 24`, 16 caractères au moins) ;
   - facultatif, vrais SMS : `YOUMA_ORANGE_CLIENT_ID`, `YOUMA_ORANGE_CLIENT_SECRET`, `YOUMA_ORANGE_EXPEDITEUR`
     (+223…), `YOUMA_ORANGE_NOM_EXPEDITEUR`. Sans elles : simulation.
   Ne pas définir `PORT` : Railway le fournit.
3. **Disque** : clic droit sur le service (ou Ctrl+K) → « Add Volume » → chemin de montage **`/donnees`**.
   Sans volume, les commandes en attente et les sauvegardes disparaissent à chaque redéploiement.
4. **Adresse** : onglet « Settings » → « Networking » → « Generate Domain » (adresse en `….up.railway.app`),
   ou « Custom Domain » pour `commande.exemple.ml` (enregistrement DNS **CNAME** indiqué par Railway ; le
   certificat HTTPS est fourni tout seul).
5. **Vérifier** : `https://ADRESSE/api/etat` répond `{"relais":true,"sms":"simulation"}`.
6. Relier le poste du restaurant : section 4 ci-dessous, avec cette adresse.

**Inscrire un restaurant pour le cloud** : installer l'outil Railway (`npm i -g @railway/cli`), puis
`railway login`, `railway link` (choisir le projet), et :

```sh
railway ssh -- youma-relais --ajouter-restaurant "Maquis Le Baobab" --donnees /donnees
```

**Mises à jour** : Railway reconstruit et redémarre le relais à chaque fusion sur `main` qui touche le relais ou
ses pages (`watchPatterns` de `railway.toml`). Pendant le redémarrage (quelques secondes), le menu en ligne est
indisponible ; les commandes en attente sont sur le volume et le poste renvoie son menu tout seul.

**Plusieurs restaurants avec commandes en ligne** : dans le même projet, « New » → « GitHub Repo » (même dépôt)
pour un second service, avec **sa propre clé**, **son propre volume** `/donnees` et sa propre adresse. Le cloud
reste sur le premier service.

**Journal** : onglet « Deployments » → « View Logs ».

## Héberger sur un VPS

### Ce qu'il faut

- Un **petit serveur Linux** (VPS) allumé en permanence : 1 processeur, 1 Go de mémoire, 10 Go de disque
  suffisent pour plusieurs restaurants (Debian 12 ou Ubuntu 24.04). Environ 4 à 6 € par mois.
- Un **nom de domaine**, par exemple `commande.exemple.ml`, avec un enregistrement DNS **A** vers l'IP du serveur.
- **Caddy** (serveur web) : il fournit le HTTPS tout seul, obligatoire pour la position du livreur.

### 1. Construire le relais

Sur un PC Linux (ou dans la CI), depuis le dépôt :

```sh
cd ui && npm ci && npm run build && cd ..
cargo build --release -p youma-relais
```

On obtient `target/release/youma-relais` (un seul fichier) et `ui/dist` (les pages du client, du livreur et du
propriétaire). Construire sur le serveur lui-même est possible, mais un VPS de 1 Go manque de mémoire pour
compiler : ajouter 2 Go d'espace d'échange (swap) ou construire ailleurs.

### 2. Installer sur le serveur

Copier sur le serveur `target/release/youma-relais`, le dossier `ui/dist` et le dossier `deploiement/relais`
du dépôt, puis, dans le dossier où ils sont :

```sh
sudo useradd --system --no-create-home --shell /usr/sbin/nologin youma
sudo mkdir -p /opt/youma /etc/youma-relais
sudo cp youma-relais /opt/youma/ && sudo cp -r dist /opt/youma/ui
sudo cp deploiement/relais/youma-relais@.service /etc/systemd/system/
sudo cp deploiement/relais/exemple.env /etc/youma-relais/principal.env
sudo chmod 600 /etc/youma-relais/principal.env
```

Dans `/etc/youma-relais/principal.env`, mettre une clé tirée au hasard :

```sh
openssl rand -hex 24      # à copier dans YOUMA_RELAIS_CLE=
```

Puis démarrer :

```sh
sudo systemctl daemon-reload
sudo systemctl enable --now youma-relais@principal
sudo systemctl status youma-relais@principal     # doit indiquer « active (running) »
```

Les données (commandes en attente, sauvegardes chiffrées) sont dans `/var/lib/youma-relais/principal`.

### 3. HTTPS avec Caddy

Installer Caddy (paquet officiel : https://caddyserver.com/docs/install), copier
`deploiement/relais/Caddyfile` dans `/etc/caddy/Caddyfile`, remplacer le nom de domaine, puis :

```sh
sudo systemctl reload caddy
```

Fermer tout le reste au pare-feu : seuls SSH, 80 et 443 restent ouverts (le port 8080 ne doit pas être
joignable depuis Internet, seul Caddy y parle) :

```sh
sudo ufw allow OpenSSH && sudo ufw allow 80 && sudo ufw allow 443 && sudo ufw enable
```

Vérifier depuis n'importe quel navigateur : `https://commande.exemple.ml/api/etat` doit répondre
`{"relais":true,"sms":"simulation"}` (ou `orange_mali` si les identifiants Orange sont renseignés).

## 4. Relier le poste du restaurant (Railway ou VPS)

Sur le poste central du restaurant :

1. **Administration → Commandes à distance** : cocher « Commandes en ligne », puis
   « Adresse du relais » = `https://commande.exemple.ml` et « Clé du relais » = la valeur de `YOUMA_RELAIS_CLE`.
   Enregistrer. Après quelques secondes, la ligne « Relais : dernier contact … » apparaît.
2. Le lien à donner aux clients (affiche, WhatsApp, QR sur les sacs) : `https://commande.exemple.ml/menu`.

Pour le **cloud** (facultatif), inscrire le restaurant sur le serveur :

```sh
sudo -u youma /opt/youma/youma-relais --ajouter-restaurant "Maquis Le Baobab" --donnees /var/lib/youma-relais/principal
```

La commande affiche la **clé du restaurant**. Sur le poste : **Administration → Cloud** : « Adresse du serveur »
= `https://commande.exemple.ml`, « Clé du restaurant » = cette clé, puis le numéro et le mot de passe de
l'espace propriétaire. Noter la phrase de chiffrement sur papier : sans elle, les sauvegardes distantes sont
illisibles, pour tout le monde.

## Plusieurs restaurants

- **Cloud** : une seule instance (`principal`) sert tous les restaurants. Inscrire chacun avec
  `--ajouter-restaurant` ; un même numéro et mot de passe propriétaire regroupe ses restaurants.
- **Commandes en ligne** : aujourd'hui, **une instance par restaurant** (le relais ne connaît qu'une clé).
  Pour un deuxième restaurant : `/etc/youma-relais/baobab.env` avec `YOUMA_PORT=8081` et sa propre clé,
  `sudo systemctl enable --now youma-relais@baobab`, et un bloc `baobab.exemple.ml` dans le Caddyfile.
  Son poste met cette adresse dans « Adresse du relais », et garde `https://commande.exemple.ml` pour le cloud.

## Mettre à jour

Reconstruire (VPS : étape 1), copier le nouveau `youma-relais` et `ui/dist` dans `/opt/youma`, puis
`sudo systemctl restart 'youma-relais@*'`. Les commandes en attente sont conservées ; le poste renvoie son menu
tout seul.

## Sauvegarder le serveur

Tout est dans `/var/lib/youma-relais`. Activer les instantanés (snapshots) du fournisseur du VPS suffit : les
sauvegardes des restaurants y sont déjà chiffrées, et chaque restaurant garde de toute façon ses propres
sauvegardes locales.

## En cas de problème

- `journalctl -u youma-relais@principal -f` : journal du relais.
- Dans Administration → Commandes à distance, le relais reste « jamais joint » ou affiche une erreur : vérifier l'adresse (avec `https://`), Internet au restaurant, et que
  `https://…/api/etat` répond.
- « Clé du relais incorrecte » : la clé du poste et `YOUMA_RELAIS_CLE` diffèrent.
- Le menu en ligne dit « restaurant fermé » : la journée n'est pas ouverte sur le poste, ou le poste n'a pas
  joint le relais depuis plus de 90 s.
