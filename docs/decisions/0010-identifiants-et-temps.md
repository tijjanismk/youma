# 0010 — Identifiants, horodatage et journée d'exploitation

## Décision
* Identifiants UUIDv7 (texte) : uniques sans coordination, triables, prêts pour la synchronisation.
* Horodatages en millisecondes UTC (`INTEGER`) ; le Mali est à UTC+0 (décalage configurable).
* Numéros lisibles (commande, reçu, achat, bulletin) : table `sequences` incrémentée dans la transaction,
  donc sans trou (RG-SYS-04).
* Garde d'horloge (RG-SYS-01) : écriture refusée si l'heure du PC est antérieure de plus de 5 minutes au
  dernier événement ; un responsable peut accepter la nouvelle heure (journalisé).
* Journée d'exploitation ouverte explicitement ; sa date tient compte de l'heure de bascule (6 h par
  défaut). Toutes les ventes vont à la journée ouverte, quelle que soit l'heure.

## Conséquences
Une vente à 1 h 30 appartient à la journée de la veille (scénario 4) ; un PC revenu en 2000 ne peut plus
écrire (scénario 5).
