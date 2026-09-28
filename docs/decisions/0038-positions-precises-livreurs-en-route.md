# 0038 — Position du client plus précise, livreurs en route vus du restaurant

## Contexte

Essai d'une commande à distance : la position partagée par le client était approximative. Le navigateur la
demandait sans `enableHighAccuracy` : le téléphone répondait avec la position du réseau (antennes, Wi-Fi), à
plusieurs centaines de mètres près, parfois en cache. Par ailleurs, seul le client voyait la position du livreur ;
le restaurant non.

## Décision

- Menu client : `watchPosition` avec `enableHighAccuracy: true`, `maximumAge: 0`. On garde la meilleure mesure
  pendant 30 s au plus et on s'arrête sous 25 m. La précision est affichée (« à 12 m près ») avec un lien
  « Vérifier sur la carte » ; au-delà de 100 m, avertissement : activer la localisation précise, le point de
  repère reste indispensable.
- Écran Livraisons : carte « Livreurs en route » (dernière position de chaque course assignée ou en route, nom du
  livreur, « il y a n min », distance au client si sa position est connue, lien carte).
  `livraison::positions_en_cours`, `GET /api/livraisons/positions`.
- La page du livreur demandait déjà la haute précision : inchangée.

## Alternatives écartées

- Carte intégrée à l'écran (tuiles OpenStreetMap chargées dans l'application) : contraire à la règle « rien n'est
  chargé depuis Internet par l'interface ». Le lien ouvre la carte dans un autre onglet.
- Refuser les positions imprécises : un client sans GPS ne pourrait plus rien partager ; le point de repère reste
  obligatoire de toute façon.

## Conséquences

- Sur un PC (pas de GPS), la position reste approximative : c'est le téléphone qui donne une position précise.
- Il faut toujours le relais en `https` pour que le téléphone accepte de donner sa position.
