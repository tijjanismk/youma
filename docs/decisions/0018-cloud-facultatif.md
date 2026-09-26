# 0018 — Cloud facultatif : résumés, sauvegardes chiffrées, espace propriétaire

## Contexte
Le cahier des charges (mode C, §23, V2) demande une sauvegarde distante chiffrée, la consultation à distance par le
propriétaire, un résumé de fin de journée (SMS/WhatsApp) et le multi-établissements, sans jamais rendre le cloud
obligatoire. Les propriétaires de maquis ont souvent plusieurs établissements et suivent leur activité par téléphone.

## Décision
* **Un seul serveur Internet facultatif** (`youma-relais`) : relais des commandes en ligne (fiche 0013) et cloud
  multi-restaurants. Le fournisseur inscrit chaque restaurant (`--ajouter-restaurant`) et lui remet une clé.
* **Le poste appelle le cloud** toutes les 5 minutes (et à la demande) : il envoie les résumés des 7 dernières
  journées, puis sa dernière sauvegarde si elle n'est pas encore partie. Aucun envoi n'est nécessaire pour vendre.
* **Résumés plutôt que réplication** : quelques chiffres par journée suffisent à la consultation et au SMS ; aucune
  donnée client ou employé ne quitte le restaurant hors des sauvegardes chiffrées.
* **Sauvegardes chiffrées sur le poste** (XChaCha20-Poly1305, clé Argon2id dérivée d'une phrase du restaurant). La
  phrase reste sur le poste ; le cloud stocke des octets illisibles (14 dernières). Récupération : téléchargée,
  déchiffrée, rangée avec les sauvegardes locales, puis restaurée par le chemin habituel (sauvegarde « avant »).
* **Espace propriétaire** (`/proprietaire`) : numéro + mot de passe ; le poste n'envoie que l'empreinte Argon2. Un même
  numéro et mot de passe sur plusieurs postes regroupent les restaurants (multi-établissements), avec les totaux par
  journée. Consultation seule.
* **Résumé SMS** à la clôture par Orange Mali (fiche SMS), simulé sans contrat ; une seule fois par journée.
* Licence : module « cloud » (sans effet en démonstration).

## Alternatives écartées
* Réplication complète des tables vers le cloud : plus lourde, plus de données personnelles hors du restaurant, sans
  besoin exprimé ; l'outbox la rend possible plus tard (fiche 0008).
* Modification des prix ou du menu depuis le cloud : non demandée pour l'instant ; consultation seule **[HYPOTHÈSE]**.
* WhatsApp : exige un compte WhatsApp Business et un fournisseur agréé ; SMS seulement pour commencer **[HYPOTHÈSE]**. Fiche 0028 : résumé envoyable par lien wa.me en attendant l'API.
* Phrase de chiffrement gardée par le fournisseur : il pourrait lire les données ; refusé.

## Conséquences
Paramètres `cloud` (secrets masqués dans `/etat` et le journal) ; règles RG-CLO-01 à 05 ; onglet Administration →
« Cloud » (protégé par mot de passe) ; page publique `/proprietaire`. Perdre la phrase de chiffrement rend les
sauvegardes distantes inutilisables : l'écran le dit et demande de la noter sur papier.
