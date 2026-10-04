# 0049 — Relais partagé par plusieurs restaurants (adresses `/r/<nom>`)

## Contexte

Le relais Internet (fiche 0013) ne servait les commandes en ligne que d'**un** restaurant : une clé, un menu, une
base. Pour un deuxième restaurant, il fallait un second service Railway avec son volume et son adresse (fiche 0030),
alors que le cloud (fiche 0018) était déjà multi-restaurants. Le porteur de projet demande (04/10/2026) : « fais le
relais partagé pour plusieurs restaurants », base du service à plusieurs clients (SaaS) noté dans `REPRISE.md`.

## Décision

1. **Inscription unique** : `youma-relais --ajouter-restaurant "NOM"` donne une clé (déjà valable pour le cloud) et
   un **nom court** tiré du nom (`maquis-le-baobab`, unique : `-2`, `-3`…), gardé dans `cloud_restaurants.slug`. Les
   restaurants déjà inscrits au cloud reçoivent le leur à l'ouverture du relais.
2. **Adresses** : `https://relais/r/<nom>/…` — menu, suivi, livreurs, API (`/r/<nom>/api/public/menu`) et
   synchronisation du poste (`/r/<nom>/api/relais/synchroniser`). Un aiguillage placé avant le routage enlève
   `/r/<nom>` et passe le restaurant aux routes par un en-tête interne (celui envoyé par un client est effacé) :
   les routes et l'interface restent les mêmes.
3. **Isolation** : une base SQLite par restaurant (`restaurants/<nom>.db`, même schéma que le relais), ouverte à la
   première demande ; sa clé seule synchronise ce restaurant. Un code de suivi, un livreur, une photo d'un
   restaurant ne sont jamais visibles chez un autre. Nom inconnu : 404.
4. **Compatibilité** : le restaurant relié par `YOUMA_RELAIS_CLE` garde les adresses à la racine (`/menu`) et sa base
   `youma-relais.db` ; rien ne change pour les postes, liens et applications déjà en service.
5. **Interface** : sur le web, `prefixeWeb()` lit `/r/<nom>` dans l'adresse de la page ; `baseApi()`, les liens
   (`cheminPublic`) et le choix de la page (`cheminCourant`) en tiennent compte. Dans les applications, l'adresse du
   relais comprend `/r/<nom>` (`adresseRelais`, `lireLien`). Le poste n'a rien à changer : « Adresse du relais » =
   `https://relais/r/<nom>`.

## Alternatives écartées

- **Un sous-domaine par restaurant** (`baobab.relais.ml`) : DNS générique et certificat générique à gérer ; Railway
  et DuckDNS le compliquent. Les adresses `/r/<nom>` marchent avec un seul domaine.
- **Une seule base avec une colonne `restaurant` partout** : réécriture de toutes les requêtes et des clés primaires
  (risque d'oubli = fuite entre restaurants). Une base par restaurant isole par construction et garde le code.
- **PostgreSQL** (fiche 0003) : pas nécessaire à cette échelle ; à revoir s'il faut plusieurs relais en parallèle.
- **Un service Railway par restaurant** : coût et volume par restaurant ; reste possible pour un restaurant très
  chargé.

## Conséquences

- Un seul service et un seul volume pour tous les restaurants ; sauvegarder `/donnees` sauvegarde tout.
- Les compteurs anti-abus (adresse, numéro) sont communs aux restaurants du relais.
- Le nom court apparaît dans les liens envoyés aux clients : le choisir lisible à l'inscription (nom du restaurant).
- Retirer un restaurant : effacer sa ligne de `cloud_restaurants` (ses adresses répondent alors 404) ; sa base reste
  sur le volume jusqu'à ce qu'on l'efface.
