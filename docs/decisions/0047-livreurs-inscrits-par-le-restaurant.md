# 0047 — Livreurs inscrits par le restaurant : téléphone + PIN, courses dans Youma Livreur

## Contexte

Demande du porteur de projet (04/10/2026) : « on inscrit nos livreurs nous-mêmes ». Choix retenu : **téléphone + PIN
donné par le restaurant**. Jusqu'ici (fiche 0041), le livreur recevait pour chaque course un lien à coller ou à ouvrir
dans l'application. Les livreurs sont des employés, souvent occasionnels, sans compte utilisateur sur le poste.

## Décision

1. **RG-LIV-05** : la fiche du livreur (Employés, permission `employe.gerer`) porte un PIN de 4 à 6 chiffres
   (`employes.pin_livreur_hash`, Argon2, migration 0013). Il faut un téléphone à 8 chiffres sur la fiche, un employé
   actif, et un téléphone pour un seul livreur. Le PIN n'est jamais réaffiché ; « Retirer l'accès » l'efface. Actions
   journalisées (`livreur.acces`, `livreur.acces_retire`).
2. **Assignation** : assigner une livraison crée, s'ils manquent, le code de suivi et le code du livreur (commande
   saisie par téléphone), pour que la course soit publiée au relais et que la position fonctionne.
3. **Publication** : à chaque synchronisation (10 s), le poste envoie `livreurs` : téléphone, nom, empreinte du PIN
   et courses assignées ou en route (numéro, client, téléphone, quartier, repère, position, reste à encaisser).
   Le relais remplace sa liste ; un poste plus ancien qui n'envoie rien ne l'efface pas.
4. **Relais** : `POST /api/public/livreur/connexion {telephone, pin}` vérifie le PIN (5 essais en 15 minutes par
   numéro, 20 par adresse) et renvoie un jeton de 30 jours, dont seule l'empreinte est gardée, liée à l'empreinte du
   PIN : un PIN changé ou un accès retiré ferme les sessions. `GET /api/public/livreur/courses` (jeton) donne les courses.
5. **Interface** : écran « Mes courses » (connexion, puis courses rafraîchies toutes les 20 s, « Position du client »,
   « Démarrer la course » vers la page de course existante) dans l'application Youma Livreur (une session par
   restaurant) et sur la page `/livreur` du relais (iPhone, navigateur). Le lien de course reste possible.
6. **Démonstration** : Ibrahim Keïta, 76 55 44 33, PIN 6666.

## Alternatives écartées

- **Compte utilisateur du poste pour chaque livreur** : rôles, permissions et connexion au Wi-Fi inutiles pour un
  livreur qui travaille dehors, sur le relais.
- **Code par SMS ou WhatsApp au livreur** : coût à chaque connexion ; le restaurant connaît ses livreurs et leur
  donne le PIN de vive voix.
- **Vérifier le PIN sur le poste** : le poste n'est pas joignable depuis Internet ; le relais ne reçoit que
  l'empreinte Argon2, jamais le PIN.
- **Le livreur marque lui-même « livrée »** : pas demandé ; le restaurant garde la main sur les statuts et
  l'encaissement (RG-LIV-02/03). À ajouter si le pilote le demande.

## Conséquences

- Le relais garde les adresses, téléphones et positions des clients des courses en cours : effacés dès que la course
  n'est plus assignée ou en route (la liste est remplacée à chaque synchronisation).
- Un PIN à 4 chiffres reste devinable hors ligne si la base du relais fuit (empreinte Argon2) : la changer suffit à
  fermer l'accès ; [HYPOTHÈSE] risque accepté pour des livreurs occasionnels.
- Le lien de course par WhatsApp (fiche 0028) reste utile pour un livreur ponctuel sans accès.
