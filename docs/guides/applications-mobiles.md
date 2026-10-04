# Applications Youma Client et Youma Livreur

Deux applications pour téléphone, **en plus** des pages web, qui restent en service (fiche 0041) :

- **Youma Client** : le client garde ses restaurants, commande en ligne, suit sa commande et reçoit une
  notification à chaque étape (tant que l'application tourne).
- **Youma Livreur** : le livreur ouvre sa course et sa position est envoyée au client **même écran verrouillé**,
  jusqu'à « Arrêter » ou la fin de la course. Une notification « Course en cours » reste affichée pendant ce temps.

Les deux applications parlent au **relais en ligne** du restaurant (`https://…`, voir `relais-en-ligne.md`),
jamais au poste de caisse. Sans relais en ligne, elles ne servent à rien : le restaurant garde les QR de table.

## 1. Récupérer les applications Android (APK)

GitHub construit les APK à chaque changement : dépôt → **Actions** → « Applications mobiles » → dernière
exécution → **Artifacts** : `youma-client-android` et `youma-livreur-android`. Pour une version numérotée
(étiquette `v…`), les APK sont joints à la page **Releases**.

- Fichier terminé par **`-essai.apk`** : construit sans clé de signature (essais seulement). On ne pourra pas le
  mettre à jour avec la version signée : il faudra le désinstaller d'abord.
- Sans `-essai` : version signée, à diffuser.

## 2. Installer sur un téléphone Android

1. Envoyer l'APK par WhatsApp (au livreur, ou aux clients fidèles) ou le déposer sur le téléphone.
2. Ouvrir le fichier. Android demande d'**autoriser l'installation depuis cette source** (WhatsApp, Fichiers) :
   accepter une fois.
3. Ouvrir l'application :
   - **Client** : coller le lien de commande en ligne du restaurant (`https://…/menu`). Le restaurant est gardé
     dans « Mes restaurants ».
   - **Livreur** : se connecter avec l'**adresse du restaurant** (celle du menu en ligne, `https://…`), son
     **téléphone** et le **PIN** donné par le restaurant (fiche du livreur → Application Youma Livreur, fiche 0047).
     Ses courses apparaissent toutes seules : **Démarrer la course**. Sans accès, on peut encore coller le lien
     « livreur » d'une course. À la première course, accepter la position (**« Toujours autoriser »** si Android
     le propose) et les notifications. Sur iPhone ou sans l'application : page `https://…/livreur` du relais.

Le Play Store reste possible plus tard (compte développeur Google : 25 $ une fois).

## 3. Créer la clé de signature Android (une seule fois)

La clé prouve que chaque mise à jour vient bien de vous. **La perdre empêche toute mise à jour** des applications
déjà installées : en garder deux copies hors du dépôt (clé USB, coffre de mots de passe).

```sh
keytool -genkeypair -v -keystore youma.jks -alias youma -keyalg RSA -keysize 2048 -validity 10000
base64 -w0 youma.jks > youma.jks.b64      # sous Windows : certutil -encode youma.jks youma.jks.b64
```

Dans GitHub : dépôt → **Settings** → **Secrets and variables** → **Actions** → **New repository secret** :

| Secret | Valeur |
| --- | --- |
| `ANDROID_KEYSTORE_BASE64` | contenu de `youma.jks.b64` (une seule ligne) |
| `ANDROID_KEYSTORE_MDP` | mot de passe du fichier choisi avec `keytool` |
| `ANDROID_CLE_ALIAS` | `youma` |
| `ANDROID_CLE_MDP` | mot de passe de la clé (souvent le même) |

Relancer ensuite le workflow (Actions → « Applications mobiles » → **Run workflow**).

## 4. iPhone

Le projet iPhone est prêt (`apps/mobile/*/ios`) et GitHub vérifie qu'il compile sur macOS. Pour l'installer sur des
iPhone, Apple impose son circuit :

1. **Compte développeur Apple** (99 $ par an) au nom de la personne ou de la société.
2. Créer les deux applications dans App Store Connect (`ml.youma.client`, `ml.youma.livreur`).
3. Ajouter dans GitHub le certificat de distribution, le profil et une clé d'API App Store Connect : la construction
   signée et l'envoi vers **TestFlight** (jusqu'à 10 000 testeurs, sans passer par la revue complète) seront alors
   ajoutés au workflow. **[HYPOTHÈSE]** Pas encore écrit ni essayé, faute de compte.

En attendant, les iPhone utilisent les pages web : le client commande et suit dans Safari, le livreur garde l'écran
allumé pendant la course (iOS 16.4 ou plus récent).

## 5. Pour les développeurs

```sh
cd ui && npm run build:client        # ou build:livreur (VITE_APPLI, dossiers dist-client / dist-livreur)
cd ../apps/mobile/client && npm ci && npx cap sync android   # puis ouvrir android/ dans Android Studio
node ui/outils/icones-applis.mjs     # depuis ui/ : icônes et écrans de démarrage depuis apps/mobile/*/icone.svg
```

Les applications embarquent l'interface (rien n'est chargé depuis Internet hormis l'API du relais) ; les appels
passent par la couche HTTP native de Capacitor (`CapacitorHttp`), donc sans CORS sur le relais.
