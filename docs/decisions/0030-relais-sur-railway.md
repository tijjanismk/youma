# 0030 — Relais hébergé sur Railway

## Contexte
Le porteur de projet (26/09/2026) demande à héberger le relais (fiches 0013 et 0018) sur Vercel, puis, Vercel ne
convenant pas, sur Railway. Le relais est un processus permanent qui garde sur disque (SQLite) les commandes en
attente, les menus et les sauvegardes chiffrées ; le guide prévoyait jusqu'ici un VPS avec Caddy.

## Décision
* **Image Docker** `deploiement/railway/Dockerfile` (contexte : racine du dépôt) en trois étapes : pages
  (`ui/dist`, Node 22), relais (`cargo build --release --locked -p youma-relais`), image finale Debian slim
  d'environ 130 Mo. Aucun paquet système : les certificats racines sont intégrés au relais (webpki-roots).
* **`railway.toml`** à la racine : construction par ce Dockerfile, contrôle de santé `/api/etat`, redémarrage
  automatique, reconstruction seulement si `crates/`, `ui/`, `Cargo.*` ou les fichiers Railway changent.
* **Démarrage** : `--port $PORT` (fourni par Railway), `--donnees /donnees` (volume Railway à créer à ce chemin),
  `--derriere-proxy` (Railway termine le HTTPS et transmet l'adresse du client dans `X-Forwarded-For`).
* **Inscription d'un restaurant au cloud** : `railway ssh -- youma-relais --ajouter-restaurant …`.
* Le guide `docs/guides/relais-en-ligne.md` présente Railway en premier ; le VPS reste documenté.

## Alternatives écartées
* **Vercel, Netlify** : fonctions arrêtées entre deux appels, sans disque ; le relais doit tourner en permanence.
* **Réécrire le stockage du relais pour une base externe** (Postgres de Railway) : inutile, un volume suffit, et
  le même binaire sert sur un VPS.
* **Instruction `VOLUME` dans le Dockerfile** : refusée par Railway ; le volume se crée dans son interface.

## Conséquences
Environ 5 $ par mois (offre Hobby) au lieu de 4 à 6 € pour un VPS, sans serveur à administrer. Un service avec
volume redémarre à chaque déploiement : quelques secondes d'indisponibilité du menu en ligne, sans perte (volume,
et le poste renvoie son menu). Une instance par restaurant pour les commandes en ligne, comme sur VPS (fiche 0013).
**[HYPOTHÈSE]** Chemins de l'interface Railway et tarif relevés en septembre 2026 : à vérifier au premier déploiement.
