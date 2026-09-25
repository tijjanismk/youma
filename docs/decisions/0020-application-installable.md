# 0020 — Application installable (PWA) et HTTPS du réseau local

## Contexte
Le propriétaire veut Youma comme une application sur les téléphones : icône sur l'écran d'accueil, plein
écran, ouverture rapide même quand le Wi-Fi est faible. Les navigateurs (Chrome Android, Safari iOS)
n'installent une application web et n'activent un *service worker* qu'en **HTTPS** (ou sur `localhost`).
Or les téléphones joignent le poste central en HTTP sur le Wi-Fi du restaurant (`http://192.168.x.x:7878`).
**Contradiction signalée** : « fonctionne sans Internet » exclut un certificat public (Let's Encrypt
demande un nom de domaine et un renouvellement en ligne tous les 90 jours).

## Décision
* **Manifeste** (`manifest.webmanifest`) : nom, icônes 192/512 + masquable, plein écran, raccourcis
  (Salle, Caisse, Cuisine). L'espace propriétaire a son propre manifeste (`/proprietaire`, « Youma Patron »).
* **Service worker produit à la compilation** (plugin dans `vite.config.ts`) : liste exacte des fichiers
  de la version, version = empreinte du contenu. Coquille en cache (ouverture instantanée, même sans
  réseau) ; **`/api/` ne passe jamais par le cache** : les données viennent toujours du poste central,
  aucune vente n'est enregistrée ailleurs que dans la base (pas de file d'attente hors ligne).
* **Nouvelle version** : un bandeau « Mettre à jour » ; l'activation attend le geste de l'utilisateur
  (pas de rechargement au milieu d'une commande).
* **HTTPS local** : le poste central crée une fois une autorité de certification propre au restaurant
  (clé ECDSA P-256, 10 ans), **conservée dans la base** (table `systeme`) : elle suit les sauvegardes et
  les restaurations, les téléphones n'ont rien à réinstaller après un changement de PC. À chaque démarrage,
  un certificat est fait pour les adresses du moment (127.0.0.1 + IP du réseau local, 1 an).
* **Autorité bridée** (contraintes de noms) : elle ne peut certifier que `localhost` et les adresses
  privées (127/8, 10/8, 172.16/12, 192.168/16, 100.64/10). Même volée avec une sauvegarde, sa clé ne permet
  pas d'usurper un site Internet sur les téléphones qui l'ont installée. Port HTTPS = port HTTP + 1 (7879), en plus de l'HTTP
  (qui reste pour le poste central lui-même et les anciens appareils). Chaque téléphone installe une fois
  le certificat de l'autorité (`/api/reseau/certificat`, public : il ne contient pas la clé).
* Cryptographie : `rustls` + `ring` (déjà présents via `reqwest`), `rcgen` pour les certificats ; aucune
  bibliothèque système (OpenSSL) ni compilateur C supplémentaire sous Windows.
* Au démarrage sans réseau, la session enregistrée n'est plus effacée (seul un refus du poste central
  la supprime) et l'application réessaie de joindre le poste central toutes les 5 s.

## Alternatives écartées
* **Certificat public** via un domaine du fournisseur pointant vers l'IP locale (type `plex.direct`) :
  dépend d'Internet pour le renouvellement et d'un service du fournisseur.
* **Certificat auto-signé sans autorité** : refusé par les navigateurs pour l'installation, et à refaire
  à chaque changement d'IP.
* **Application native (Android)** : distribution, mises à jour et coût de développement ; la PWA reste
  à jour avec le poste central.
* **Ventes hors ligne dans le téléphone** : contraire à « une seule base, un seul écrivain » (fiche 0003).

## Conséquences
* Une manipulation par téléphone (installer le certificat), guidée dans Administration → Appareils.
  **[HYPOTHÈSE]** acceptable pour 2 à 10 téléphones par restaurant ; à valider sur le terrain.
* Une base neuve (sans restauration) crée une nouvelle autorité : chaque téléphone réinstalle alors le
  certificat. Si le PC n'a pas d'adresse privée (cas rare), l'HTTPS n'est pas proposé (message au journal).
* Si l'IP du PC change (box qui redémarre), un redémarrage de Youma refait le certificat ; l'appairage
  (jeton par adresse) est à refaire si l'adresse change. **[HYPOTHÈSE]** recommander une IP fixe (bail DHCP
  réservé) dans le guide d'installation.
* Le relais Internet, déjà derrière un proxy HTTPS, sert la même application : l'espace propriétaire et
  le menu en ligne s'installent sans autre manipulation.
