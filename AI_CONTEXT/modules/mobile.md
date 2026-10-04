# Module : applications mobiles (Youma Client, Youma Livreur)

Rôle : coquilles Capacitor 8 (Android + iOS) autour de l'interface compilée avec `VITE_APPLI` ; parlent au relais seulement (fiche 0041).

## Fichiers
- `ui/src/appli.ts` — `APPLI`, `baseApi()` (préfixe de `/api` dans `api.ts`), restaurants, courses, codes de suivi par relais, `adresseRelais`, `lireLien` (`youma-client://`, `youma-livreur://`, liens web du relais).
- `ui/src/appli/Appli.tsx` — accueil client (Mes restaurants) / livreur (Mes courses), liens d'ouverture et bouton Retour (`@capacitor/app`), puis pages publiques habituelles.
- `ui/src/appli/positionArrierePlan.ts` — `@capacitor-community/background-geolocation` (service de premier plan, notification « Course en cours »).
- `ui/src/appli/notifier.ts` — notification locale quand l'étape du suivi change (client).
- `ui/src/public/Livreur.tsx`, `Suivi.tsx`, `MenuClient.tsx` (`Page` : lien retour), `panierClient.ts` (`codesSuivis`, panier par relais), `pwa.ts` (pas de service worker en application).
- `apps/mobile/{client,livreur}/` — `capacitor.config.json` (CapacitorHttp ; livreur `useLegacyBridge`), `android/` (schéma d'ouverture, permissions, signature par variables d'environnement, `versionCode` = numéro CI), `ios/` (Info.plist : position, schéma, `UIBackgroundModes location` pour le livreur), `icone.svg`.
- `ui/outils/icones-applis.mjs` — icônes et écrans de démarrage ; `.github/workflows/applis-mobiles.yml` — APK (signés si secrets `ANDROID_*`) et compilation iOS simulateur.

## Règles
- [CONFIRMÉ] Sur le web `APPLI` est nul : `baseApi()` renvoie "" et rien ne change — `ui/src/appli.ts`.
- [CONFIRMÉ] Adresse du relais toujours en https — `adresseRelais` (test `logique.test.ts`).
- [DÉDUIT] Pas d'outils Android dans l'environnement de développement (dl.google.com bloqué) : la CI est le seul banc de construction.
- Youma Livreur : connexion téléphone + PIN donné par le restaurant, courses assignées listées (`ui/src/public/MesCourses.tsx`, aussi page `/livreur` du relais, fiche 0047).
