# Module : interface (`ui/src`)

Rôle : écrans du personnel et pages publiques ; aucune logique métier ni accès base.

## Fichiers clés
- `api.ts` — `appel` : `fetch('/api' + chemin)` relatif (l. 105), jetons en `localStorage` (l. 50–76), `telecharger` (export CSV, l. 140), `surReseau`.
- `contexte.tsx` — `useApp().agir` (PIN / mot de passe redemandés et rejoués), WebSocket bâti sur `location.host` (l. 101–102), reconnexion.
- `App.tsx` — routes, `Appairage` (code lu dans l'URL `?appairage=`, l. 108–116), pages publiques sans connexion.
- `pwa.ts` — service worker seulement si `isSecureContext` (l. 29).
- `types.ts`, `paiement.ts` (validation des parts), `panier.ts`, `format.ts`, `i18n.ts`, `whatsapp.ts`, `listesStock.ts`, `theme.ts`.
- `pages/` — PriseCommande (713 l.), Administration (1335 l., onglet Fidélité), Caisse, Encaissement (carte cadeau, société, points), Clients (+ `CartesSocietes.tsx`), Entrantes, Cuisine, Livraisons, Stock, Achats, Paie…
- `public/` — `MenuClient.tsx` (position GPS haute précision, case « Partager ma position » seulement en contexte sécurisé, bandeau « déjà envoyée »), `position.ts` (`positionPossible`, `messageErreurPosition`), `Suivi`, `Livreur.tsx` (Wake Lock pendant la course), propriétaire.
- `composants/` — Base, Chiffres, Ticket, Plat, IconeCategorie, MenuDuJour, Secours, Installation.

## Entrant
`api.ts` importé par 37, `types.ts` 35, `format.ts` 34, `contexte.tsx` 32.

## Règles d'interface (CLAUDE.md)
Jetons CSS « Mali vivant », pas de couleur en dur, pas d'émoji, rien depuis Internet, `aria-label` stables pour `ui/e2e/`.
Prettier `--print-width 160` seulement sur les fichiers déjà propres.

## Code mort probable (carte, SÛR — à confirmer)
`iconeCategorie`, `compresserImage`, `teinte`, `definirLangue`, `estInstallee`, `QUARTIERS_PAR_VILLE` (exports jamais importés).
