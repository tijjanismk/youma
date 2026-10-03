# 0039 — Fidélité, cartes cadeaux, bons d'avoir, sociétés sous contrat, clients privilégiés

## Contexte

Demande du propriétaire : fidéliser les clients (points dont il définit lui-même la logique), vendre des cartes
cadeaux, offrir un bon d'avoir à un client mécontent, passer des contrats avec des sociétés qui paient une part du
repas de leurs employés, et donner des privilèges (priorité des commandes à distance). Autre retour d'essai : un
client qui recharge la page peut commander une seconde fois en croyant que la première n'est pas partie.

## Décision

- **Points** (RG-FID) : règles dans les paramètres (`Fidelite` : FCFA par point, valeur d'un point, minimum). Solde =
  somme de `mouvements_fidelite` (ajout seul). Gain à l'addition soldée sur ce que le client paie lui-même, retrait à
  l'annulation du paiement, utilisation = une remise tracée.
- **Cartes** (RG-CAD) : `cartes_cadeaux` + `mouvements_carte` (ajout seul, solde calculé). Une carte vendue encaisse
  de l'argent sans chiffre d'affaires (mouvement `vente_carte_cadeau`) ; le bon d'avoir est une carte offerte, sans
  argent entrant, avec le PIN du gérant. Nouveau moyen de paiement `carte_cadeau`.
- **Sociétés** (RG-SOC) : `contrats_societe` ; la part de la société est une part « crédit » sur le compte de la
  société (client à crédit), avec le contrat et le nom de l'employé sur la part. Relevé à facturer par période.
- **Privilèges** (RG-VIP) : `clients.vip` / `vip_jusqu_au` ; tri de la file des commandes reçues et de la cuisine
  (commandes à distance seulement, les tables restent dans l'ordre d'arrivée, comme demandé).
- **Doublons** (RG-CAN-07) : même téléphone et même panier en moins de 5 minutes = même commande, côté poste et côté
  relais ; la page du client montre la commande en cours.
- Migration 0010 : `parts_paiement` est reconstruite (nouveau moyen, colonne `contrat_id`) ; le lanceur de
  migrations coupe les clés étrangères le temps de la migration (marque `-- youma:cles-etrangeres-coupees`) puis les
  vérifie. Test : `migration_0010_garde_les_paiements`.
- La remise au coffre à la clôture (fiche 0037) devient RG-CAI-16 : RG-CAI-15 est le paiement par carte TPE.

## Alternatives écartées

- Tampons (10 repas, le 11e offert) : le propriétaire veut définir lui-même la logique « points et prix ».
- Remise sans contrepartie pour les employés de la société : choix du propriétaire, la société paie sa part.
- Liste d'attente de tables : les tables restent par ordre d'arrivée ; la priorité ne vaut que pour les commandes à
  distance et les clients privilégiés.
- Bloquer le renvoi d'une même commande côté page seulement : un second téléphone ou un cache vidé le contourne.

## Conséquences

- **[HYPOTHÈSE]** Les points se gagnent sur les paiements en espèces, Mobile Money, carte TPE et crédit du client ; pas
  sur la part d'une société, une carte cadeau d'avoir ni les points eux-mêmes (remise).
- **[HYPOTHÈSE]** Une carte cadeau payée par une autre carte cadeau gagne des points (l'argent est déjà entré) ; à revoir
  si le propriétaire préfère l'inverse.
- Cartes cadeaux et bons d'avoir : un code perdu se retrouve dans la liste (Clients → Cartes cadeaux) ; il vaut de
  l'argent, à garder comme tel.
- Le relevé de société et le solde du compte client se recoupent : règlement par `clients::regler`.
