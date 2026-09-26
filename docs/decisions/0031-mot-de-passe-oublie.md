# 0031 — Mot de passe d'administration oublié

## Contexte
Le porteur de projet (26/09/2026) demande une réinitialisation du mot de passe d'administration. Le propriétaire
peut déjà redéfinir celui d'un gérant (Administration → Utilisateurs), mais si le **propriétaire** oublie le sien,
plus personne n'ouvre l'administration (utilisateurs, paramètres, sauvegardes, licence). Le poste central
fonctionne sans Internet et chez des restaurants sans service informatique : pas d'e-mail de réinitialisation.

## Décision
Règle **RG-AUT-07**. « Mot de passe oublié ? » dans la fenêtre du mot de passe d'administration. Le propriétaire,
**déjà connecté avec son PIN**, choisit un nouveau mot de passe en apportant une seconde preuve :

* **Code de secours** (`youma-core/src/secours.rs`) : 16 caractères (`XXXX-XXXX-XXXX-XXXX`, alphabet sans 0/O ni
  1/I/L, environ 79 bits), affiché **une seule fois** à la fin de l'installation, à recopier sur papier. Seule son
  empreinte Argon2 est gardée (table `systeme`, incluse dans les sauvegardes). Renouvelable dans Administration →
  Utilisateurs (mot de passe exigé). Casse, espaces et tirets ignorés à la saisie.
* **Réponse du fournisseur**, si le code est perdu ou pour les installations antérieures : le poste affiche un code
  de demande (`XXXX-XXXX-XXXX`, le même tant qu'il n'a pas servi), le fournisseur le signe avec la **clé privée des
  licences** (`youma-licence secours --cle-privee … --demande …`, Ed25519) et renvoie la réponse (86 caractères,
  par WhatsApp ou SMS, à coller). Le poste la vérifie avec la clé publique déjà intégrée pour les licences.

Après réussite : nouveau mot de passe, autres sessions de l'utilisateur fermées, session courante confirmée,
code de demande effacé, **nouveau code de secours** affiché (l'ancien ne sert plus), trace dans le journal
(`utilisateur.mot_de_passe_reinitialise`, avec le moyen). Les échecs comptent avec ceux du PIN (5 échecs →
verrouillage de 5 minutes, RG-AUT-02). Réservé au rôle propriétaire.

## Alternatives écartées
* **Réinitialisation par e-mail ou SMS** : exige Internet et un compte en ligne ; contraire au local-first.
* **PIN du propriétaire seul** : 4 à 6 chiffres, souvent vus par le personnel ; le mot de passe existe justement
  pour ne pas s'y limiter (RG-AUT-06).
* **Commande en ligne de commande sur la caisse** : inutilisable pour un propriétaire sur une caisse POS Windows
  avec l'application Youma ; et quiconque touche la caisse pourrait s'en servir.
* **Mot de passe maître du fournisseur** : un secret commun à tous les restaurants ; la signature d'une demande à
  usage unique ne donne accès qu'à un poste, une fois.

## Conséquences
Le code de secours doit être recopié et rangé : le guide d'installation le rappelle. Les installations faites avant
cette version n'ont pas de code : passer par le fournisseur, ou en créer un dans Administration → Utilisateurs.
Le fournisseur doit vérifier l'identité de la personne qui demande (voix, numéro connu du propriétaire) avant de
signer. Une clé de développement signe aussi les réponses des postes compilés sans `YOUMA_CLE_PUBLIQUE` : jamais
de tels postes chez un client (déjà la règle pour les licences).
