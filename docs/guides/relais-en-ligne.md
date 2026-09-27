# Mettre le relais Youma en ligne

Le relais est **facultatif**. Sans lui, le restaurant vend, encaisse, imprime et prend les commandes par QR sur
le Wi-Fi du restaurant. Il ne sert qu'à :

- la **commande en ligne** (livraison, à emporter) depuis le téléphone du client, n'importe où ;
- le **suivi de la commande** par le client et la **position du livreur** ;
- le **cloud** : sauvegardes chiffrées, espace propriétaire (`/proprietaire`), résumé SMS de fin de journée.

Le poste du restaurant n'a pas besoin d'adresse publique : **c'est lui qui appelle le relais** (toutes les 10 s
pour les commandes, toutes les 5 min pour le cloud). Si Internet coupe au restaurant, le relais affiche
« restaurant fermé » aux clients et garde les commandes déjà passées. Voir les fiches 0013 et 0018.

Trois façons de l'héberger :

- **Railway** (plus simple, aucun serveur à administrer, environ 5 $ par mois) : « Héberger sur Railway » ;
- **Oracle Cloud « Always Free »** (gratuit, un vrai serveur à administrer) : « Héberger gratuitement sur Oracle
  Cloud » ;
- **un VPS** payant (4 à 6 € par mois) : sections 1 à 4.

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

## Héberger gratuitement sur Oracle Cloud

L'offre **Always Free** d'Oracle fournit sans limite de durée une machine Linux allumée en permanence, avec un
disque qui garde les données : c'est un VPS gratuit. Les étapes ci-dessous reprennent la partie VPS, avec ce
qui change chez Oracle (pare-feu, processeur ARM, nom DuckDNS).

**[HYPOTHÈSE]** Offre, limites et noms des menus relevés en septembre 2026, à vérifier à l'inscription : les offres
gratuites changent souvent.

### À savoir avant de commencer

- Une **carte bancaire** est demandée à l'inscription, pour vérifier l'identité ; rien n'est prélevé tant qu'on
  reste dans l'offre gratuite.
- La **région d'origine** se choisit à l'inscription et ne se change plus : prendre une région d'Europe (Paris,
  Marseille, Madrid, Francfort…), proche du Mali.
- Oracle peut **récupérer une machine gratuite jugée inactive** (processeur, réseau et mémoire presque au repos
  pendant 7 jours). Un relais peu chargé peut être dans ce cas. Pour l'éviter : passer le compte en
  **« Pay As You Go »** (Facturation → Mettre à niveau) ; les ressources Always Free restent gratuites.
  Prévoir aussi une alerte de budget à 1 € pour être prévenu de toute facturation.
- Les machines gratuites les plus confortables sont en **ARM** (Ampere). On compile donc le relais **sur la
  machine elle-même** (le fichier construit sur un PC ordinaire ne tourne pas sur ARM).

### 1. Créer la machine

1. Console Oracle Cloud → **Compute → Instances → Create instance**.
2. **Image** : Ubuntu 24.04 (ou 22.04). **Shape** : `VM.Standard.A1.Flex` (Ampere), **1 OCPU et 6 Go** de
   mémoire suffisent (l'offre gratuite en permet jusqu'à 4 et 24 Go).
   Si Oracle répond « Out of capacity », réessayer plus tard ou dans un autre domaine de disponibilité ; en
   dépannage, la shape `VM.Standard.E2.1.Micro` (x86, 1 Go, toujours gratuite) convient aussi, avec de l'espace
   d'échange pour compiler (voir la section VPS, étape 1).
3. **Réseau** : laisser créer le réseau virtuel (VCN) avec une **adresse IPv4 publique**.
4. **Clé SSH** : télécharger la clé privée proposée (ou donner la sienne) et la garder précieusement.
5. Créer, puis noter l'**adresse IP publique** de la machine.

Se connecter : `ssh -i cle.key ubuntu@ADRESSE_IP`.

### 2. Ouvrir les ports 80 et 443

Il y a **deux pare-feu** à ouvrir ; oublier l'un ou l'autre est la cause la plus fréquente d'échec.

1. **Côté Oracle** : page de l'instance → sous-réseau → **Security List** par défaut → **Add Ingress Rules** :
   source `0.0.0.0/0`, protocole TCP, port de destination `80` ; recommencer pour `443`.
2. **Sur la machine** : les images Ubuntu d'Oracle bloquent tout sauf SSH avec `iptables` (ne pas utiliser `ufw`,
   qui entre en conflit) :

```sh
sudo iptables -I INPUT 6 -m state --state NEW -p tcp --dport 80 -j ACCEPT
sudo iptables -I INPUT 6 -m state --state NEW -p tcp --dport 443 -j ACCEPT
sudo netfilter-persistent save
```

Le port 8080 du relais reste fermé : seul Caddy y parle, depuis la machine.

### 3. Un nom gratuit avec DuckDNS

1. Sur https://www.duckdns.org, se connecter et créer un sous-domaine, par exemple `baobab` →
   `baobab.duckdns.org`.
2. Mettre dans « current ip » l'**adresse IP publique** de la machine, puis « update ip ».

L'adresse publique d'une instance Oracle ne change pas tant qu'on ne supprime pas la machine. Un vrai nom de
domaine (environ 10 € par an) peut remplacer DuckDNS plus tard : enregistrement **A** vers la même adresse.

### 4. Compiler et installer le relais sur la machine

Sur la machine (ARM) :

```sh
sudo apt update && sudo apt install -y build-essential pkg-config git
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
. "$HOME/.cargo/env"
git clone https://github.com/tijjanismk/youma.git && cd youma   # dépôt privé : copier le dossier avec scp
cargo build --release -p youma-relais                          # 10 à 20 minutes la première fois
```

Les pages du client (`ui/dist`) ne dépendent pas du processeur : les construire sur un PC
(`cd ui && npm ci && npm run build`) et copier le dossier `ui/dist` dans `~/youma/ui/dist` sur la machine
(`scp -r -i cle.key ui/dist ubuntu@ADRESSE_IP:~/youma/ui/`).

Puis, dans `~/youma`, installer comme sur un VPS (section 2) :

```sh
sudo useradd --system --no-create-home --shell /usr/sbin/nologin youma
sudo mkdir -p /opt/youma /etc/youma-relais
sudo cp target/release/youma-relais /opt/youma/ && sudo cp -r ui/dist /opt/youma/ui
sudo cp deploiement/relais/youma-relais@.service /etc/systemd/system/
sudo cp deploiement/relais/exemple.env /etc/youma-relais/principal.env
sudo chmod 600 /etc/youma-relais/principal.env
openssl rand -hex 24      # à copier dans YOUMA_RELAIS_CLE= de /etc/youma-relais/principal.env
sudo systemctl daemon-reload && sudo systemctl enable --now youma-relais@principal
```

### 5. HTTPS avec Caddy

Installer Caddy (https://caddyserver.com/docs/install, paquet Ubuntu), copier `deploiement/relais/Caddyfile`
dans `/etc/caddy/Caddyfile` en remplaçant `commande.exemple.ml` par `baobab.duckdns.org`, puis
`sudo systemctl reload caddy`. Caddy obtient le certificat tout seul (ports 80 et 443 ouverts, étape 2).

Vérifier depuis un téléphone : `https://baobab.duckdns.org/api/etat` doit répondre
`{"relais":true,"sms":"simulation"}`. Relier ensuite le poste du restaurant (section 4).

### 6. Surveiller (gratuit)

Sur https://uptimerobot.com, un moniteur HTTP(S) sur `https://baobab.duckdns.org/api/etat` : un e-mail arrive si le
relais ne répond plus. Mises à jour : `git pull`, `cargo build --release -p youma-relais`, recopier le fichier et
`ui/dist` dans `/opt/youma`, puis `sudo systemctl restart 'youma-relais@*'`.

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

## 4. Relier le poste du restaurant (Railway, Oracle ou VPS)

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

- Oracle : `https://…/api/etat` ne répond pas depuis Internet mais `curl http://127.0.0.1:8080/api/etat` répond sur
  la machine : un des deux pare-feu est fermé (Security List d'Oracle, ou `iptables` sur la machine, étape 2).

- `journalctl -u youma-relais@principal -f` : journal du relais.
- Dans Administration → Commandes à distance, le relais reste « jamais joint » ou affiche une erreur : vérifier l'adresse (avec `https://`), Internet au restaurant, et que
  `https://…/api/etat` répond.
- « Clé du relais incorrecte » : la clé du poste et `YOUMA_RELAIS_CLE` diffèrent.
- Le menu en ligne dit « restaurant fermé » : la journée n'est pas ouverte sur le poste, ou le poste n'a pas
  joint le relais depuis plus de 90 s.
