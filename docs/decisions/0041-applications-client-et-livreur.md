# 0041 — Applications Youma Client et Youma Livreur (Capacitor), pages web gardées

## Contexte

Fiche 0040 : le personnel reste sur le navigateur ; le suivi du livreur, qui sert à rassurer le client, se coupe
quand l'écran se verrouille. Le porteur de projet demande (03/10/2026) **deux applications**, une pour les
livreurs et une pour les clients des commandes à distance, en **gardant** les pages web, puis demande les iPhone.
Les livreurs sont surtout occasionnels mais connus du restaurant ; les clients commandent en ligne via le relais.

## Décision

- **Capacitor 8** (option 2 de l'étude, `AI_CONTEXT/ETUDE-MOBILE.md`) : la même interface React, compilée avec
  `VITE_APPLI=client|livreur` (`npm run build:client|build:livreur`), embarquée dans deux projets
  `apps/mobile/client` (`ml.youma.client`) et `apps/mobile/livreur` (`ml.youma.livreur`), Android et iOS.
- **Relais seulement** : les applications appellent le relais du restaurant (`https`), jamais le poste central ;
  aucun changement sur le poste ni sur le relais. Adresse gardée par application (`ui/src/appli.ts`, `baseApi()`
  ajoutée devant `/api` dans `api.ts` ; vide sur le web). Appels par `CapacitorHttp` (natif) : pas de CORS.
- **Youma Client** : liste de restaurants (ajout par lien collé ou `youma-client://`), menu, suivi ; codes de
  suivi rattachés à leur restaurant ; panier par restaurant ; notification locale quand l'étape change
  (`@capacitor/local-notifications`, aucun serveur de notifications).
- **Youma Livreur** : liste des courses (lien collé, ou bouton « ouvrir dans l'application » de la page web,
  `youma-livreur://course?relais=…&code=…`) ; position par `@capacitor-community/background-geolocation`
  (service Android de premier plan avec notification « Course en cours » ; iOS `UIBackgroundModes location`),
  envoyée toutes les 15 s au plus, arrêtée par « Arrêter » ou quand le relais répond que la course est finie.
- **Pages web gardées** : elles proposent seulement un lien « ouvrir dans l'application » (en `https`).
- **Construction par la CI** (`.github/workflows/applis-mobiles.yml`) : APK signé si les secrets de la clé
  existent, sinon APK d'essai ; versions `v*` jointes aux Releases ; compilation iPhone pour simulateur sur macOS
  (à la demande, PR des applications, étiquettes). Pas d'outillage Android dans l'environnement de développement :
  la CI fait foi.
- **iPhone** : projet prêt ; diffusion par TestFlight / App Store quand un compte Apple (99 $/an) existera.

## Alternatives écartées

- **Réécriture native (React Native, Flutter)** : plusieurs mois et deux interfaces à maintenir pour le même
  résultat (fiche 0040, étude).
- **Une application par restaurant** : une construction et une diffusion par restaurant ; contraire au relais
  partagé visé (SaaS). Une application, plusieurs restaurants.
- **Une seule application client + livreur** : le livreur n'a pas à voir le menu, et les autorisations de position
  en arrière-plan inquiéteraient les clients.
- **Liens vérifiés Android (App Links)** : il faudrait publier l'empreinte de la clé sur chaque relais ; le schéma
  `youma-…://` suffit et marche aussi sur iPhone.
- **Notifications poussées (Firebase, APNs)** : service extérieur et serveur à gérer ; la notification locale
  suffit tant que l'application tourne. À revoir si les clients la demandent écran éteint.

## Conséquences

- Une mise à jour de l'interface des applications passe par un nouvel APK (numéro `versionCode` = numéro de
  l'exécution de la CI). Les pages publiques et l'API du relais doivent rester compatibles avec les applications
  déjà installées.
- La clé de signature Android est un secret à ne jamais perdre (`docs/guides/applications-mobiles.md`).
- **[HYPOTHÈSE]** Essais à faire sur de vrais téléphones Android (position écran verrouillé, économie d'énergie
  des constructeurs qui tuent les services) et, plus tard, sur iPhone.
- `npm audit` signale une faille modérée dans `uuid`, dépendance de l'outil `@capacitor/cli` (génération du projet
  Xcode) : non embarquée dans les applications.
