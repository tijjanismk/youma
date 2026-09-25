# 0022 — Design system « Mali vivant » : jetons, thème clair et sombre

## Contexte
Le design system Youma (artifact « Mali vivant », synchronisé depuis `main@fa5f273`) propose une nouvelle palette :
encre indigo (teinture de Ségou), mangue pour la marque, une couleur du pays par état (baobab, bissap, mil, indigo),
et un thème sombre pour la cuisine et le service du soir. Le code gardait l'ancienne palette crème et orange, avec
une cinquantaine de couleurs écrites en dur, illisibles dans un thème sombre.

## Décision
* **Jetons** repris de `tokens.json` dans `ui/src/styles.css` : thème clair dans `:root`, sombre sous
  `[data-theme="sombre"]`. Nouveaux jetons : `carte-2`, `bord-fort`, `mangue`, `sur-accent`, `menu-fond`,
  `menu-texte`, les teintes `-doux` des états, `sur-etat`, `focus`, `ombre-accent`, `rayon-tuile`, `rayon-pilule`.
  `vert-clair` (jamais utilisé) est retiré.
* **Plus de couleur en dur** : corps de `bundle.css` repris tel quel (même feuille, couleurs remplacées par les jetons),
  puis la couche « Mali vivant » en fin de fichier. Seules exceptions : le jaune du bandeau « démonstration », les
  ombres et voiles en `rgba`, la bordure grise de l'impression.
* Écart avec `bundle.css` : le texte des notifications (sur `menu-fond`) prend `menu-texte` et non `sur-etat`, qui
  vaut l'indigo nuit en sombre et serait illisible sur le fond du menu.
* **Choix du thème** : bouton lune / soleil dans l'en-tête (`aria-label` « Thème sombre », `aria-pressed`), mémorisé
  par poste (`localStorage` `youma.theme`, `src/theme.ts`), posé sur `<html data-theme>` avant le premier rendu ;
  la balise `theme-color` suit le fond du thème. Clair par défaut.
* **Impression** : une page imprimée reprend toujours les jetons clairs.

## Alternatives écartées
* Suivre `prefers-color-scheme` du téléphone : le choix dépend du poste (cuisine sombre, caisse au soleil), pas du
  réglage personnel de l'appareil ; clair par défaut, choix explicite.
* Réglage du thème dans l'administration (pour tout le restaurant) : chaque poste a sa lumière.

## Conséquences
Les manifestes gardent un seul `theme_color` (clair, `#f5f6fa`). Toute nouvelle règle CSS prend les jetons ; une
couleur posée sur un plein d'état utilise `sur-etat`, sur le menu `menu-texte`, sur l'accent `sur-accent`.
Captures : `docs/captures/mali-vivant/`. Restent hors de ce lot : les émojis de certains boutons (le design system
les écarte) et la graisse 800 de Poppins (`montant-fort`, non importée : affichée en 700).
