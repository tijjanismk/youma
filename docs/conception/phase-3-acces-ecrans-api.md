# Phase 3 — Accès, écrans et interfaces

## 1. Matrice des permissions

Générée depuis `youma-core/src/permissions.rs` (rôles par défaut, modifiables par le propriétaire dans
Administration → Rôles et droits ; le rôle propriétaire est intouchable, RG-AUT-05).

| Permission | Propriétaire | Administrateur | Gérant | Caissier | Serveur | Cuisinier | Livreur | Magasinier | RH |
|---|---|---|---|---|---|---|---|---|---|
| `commande.creer` | ✓ | ✓ | ✓ | ✓ | ✓ |  |  |  |  |
| `commande.modifier_autres` | ✓ | ✓ | ✓ |  |  |  |  |  |  |
| `commande.annuler_envoye` | ✓ | ✓ | ✓ |  |  |  |  |  |  |
| `commande.remise` | ✓ | ✓ | ✓ | ✓ |  |  |  |  |  |
| `commande.offrir` | ✓ | ✓ | ✓ |  |  |  |  |  |  |
| `commande.transferer` | ✓ | ✓ | ✓ | ✓ |  |  |  |  |  |
| `commande.conso_employe` | ✓ | ✓ | ✓ |  |  |  |  |  |  |
| `cuisine.voir` | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |  |  |
| `caisse.encaisser` | ✓ | ✓ | ✓ | ✓ |  |  |  |  |  |
| `caisse.session` | ✓ | ✓ | ✓ | ✓ |  |  |  |  |  |
| `caisse.mouvement` | ✓ | ✓ | ✓ | ✓ |  |  |  |  |  |
| `caisse.retrait_proprietaire` | ✓ | ✓ |  |  |  |  |  |  |  |
| `caisse.annuler_paiement` | ✓ | ✓ | ✓ |  |  |  |  |  |  |
| `caisse.verifier_mm` | ✓ | ✓ | ✓ |  |  |  |  |  |  |
| `caisse.ecart` | ✓ | ✓ | ✓ |  |  |  |  |  |  |
| `depense.creer` | ✓ | ✓ | ✓ | ✓ |  |  |  |  |  |
| `stock.voir` | ✓ | ✓ | ✓ | ✓ |  |  |  | ✓ |  |
| `stock.mouvement` | ✓ | ✓ | ✓ |  |  |  |  | ✓ |  |
| `stock.inventaire` | ✓ | ✓ | ✓ |  |  |  |  | ✓ |  |
| `stock.valider_inventaire` | ✓ | ✓ | ✓ |  |  |  |  |  |  |
| `achat.gerer` | ✓ | ✓ | ✓ |  |  |  |  | ✓ |  |
| `client.gerer` | ✓ | ✓ | ✓ | ✓ | ✓ |  |  |  |  |
| `client.credit` | ✓ | ✓ | ✓ | ✓ |  |  |  |  |  |
| `client.depasser_limite` | ✓ | ✓ | ✓ |  |  |  |  |  |  |
| `employe.voir` | ✓ | ✓ | ✓ |  |  |  |  |  | ✓ |
| `employe.gerer` | ✓ | ✓ | ✓ |  |  |  |  |  | ✓ |
| `employe.presence` | ✓ | ✓ | ✓ |  |  |  |  |  | ✓ |
| `employe.avance` | ✓ | ✓ | ✓ |  |  |  |  |  | ✓ |
| `employe.depasser_plafond` | ✓ | ✓ |  |  |  |  |  |  |  |
| `paie.gerer` | ✓ | ✓ | ✓ |  |  |  |  |  | ✓ |
| `livraison.gerer` | ✓ | ✓ | ✓ | ✓ |  |  |  |  |  |
| `rapport.voir` | ✓ | ✓ | ✓ |  |  |  |  |  |  |
| `catalogue.gerer` | ✓ | ✓ | ✓ |  |  |  |  |  |  |
| `salle.gerer` | ✓ | ✓ | ✓ |  |  |  |  |  |  |
| `utilisateur.gerer` | ✓ | ✓ |  |  |  |  |  |  |  |
| `parametre.gerer` | ✓ | ✓ |  |  |  |  |  |  |  |
| `journee.gerer` | ✓ | ✓ | ✓ | ✓ |  |  |  |  |  |
| `sauvegarde.gerer` | ✓ | ✓ | ✓ |  |  |  |  |  |  |
| `audit.voir` | ✓ | ✓ | ✓ |  |  |  |  |  |  |
| `horloge.forcer` | ✓ | ✓ |  |  |  |  |  |  |  |
| `licence.gerer` | ✓ | ✓ |  |  |  |  |  |  |  |
| `appareil.gerer` | ✓ | ✓ | ✓ |  |  |  |  |  |  |
| Plafond de remise | 100 % | 100 % | 50 % | 10 % | 0 % | 0 % | 0 % | 0 % | 0 % |

Une permission absente n'est pas un refus sec : l'action peut être autorisée ponctuellement par le PIN d'un
responsable présent qui détient la permission (RG-AUT-03). Les deux identités sont journalisées.

Choix notables :
* le **dépassement du plafond d'avance** et le **retrait propriétaire** sont réservés au propriétaire ;
* le caissier peut accorder une remise jusqu'à 10 % ; au-delà, PIN du gérant (50 %) ou du propriétaire ;
* le serveur ne peut ni annuler un article envoyé, ni offrir, ni faire de remise ;
* le caissier peut signaler une rupture du jour sans droit de catalogue.

## 2. Écrans

| Écran | Chemin | Rôles principaux |
|---|---|---|
| Connexion (tuiles + PIN) | `/` | tous |
| Première installation | `/` (base vide) | propriétaire |
| Accueil (journée, raccourcis) | `/` | tous |
| Salle (plan des tables par zone) | `/salle` | serveur, caissier |
| Prise de commande | `/commande/:id` | serveur, caissier |
| Encaissement | `/encaisser/:id` | caissier |
| Caisse (session, mouvements, dépenses, comptes, clôture Z) | `/caisse` | caissier, gérant |
| Cuisine / bar | `/cuisine` | cuisinier, barman |
| Livraisons et remise livreur | `/livraisons` | caissier, gérant |
| Ma journée (tableau de bord) | `/tableau-de-bord` | gérant, propriétaire |
| Mobile Money à vérifier | `/mobile-money` | gérant |
| Stock, inventaire | `/stock` | magasinier, gérant |
| Achats, fournisseurs | `/achats` | magasinier, gérant |
| Clients et crédit | `/clients` | caissier, gérant |
| Employés, présences | `/employes`, `/employes/:id` | RH, gérant |
| Paie | `/paie` | RH, gérant |
| Rapports (+ CSV, impression) | `/rapports` | gérant, propriétaire |
| Journal d'audit | `/journal` | propriétaire |
| Administration | `/administration` | propriétaire, gérant |

### Maquettes textuelles des écrans principaux

**Prise de commande** (objectif ≤ 6 touches pour 3 articles ; atteint : 5)

```
┌ [Rechercher un produit…]                        ┐┌ Table 4 ─────────── Ouverte ┐
│ [🥤 Boissons] [🍺 Bières] [🍗 Grillades] [🍛 …]   ││ 3× Bière blonde   Envoyé 3000│
│ ┌──────────┐ ┌──────────┐ ┌──────────┐           ││ 2× Brochettes  [−][+]   3000│  ← panier local
│ │Bière     │ │Coca-Cola │ │Eau       │           ││─────────────────────────────│
│ │1 000 FCFA│ │  750 FCFA│ │  500 FCFA│           ││ Total           6 000 FCFA  │
│ └──────────┘ └──────────┘ └──────────┘           ││ [ Envoyer (2) ]  (vert)     │
│                                                  ││ [ Encaisser 6 000 FCFA ]    │
└──────────────────────────────────────────────────┘│ [ Plus… ] [ ← Salle ]       │
                                                    └─────────────────────────────┘
```
Toucher une ligne envoyée : Annuler (motif, PIN), Offrir, Remise. « Plus… » : addition, remise globale,
transfert, fusion, client (crédit), consommation d'un employé, abandon.

**Encaissement** (standard < 5 s : « Espèces » → « Valider »)

```
┌ Reste dû          12 500 FCFA ┐┌ Répartition ─────────────────────┐
│ Diviser : [2][3][4][5] parts  ││ Espèces            [ 5 000 ]  ✕  │
│ À encaisser  [ 12 500 ]       ││ Orange Money       [ 7 500 ]  ✕  │
│ [💵 Espèces] [📱 Orange Money] ││   Référence  [PP260314.1830…]    │
│ [📱 Moov]    [🧾 Crédit]       ││ Espèces reçues [10 000] [5 000]… │
└───────────────────────────────┘│ Rendu : 5 000 FCFA               │
                                 │ [      Valider le paiement     ] │
                                 └──────────────────────────────────┘
```

**Cuisine** : une carte par envoi, gros caractères, temps d'attente, bordure rouge au-delà de 20 min,
boutons « En préparation », « Prêt ✓ », « Servi », « Problème ». Bandeau des tickets non imprimés avec réimpression.

**Clôture de caisse** : billetage par coupure (10 000 → 5), attendu / compté / écart en direct ; motif
obligatoire au-delà du seuil ; rapport Z affiché et imprimable.

## 3. Composants d'interface

`PinPad` (pavé tactile), `ChampMontant` (entiers FCFA + raccourcis), `Modal`, `Onglets`, `DemandeMotif`
(motifs proposés en un clic), `TableauDonnees` (défilement horizontal sur téléphone), `Billetage`,
`BulletinImprimable`, `AffichageRapport` (indicateurs + formules). Le contexte `useApp().agir()` rejoue une
action avec le PIN d'un responsable quand l'API répond `AUTORISATION_REQUISE`.

## 4. API du poste central

Toutes les routes sont sous `/api`, en JSON. Authentification : `Authorization: Bearer <jeton>` ;
autorisation ponctuelle : `X-Autorisation-Pin: <PIN>` ; appareil distant (mode B) : `X-Appareil: <jeton>`.

### Format d'erreur

```json
{ "code": "AUTORISATION_REQUISE", "message": "…", "regle": "RG-AUT-03", "permission": "commande.annuler_envoye" }
```

| Code | HTTP | Sens |
|---|---|---|
| `NON_AUTHENTIFIE` | 401 | session absente ou expirée (RG-AUT-04) |
| `PIN_INCORRECT` | 401 | PIN faux |
| `VERROUILLE` | 423 | 5 échecs (RG-AUT-02) |
| `INTERDIT` | 403 | droit de lecture manquant, appareil non autorisé |
| `AUTORISATION_REQUISE` | 403 | action possible avec le PIN d'un responsable |
| `VALIDATION`, `REGLE_METIER` | 422 | données ou règle `RG-*` (champ `regle`) |
| `HORLOGE_INCOHERENTE` | 409 | RG-SYS-01 |
| `AJOUT_SEUL` | 409 | tentative de modification d'une donnée financière |
| `NON_TROUVE` | 404 | |

### Routes

| Domaine | Routes |
|---|---|
| Système | `GET /etat`, `POST /installation`, `GET /connexion/utilisateurs`, `POST /connexion`, `POST /deconnexion`, `GET /session`, `POST /horloge/accepter`, `GET /ws?jeton=` |
| Journée | `GET /journees`, `POST /journee/ouvrir`, `POST /journee/cloturer` |
| Catalogue | `GET /catalogue`, `POST /categories`, `POST /produits`, `POST /produits/import` (CSV), `POST /produits/{id}/disponibilite`, `GET /produits/{id}/historique`, `POST /postes` |
| Salle | `GET /salle`, `POST /zones`, `POST /tables`, `POST /tables/serie`, `POST /tables/{id}/marquer` |
| Commandes | `GET /commandes?statut=`, `POST /commandes`, `GET /commandes/{id}`, `POST /commandes/{id}/lignes`, `POST /commandes/{id}/envoyer`, `POST /commandes/{id}/remise`, `POST /commandes/{id}/transferer`, `POST /commandes/{id}/fusionner`, `POST /commandes/{id}/abandonner`, `POST /commandes/{id}/imputer`, `POST /commandes/{id}/client`, `GET /commandes/{id}/ticket`, `POST /commandes/{id}/imprimer`, `GET /commandes/{id}/diviser?parts=`, `POST /commandes/{id}/montant-lignes`, `GET /commandes/{id}/paiements`, `PUT /lignes/{id}`, `POST /lignes/{id}/annuler`, `POST /lignes/{id}/offrir` |
| Cuisine | `GET /cuisine?poste=`, `POST /envois/{id}/statut`, `GET /impressions`, `POST /impressions/{id}/reimprimer` |
| Caisse | `GET /caisse`, `POST /caisse/ouvrir`, `POST /caisse/{id}/cloturer`, `GET /caisse/{id}/z`, `POST /caisse/encaisser`, `POST /caisse/mouvement`, `POST /caisse/transfert`, `GET /caisse/mouvements`, `POST /paiements/{id}/annuler`, `GET|POST /comptes`, `GET|POST /depenses`, `POST /depenses/{id}/annuler`, `GET|POST /depenses/categories`, `GET /mobile-money?statut=`, `POST /mobile-money/{id}/verifier` |
| Stock, achats | `GET /stock`, `POST /stock/articles`, `GET /stock/articles/{id}/mouvements`, `POST /stock/mouvements`, `GET|POST /inventaires`, `GET /inventaires/{id}`, `POST /inventaires/{id}/comptage`, `POST /inventaires/{id}/valider`, `POST /inventaires/{id}/abandonner`, `GET|POST /fournisseurs`, `POST /fournisseurs/reglement`, `GET|POST /achats` |
| Clients | `GET /clients?q=`, `POST /clients`, `GET /clients/{id}`, `POST /clients/reglement` |
| Employés, paie | `GET|POST /employes`, `GET /employes/references`, `GET /employes/{id}`, `POST /employes/avance`, `POST /employes/evenement`, `GET|POST /presences`, `GET /paie/apercu`, `POST /paie/cloturer`, `POST /paie/payer`, `GET /paie/bulletins`, `GET /paie/bulletins/{id}` |
| Livraison | `GET /livraisons`, `POST /livraisons/{id}/assigner`, `POST /livraisons/{id}/statut`, `GET /livreurs`, `POST /livreurs/{id}/remise` |
| Rapports | `GET /tableau-de-bord`, `GET /rapports/periode?debut&fin[&format=csv]`, `GET /rapports/stock`, `GET /rapports/dettes`, `GET /audit?action=` |
| Administration | `GET|PUT /parametres`, `GET|PUT /restaurant`, `GET|PUT /roles`, `GET|POST /utilisateurs`, `PUT /utilisateurs/{id}`, `GET /appareils`, `POST /appareils/code`, `POST /appareils/appairer`, `POST /appareils/{id}/revoquer`, `GET /reseau`, `GET /diagnostic`, `GET|POST /sauvegardes`, `POST /sauvegardes/exporter`, `POST /sauvegardes/restaurer`, `POST /integrite`, `GET|POST /licence` |

### Événements WebSocket

`{ "type": "<type>", "id": "<identifiant ou null>" }` après chaque commit : `commande`, `envoi`,
`envoi_pret`, `paiement`, `table`, `caisse`, `stock`, `catalogue`, `journee`, `livraison`, `impression`,
`employes`, plus `connecte` et `resynchroniser` (client en retard : il recharge). L'interface recharge
les données concernées ; la reconnexion est automatique (1 s → 15 s).
