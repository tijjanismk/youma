# 0048 — Photos du menu servies à part par le relais, réponses compressées

## Contexte

Question du porteur de projet (04/10/2026) : « si j'ai 50 000 utilisateurs, comment mon application réagira ? ».
Le point faible relevé : les photos des plats sont des URL `data:` dans le menu (fiche 0029), donc **chaque ouverture
du menu en ligne retélécharge toutes les photos**. Un menu de 1 Mo ouvert 50 000 fois fait 50 Go, lent en 3G et payé
au volume par l'hébergeur. Demande : « corrige les photos du menu pour les 50 000 utilisateurs ».

## Décision

1. **Relais** : à la réception d'un menu (synchronisation), chaque photo `data:image/…;base64,…` est décodée et rangée
   une seule fois dans la table `photos` (clé : empreinte SHA-256 du contenu). Le menu publié ne porte plus que
   `/api/public/photos/<empreinte>`. Les photos qui ne servent plus sont effacées.
2. **`GET /api/public/photos/{empreinte}`** : l'image avec `Cache-Control: public, max-age=31536000, immutable`.
   L'adresse change avec la photo : le téléphone garde chaque photo un an sans la redemander.
3. **Compression gzip** de toutes les réponses du relais (`tower-http` `CompressionLayer`) : menu, suivi, courses.
4. **Relais déjà en service** : un menu reçu avant cette version est converti à l'ouverture du relais (redémarrage
   après mise à jour), sans attendre que le restaurant change son menu.
5. **Interface** : `srcPhoto` (`composants/Plat.tsx`) préfixe l'adresse par celle du relais dans les applications ;
   images en `loading="lazy"` et `decoding="async"`.

Rien ne change sur le poste central : photos toujours en URL `data:` dans la base, compressées par le navigateur
(fiche 0029), sur le Wi-Fi du restaurant.

## Alternatives écartées

- **Stockage objet / CDN** (S3, Cloudflare R2) : un service et un compte de plus ; le cache d'un an du téléphone
  supprime déjà l'essentiel du trafic. À reprendre si un restaurant a des centaines de milliers de visiteurs.
- **Photos en fichiers sur le disque du relais** : la base SQLite du volume Railway est déjà sauvegardée avec le reste ;
  quelques centaines de photos de 30 à 60 ko n'en font pas un problème.
- **Changer le stockage du poste** (fichiers au lieu de `data:`) : migration et sauvegardes à reprendre, pour un gain
  nul sur le Wi-Fi local.

## Conséquences

- Première visite : le menu (texte, compressé) puis les photos visibles, une par une ; visites suivantes : le menu seul,
  quelques ko.
- Une photo changée sur le poste prend une nouvelle adresse : jamais d'ancienne photo affichée par le cache.
- Le relais garde un exemplaire de chaque photo (base un peu plus grosse, mais une seule copie par photo).
