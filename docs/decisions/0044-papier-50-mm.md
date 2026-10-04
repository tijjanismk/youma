# 0044 — Tickets sur papier de 50 mm

## Contexte

Demande du porteur de projet (04/10/2026) : « ajoute impression à 50 mm ». Certaines imprimantes thermiques bon marché
(portatives, Bluetooth, petites caisses) utilisent des rouleaux de 50 mm. La largeur des tickets se réglait en caractères
(32 en 58 mm, 42 ou 48 en 80 mm). En la baissant, les titres en gros caractères (`##`, double largeur) dépassaient :
« BON DE SORTIE n°12 » occupe 36 caractères en double largeur. C'était déjà le cas en 58 mm, et l'imprimante
coupait le texte au hasard.

## Décision

- Nouvelle largeur **50 mm = 28 caractères** (Administration → Restaurant et règles → Largeur du papier).
  **[HYPOTHÈSE]** Environ 40 mm imprimables et la police A (12 points par caractère) : à vérifier sur l'imprimante
  réelle ; « Autre » reste possible en saisissant le nombre de caractères voulu.
- Titres en gros caractères (`impression::double`), lignes centrées (`centre`) et en gras (`gras`) **coupés aux
  mots** à la largeur du papier, quelle que soit cette largeur. Cela vaut pour les tickets client, la cuisine et le
  rapport Z.
- Ticket imprimé par le navigateur : largeur utile selon le papier (40, 48 ou 72 mm, `largeurImprimableMm`).

## Alternatives écartées

- Réduire la police à l'impression ESC/POS (police B) : moins lisible en cuisine, et pas gérée par toutes les
  imprimantes compatibles.
- Abandonner les gros caractères en dessous de 58 mm : le titre et les plats en gros restent utiles en cuisine.

## Conséquences

- Test `ticket_sur_papier_de_50_mm` : aucune ligne des tickets client et cuisine ne dépasse 28 caractères.
- Un nom de restaurant long s'imprime sur deux lignes en gros caractères.
