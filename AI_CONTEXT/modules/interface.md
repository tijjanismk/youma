# Module : interface (`ui/src`)

Rôle : écrans du personnel et pages publiques ; aucune logique métier ni accès base.

## Fichiers clés
- `api.ts` — `appel` : `fetch('/api' + chemin)` relatif (l. 105), jetons en `localStorage` (l. 50–76), `telecharger` (export CSV, l. 140), `surReseau`.
- `contexte.tsx` — `useApp().agir` (PIN / mot de passe redemandés et rejoués), WebSocket bâti sur `location.host` (l. 101–102), reconnexion.
- `App.tsx` — routes, `Appairage` (code lu dans l'URL `?appairage=`), pages publiques sans connexion (`cheminCourant()` enlève `/r/<nom>`, fiche 0049), bandeau `JourneeEnRetard` (journée oubliée ouverte, `format.ts::joursJourneeOuverte`), bouton « Personnaliser le menu ».
- `menuPerso.ts` + `composants/PersonnaliserMenu.tsx` — écrans affichés et ordre, par utilisateur sur l'appareil (fiche 0051) ; appliqué par `pages/Accueil.tsx::menuVisible(permissions, etat, utilisateur)`.
- `pwa.ts` — service worker seulement si `isSecureContext` (l. 29).
- `types.ts`, `paiement.ts` (validation des parts), `panier.ts`, `format.ts`, `i18n.ts`, `whatsapp.ts`, `listesStock.ts`, `theme.ts`.
- `pages/` — PriseCommande (713 l.), Administration (1335 l., onglet Fidélité), Caisse, Encaissement (carte cadeau, société, points), Clients (+ `CartesSocietes.tsx`), Entrantes, Cuisine, Livraisons, Stock, Achats, Paie…
- `public/` — `MenuClient.tsx` (position partagée avec le restaurant et le livreur, numéro vérifié une fois par SMS ou WhatsApp : `jeton_client`, `panierClient.ts::lireClientVerifie`, fiche 0046), `position.ts`, `Suivi`, `Livreur.tsx` (Wake Lock), `MesCourses.tsx` + `EspaceLivreur.tsx` (connexion livreur téléphone + PIN, page `/livreur`, fiche 0047), propriétaire. Liens via `appli.ts::cheminPublic` (relais partagé).
- `pages/` — `PriseCommande` (mode « Ruptures »), `Paie` (« Payer », « Tout payer »), `FicheEmploye` (accès livreur), `Caisse` (remise au coffre à l'ouverture), `Entrantes` (position du client).
- `composants/` — Base, Chiffres, Ticket, Plat (`srcPhoto` : photos servies par le relais, fiche 0048), PositionClient, PersonnaliserMenu, IconeCategorie, MenuDuJour, Secours, Installation.

## Entrant
`api.ts` importé par 37, `types.ts` 35, `format.ts` 34, `contexte.tsx` 32.

## Règles d'interface (CLAUDE.md)
Jetons CSS « Mali vivant », pas de couleur en dur, pas d'émoji, rien depuis Internet, `aria-label` stables pour `ui/e2e/`.
Prettier `--print-width 160` seulement sur les fichiers déjà propres.

## Code mort probable (carte, SÛR — à confirmer)
`iconeCategorie`, `definirLangue`, `estInstallee`, `QUARTIERS_PAR_VILLE` (exports jamais importés, carte du 21/10).
