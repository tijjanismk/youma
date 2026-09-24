Tu es un architecte logiciel senior, product designer et développeur full-stack expérimenté dans les logiciels de gestion pour PME et commerces en Afrique de l’Ouest.

Je veux concevoir une application complète de gestion de restaurant destinée en priorité aux petits et moyens restaurants, maquis, fast-foods, cafés et petits établissements au Mali.

IMPORTANT : ne conçois pas un simple POS occidental traduit en français. Le produit doit être pensé autour des réalités opérationnelles d’un restaurant malien : fonctionnement hors ligne, coupures d’électricité, Internet instable, utilisation d’espèces et de Mobile Money, gestion simple des employés, avances sur salaire, ventes à crédit, pertes de stock, livraison locale, fonctionnement avec un seul PC ou avec plusieurs appareils sur un réseau local.

## 1. PRINCIPES DU PRODUIT

Le produit doit être :

* local-first ;
* utilisable sans Internet pour les opérations essentielles ;
* installable localement sur Windows ;
* simple à utiliser par un personnel peu technophile ;
* configurable selon la taille et le fonctionnement du restaurant ;
* capable d'évoluer vers une architecture réseau local ;
* capable d'utiliser des services cloud optionnels ;
* adapté au FCFA et au contexte malien ;
* robuste face aux coupures d'électricité et aux arrêts brutaux ;
* conçu pour pouvoir être vendu sous forme de licence + installation + maintenance ;
* sans abonnement obligatoire pour les fonctions locales.

Le cloud ne doit jamais être une dépendance pour :

* prendre une commande ;
* gérer une table ;
* encaisser ;
* consulter le stock ;
* gérer les employés ;
* calculer les salaires ;
* consulter les rapports locaux.

## 2. MODES D'INSTALLATION

Prévoir trois modes.

### Mode A — Mono-poste

Un seul ordinateur Windows :

Application
→ SQLite
→ toutes les fonctions locales.

### Mode B — Réseau local

Un ordinateur sert de serveur local.

Postes :

* caisse ;
* serveur ;
* cuisine ;
* administration ;
* éventuellement tablettes.

Communication via réseau local.

Prévoir WebSocket pour les événements temps réel :

* nouvelle commande ;
* modification de commande ;
* commande envoyée en cuisine ;
* commande prête ;
* paiement ;
* changement de statut.

### Mode C — Local + Cloud

Le restaurant continue de fonctionner localement.

Le cloud apporte :

* menu QR public ;
* commandes à distance ;
* livraison ;
* sauvegarde distante ;
* synchronisation ;
* consultation à distance par le propriétaire ;
* notifications.

Ne jamais transformer le produit en SaaS obligatoire.

## 3. MODULES PRINCIPAUX

Concevoir les modules suivants.

### Dashboard

Afficher simplement :

* chiffre d'affaires du jour ;
* commandes ;
* ventes sur place ;
* ventes à emporter ;
* ventes livraison ;
* paiements espèces ;
* paiements Mobile Money ;
* dépenses ;
* bénéfice estimé ;
* stock critique ;
* commandes en attente ;
* employés présents ;
* salaires/avances à venir.

Éviter les dashboards remplis de graphiques inutiles.

### Tables et salle

Gestion :

* tables ;
* zones/salles ;
* capacité ;
* libre ;
* occupée ;
* réservée ;
* nettoyage/préparation si pertinent.

Actions :

* ouvrir une table ;
* prendre commande ;
* ajouter des articles ;
* transférer une table ;
* fusionner des tables ;
* diviser une addition ;
* fermer la table.

Prévoir également le mode sans tables pour :

* comptoir ;
* fast-food ;
* vente rapide.

## 4. COMMANDES

Une commande peut être :

* sur place ;
* comptoir ;
* à emporter ;
* livraison ;
* éventuellement commande QR.

Workflow :

Commande créée
→ préparation
→ envoyée en cuisine
→ en préparation
→ prête
→ servie/remise/livrée
→ paiement
→ clôture.

Une commande peut être modifiée tant qu'elle n'est pas définitivement clôturée, avec journalisation des modifications importantes.

Prévoir :

* quantité ;
* variantes ;
* suppléments ;
* options ;
* commentaires ;
* remise ;
* annulation ;
* repas offert ;
* consommation personnelle ;
* client ;
* serveur ;
* table ;
* livreur.

## 5. CUISINE

Prévoir un écran cuisine.

Les commandes doivent être regroupées intelligemment.

Exemple :

TABLE 12

2 × Poulet braisé
1 × Riz sauce
2 × Coca

Note :
"Poulet bien cuit"

La cuisine peut modifier :

* reçu ;
* en préparation ;
* prêt ;
* problème ;
* annulé.

Prévoir éventuellement plusieurs postes :

* cuisine ;
* grill ;
* bar ;
* pâtisserie.

Une commande peut être distribuée entre plusieurs postes.

## 6. MENU ET PRODUITS

Un produit peut être :

* plat ;
* boisson ;
* dessert ;
* accompagnement ;
* supplément ;
* menu/combo.

Informations :

* nom ;
* description ;
* photo ;
* prix ;
* catégorie ;
* disponibilité ;
* code interne ;
* éventuellement code-barres ;
* TVA si nécessaire ;
* recette ;
* prix d'achat estimé.

Ne pas rendre les recettes obligatoires.

Exemple :

Coca 750 FCFA
→ simple produit de vente.

Poulet braisé 3 500 FCFA
→ produit avec recette.

## 7. RECETTES ET STOCK

Le système doit pouvoir gérer les matières premières.

Exemple :

Poulet braisé :

* 1 poulet ;
* huile ;
* oignon ;
* épices ;
* charbon/gaz si le restaurant souhaite le suivre.

Lorsqu'un plat est vendu :
→ calcul de la consommation théorique ;
→ diminution du stock.

Mais prévoir également :

* pertes ;
* produits périmés ;
* casse ;
* repas du personnel ;
* repas offert ;
* consommation interne ;
* correction d'inventaire ;
* vol/perte constatée ;
* régularisation.

Le système doit permettre de comprendre où part le stock.

## 8. ACHATS ET FOURNISSEURS

Prévoir :

* fournisseurs ;
* produits achetés ;
* achats ;
* réception ;
* prix d'achat ;
* dette fournisseur ;
* règlement fournisseur ;
* historique des prix.

Les achats doivent pouvoir fonctionner sans dépendre des ventes.

## 9. CAISSE

Prévoir :

* ouverture de caisse ;
* fond de caisse ;
* ventes ;
* dépenses ;
* retraits ;
* entrées ;
* clôture ;
* différence de caisse ;
* historique.

Moyens de paiement :

* espèces ;
* Orange Money ;
* Moov Money ;
* autres Mobile Money configurables ;
* virement ;
* carte si nécessaire ;
* paiement mixte.

Exemple :

Addition : 12 500 FCFA

Espèces : 5 000
Mobile Money : 7 500

Total : 12 500.

Le système doit enregistrer précisément la répartition.

## 10. CLIENTS ET CRÉDIT

Prévoir un module client simple.

Informations :

* nom ;
* téléphone ;
* adresse ;
* historique des commandes ;
* historique des paiements ;
* dette.

Permettre au restaurateur d'autoriser certains clients à acheter à crédit.

Prévoir :

* limite de crédit ;
* dette actuelle ;
* règlement partiel ;
* règlement total ;
* historique.

Ne jamais supposer que tous les clients peuvent acheter à crédit.

## 11. LIVRAISON

Prévoir un module de livraison.

Commande :

Client
→ adresse/description de localisation
→ téléphone
→ montant
→ statut
→ livreur.

Les adresses doivent être flexibles.

Prévoir :

* quartier ;
* rue si connue ;
* point de repère ;
* téléphone ;
* commentaire ;
* localisation GPS optionnelle.

Statuts :

Nouvelle
→ confirmée
→ préparation
→ prête
→ assignée au livreur
→ récupérée
→ en route
→ livrée
→ annulée.

Prévoir plus tard le suivi GPS temps réel.

Ne pas dépendre d'un service externe pour le fonctionnement de base.

## 12. MENU QR

Chaque table peut avoir son QR Code.

Le client scanne :

QR
→ menu
→ catégorie
→ produit
→ panier
→ commande.

Prévoir deux modes :

### QR consultation

Le client consulte uniquement le menu.

### QR commande

Le client peut envoyer directement sa commande.

Prévoir un QR général pour :

* commandes à emporter ;
* commandes livraison.

Le QR doit être une fonctionnalité optionnelle.

## 13. EMPLOYÉS

Créer une gestion RH légère adaptée aux petits restaurants.

Informations :

* nom ;
* téléphone ;
* fonction ;
* date d'embauche ;
* salaire ;
* type de rémunération ;
* statut ;
* horaires.

Fonctions possibles :

* gérant ;
* serveur ;
* caissier ;
* cuisinier ;
* aide-cuisinier ;
* livreur ;
* plongeur ;
* responsable ;
* autre.

Prévoir :

* présence ;
* absence ;
* retard ;
* congé ;
* avance ;
* prime ;
* retenue ;
* paiement du salaire.

## 14. PAIE

La paie doit rester simple.

Exemple :

Salaire de base : 100 000
Prime : 10 000
Avance : -20 000
Retenue : -5 000

Net à payer : 85 000 FCFA.

Chaque paiement doit générer un historique.

Prévoir plusieurs modes de paiement :

* espèces ;
* Mobile Money ;
* virement.

Ne pas chercher à reproduire immédiatement un logiciel de paie complexe.

## 15. DÉPENSES

Prévoir :

* électricité ;
* eau ;
* gaz ;
* transport ;
* salaire ;
* entretien ;
* achat urgent ;
* téléphone ;
* autres dépenses.

Chaque dépense doit être catégorisée et liée à la caisse lorsqu'elle est payée depuis celle-ci.

## 16. RAPPORTS

Prévoir :

* ventes journalières ;
* ventes mensuelles ;
* produits les plus vendus ;
* ventes par catégorie ;
* ventes par serveur ;
* ventes par moyen de paiement ;
* dépenses ;
* achats ;
* stock ;
* pertes ;
* dettes clients ;
* dettes fournisseurs ;
* salaires ;
* bénéfice estimé.

Prévoir export :

* PDF ;
* Excel/CSV ;
* impression.

## 17. UTILISATEURS ET SÉCURITÉ

Prévoir RBAC :

* propriétaire ;
* administrateur ;
* gérant ;
* caissier ;
* serveur ;
* cuisinier ;
* livreur ;
* stock ;
* RH.

Chaque rôle possède des permissions.

Exemple :

Serveur :

* créer commande ;
* modifier sa commande ;
* voir tables.

Caissier :

* encaisser ;
* ouvrir/fermer caisse.

Gérant :

* rapports ;
* dépenses ;
* stock ;
* employés.

Propriétaire :

* accès complet.

Journaliser les actions sensibles :

* annulation ;
* remise ;
* suppression ;
* modification de prix ;
* correction de stock ;
* modification de salaire ;
* ouverture/fermeture caisse.

## 18. RÉSILIENCE

Le logiciel doit être conçu pour les coupures.

Toutes les opérations importantes doivent être transactionnelles.

Prévoir :

* récupération après arrêt brutal ;
* sauvegarde automatique ;
* sauvegarde manuelle ;
* export USB ;
* restauration ;
* historique ;
* contrôle d'intégrité.

Une coupure électrique ne doit pas corrompre la base de données.

## 19. INTERFACE

L'interface doit être :

* très simple ;
* rapide ;
* tactile-friendly ;
* utilisable avec souris/clavier ;
* lisible sur petits écrans ;
* en français ;
* montants en FCFA ;
* avec grands boutons pour caisse et commandes ;
* avec peu de fenêtres inutiles.

Design moderne mais professionnel.

Ne pas surcharger l'écran.

Pour les opérations fréquentes :

* peu de clics ;
* raccourcis ;
* recherche rapide ;
* catégories visuelles ;
* boutons clairement identifiés.

## 20. ARCHITECTURE TECHNIQUE

Proposer une architecture propre et maintenable.

Privilégier :

Desktop :

* Tauri ;
* React ;
* TypeScript.

Base locale :

* SQLite pour mono-poste.

Mode réseau :

* backend local ;
* PostgreSQL ;
* WebSocket.

Cloud :

* API séparée ;
* PostgreSQL ;
* authentification ;
* synchronisation.

L'architecture doit permettre de commencer simplement puis d'évoluer.

Ne pas introduire inutilement :

* microservices ;
* Kubernetes ;
* architecture distribuée complexe ;
* dépendances cloud obligatoires.

## 21. MODÈLE DE DONNÉES

Avant de coder, concevoir les principales entités :

Restaurant
Utilisateur
Rôle
Permission
Employé
Présence
Salaire
Avance
PaiementSalaire
Client
Fournisseur
Catégorie
Produit
Recette
Ingredient
Stock
MouvementStock
Table
Zone
Commande
LigneCommande
Paiement
Caisse
MouvementCaisse
Dépense
Achat
LigneAchat
Livraison
Livreur
QRMenu
Promotion
JournalAudit
Sauvegarde.

Définir clairement les relations et les règles métier avant d'écrire le code.

## 22. RÈGLE IMPORTANTE DE DÉVELOPPEMENT

Ne commence PAS directement à coder toute l'application.

Commence par produire :

1. vision produit ;
2. fonctionnalités MVP ;
3. fonctionnalités V2 ;
4. architecture ;
5. structure du projet ;
6. modèle de données ;
7. flux métier ;
8. permissions ;
9. stratégie offline/local ;
10. stratégie de synchronisation ;
11. écrans ;
12. composants principaux ;
13. API ;
14. plan de développement par étapes.

Pour chaque module, explique :

* son objectif ;
* ses principales fonctions ;
* ses dépendances ;
* ses règles métier ;
* les données nécessaires ;
* les écrans nécessaires.

Après validation de l'architecture, seulement commencer l'implémentation.

## 23. PRIORITÉ MVP

Le premier MVP doit contenir uniquement :

1. utilisateurs ;
2. produits/catégories ;
3. tables ;
4. commandes ;
5. cuisine ;
6. caisse ;
7. paiements ;
8. dépenses ;
9. stock simple ;
10. clients ;
11. fournisseurs ;
12. employés ;
13. avances et paie simple ;
14. rapports journaliers ;
15. sauvegarde/restauration.

V2 :

* recettes avancées ;
* QR menu ;
* commandes en ligne ;
* livraison ;
* application livreur ;
* suivi GPS ;
* cloud ;
* synchronisation ;
* accès distant ;
* statistiques avancées.

## 24. APPROCHE PRODUIT

Le produit doit être conçu comme un logiciel commercialisable au Mali.

Il doit pouvoir être vendu sous plusieurs formes :

* installation mono-PC ;
* installation réseau ;
* installation + formation ;
* maintenance annuelle ;
* sauvegarde cloud optionnelle ;
* module QR ;
* module livraison ;
* accès distant.

L'objectif est d'avoir un logiciel que le restaurateur peut réellement utiliser quotidiennement, et non une démonstration technique.

Avant toute génération de code, présente-moi l'architecture complète, les choix techniques, le schéma de données et les flux métier afin que je puisse les valider.
