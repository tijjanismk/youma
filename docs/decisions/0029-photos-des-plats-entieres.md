# 0029 — Photos des plats entières, sans recadrage

## Contexte
Le porteur de projet (26/09/2026) signale que les photos importées pour les produits sont tronquées et demande
qu'elles soient « flexibles ». L'import recadrait toute image en carré de 320 px au centre (une photo en largeur
perdait ses côtés, une photo en hauteur le haut et le bas), puis l'affichage la rognait encore en cercle.

## Décision
* **Import sans recadrage** (`compresserImage`, `ui/src/composants/Plat.tsx`) : l'image garde ses proportions, réduite
  à 480 px sur son plus grand côté (jamais agrandie), en JPEG qualité 0,8 ; fond blanc sous les PNG transparents
  (le JPEG n'a pas de transparence). Reste une URL `data:` dans la base, sans serveur d'images.
* **Affichage entier** : `.visuel-plat.photo` en `object-fit: contain` dans un cadre arrondi à fond `--carte-2` ; la
  photo tient dans le cadre quelle que soit sa forme (bandes neutres autour si besoin). Les plats sans photo gardent
  l'icône de catégorie sur un cercle teinté.
* **Image illisible** (format non reconnu par le navigateur) : message clair au lieu d'un échec silencieux.

## Alternatives écartées
* **Outil de recadrage à l'import** : un clic de plus et une interface lourde sur téléphone ; à reprendre si les
  restaurants le demandent.
* **`object-fit: cover`** (remplir le cadre) : c'est lui qui coupe l'image.

## Conséquences
Les photos déjà enregistrées (carrées) s'affichent comme avant. Une photo de 480 px pèse environ 30 à 60 Ko dans la
base et dans le menu envoyé au relais, contre 15 à 25 Ko auparavant.
