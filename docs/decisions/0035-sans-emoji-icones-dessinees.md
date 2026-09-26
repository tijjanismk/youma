# 0035 — Plus d'émojis ni de symboles décoratifs : icônes dessinées

## Contexte
Le porteur de projet (26/09/2026) : « enlève aussi ✓ et les émojis comme ça qui n'ont pas d'utilité, dans toute
l'app ». Il restait des coches (« Prêt ✓ », « Reçu ✓ », droits des rôles, INPS/AMO, paie soldée), des symboles
de boutons (✕ fermer, ✎ modifier, ← retour) et les **émojis des catégories** (🥤 🍺 🍗…), tapés à la main dans
l'Administration et affichés dans les onglets, sur les plats sans photo et au menu client.

## Décision
* Coches décoratives retirées (« Prêt », « Reçu ») ; celles qui portent une information deviennent des mots
  (« INPS : oui / non », « (payé) ») ou l'icône `Check` (tableau des droits).
* Boutons : icônes lucide `X`, `Pencil`, `ArrowLeft`, avec leur libellé ou leur `aria-label`.
* **Catégories** : l'icône se **choisit dans une liste** (`composants/IconeCategorie.tsx`, 19 icônes : boissons,
  bières, grillades, plats, desserts…) ; la base garde une clé (`boissons`, `bieres`…). Les anciens émojis des
  bases existantes sont reconnus et affichés avec l'icône correspondante, sans migration ; une valeur inconnue
  donne les couverts. La démonstration utilise les clés.
* Restent : les flèches « → » des chemins écrits dans les aides (« Administration → Produits ») et le signe « × »
  des quantités, qui ont un sens.

## Alternatives écartées
* **Migration SQL des émojis vers les clés** : inutile tant que l'affichage reconnaît les deux ; le relais et les
  anciennes sauvegardes restent lisibles.
* **Supprimer toute icône de catégorie** : l'icône aide à repérer l'onglet au premier coup d'œil.

## Conséquences
Rendu identique sur tous les appareils (les émojis changeaient d'un téléphone à l'autre) et cohérent avec le design
system (fiche 0022). La version à imprimer de la formation (`docs/guides/formation/`) est produite avec ces écrans.
