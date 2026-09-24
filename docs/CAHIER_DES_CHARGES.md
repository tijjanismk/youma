# Cahier des charges — Logiciel de gestion de restaurant (Mali) — v2

## 0. RÔLE, CONTEXTE ET MÉTHODE

Tu es un architecte logiciel senior, product designer et développeur full-stack expérimenté dans les logiciels de gestion pour PME et commerces en Afrique de l'Ouest.

Je veux concevoir une application complète de gestion de restaurant destinée en priorité aux petits et moyens restaurants, maquis, fast-foods, cafés et petits établissements au Mali.

IMPORTANT : ne conçois pas un simple POS occidental traduit en français. Le produit doit être pensé autour des réalités opérationnelles d'un restaurant malien (section 2).

### Contexte du porteur de projet (à compléter)

* Équipe : [solo / N développeurs]
* Compétences : [ex. Rust, TypeScript, React, Tauri 2]
* Restaurant pilote disponible : [oui / non — lequel]
* Délai visé pour une première version utilisable en réel : [ex. 4 mois]
* Mode de vente visé en premier : [mono-poste / réseau]

Adapte l'ampleur du plan à ces moyens. Un plan irréalisable par l'équipe réelle n'est pas un bon plan.

### Règles de réponse

* **Ne commence PAS à coder.** Produis d'abord la conception, par phases (section 26), et arrête-toi à chaque point de validation.
* Quand une exigence est ambiguë, prends une décision, marque-la **[HYPOTHÈSE]** et justifie-la en une phrase.
* Si ce document se contredit ou impose un choix que tu juges mauvais, **dis-le explicitement** et propose une alternative plutôt que de trancher en silence.
* Numérote les règles métier (`RG-CMD-01`, `RG-CAI-03`…) pour pouvoir les citer ensuite dans le code, les tests et les discussions.
* Pour chaque choix technique important, donne une fiche courte : décision, alternatives écartées, raison, conséquence.
* Liste à la fin de chaque phase les **questions ouvertes** qui demandent ma réponse.

---

## 1. PRINCIPES DU PRODUIT

Le produit doit être :

* local-first ;
* utilisable sans Internet pour toutes les opérations essentielles ;
* installable localement sur Windows, **sans connexion Internet pendant l'installation** ;
* simple à utiliser par un personnel peu technophile, parfois peu à l'aise avec l'écrit ;
* configurable selon la taille et le fonctionnement du restaurant ;
* capable d'évoluer vers une architecture réseau local sans réinstallation ni migration de données ;
* capable d'utiliser des services cloud optionnels ;
* adapté au FCFA et au contexte malien ;
* robuste face aux coupures d'électricité, aux arrêts brutaux et aux horloges système déréglées ;
* conçu pour être vendu sous forme de licence + installation + maintenance ;
* sans abonnement obligatoire pour les fonctions locales.

Le cloud ne doit jamais être une dépendance pour :

* prendre une commande ;
* gérer une table ;
* encaisser ;
* imprimer un ticket ;
* consulter le stock ;
* gérer les employés ;
* calculer les salaires ;
* consulter les rapports locaux.

**Une licence expirée ou un contrat de maintenance échu ne doit jamais bloquer les ventes.** Au pire, il bloque les mises à jour et les modules cloud.

---

## 2. RÉALITÉS TERRAIN À INTÉGRER DANS LA CONCEPTION

Ces contraintes doivent se retrouver dans l'architecture, le modèle de données et les écrans, pas seulement dans un paragraphe d'intention.

**Énergie et matériel**

* Coupures fréquentes. Un onduleur n'est pas toujours présent. Un PC portable (batterie) est souvent le meilleur « serveur ».
* Des PC anciens ou d'entrée de gamme (4 Go de RAM, parfois disque dur mécanique).
* Pile BIOS usée : après une coupure, **l'horloge du PC peut revenir à une date ancienne**. Les rapports, la numérotation et la synchronisation ne doivent pas en être corrompus.
* Redémarrages forcés par Windows Update, antivirus agressifs, disque plein.

**Organisation du service**

* Les maquis et bars ferment souvent après minuit. Une vente à 1 h 30 appartient à la **journée d'exploitation** de la veille, pas au jour calendaire.
* Au maquis, une table commande par **tournées** successives sur une même addition (3 bières, puis 2 autres, puis des brochettes).
* Prix parfois différents selon l'espace (salle climatisée, VIP, terrasse).
* Les serveurs utilisent souvent **leur propre téléphone Android**. Aucune installation d'application ne doit être nécessaire pour prendre une commande en réseau local.
* Beaucoup de petits établissements n'achèteront pas d'écran cuisine : le **ticket cuisine imprimé** est le cas principal, l'écran est l'option.
* Passation de caisse entre deux caissiers au cours d'une même journée.

**Argent**

* Montants en FCFA, **entiers, sans centimes**.
* Espèces majoritaires. Comptage par coupures (billetage) utile à l'ouverture et à la clôture.
* Mobile Money (Orange Money, Moov Money, autres opérateurs configurables) : l'argent n'est pas dans le tiroir mais sur un compte. La fraude par faux SMS ou fausse capture d'écran est courante. Il faut pouvoir saisir la **référence de transaction** et suivre les paiements « à vérifier ».
* Le propriétaire prélève souvent de l'argent dans la caisse : c'est un **retrait propriétaire**, pas une dépense.
* Ventes à crédit (ardoise) pour certains clients connus, et consommations des employés déduites du salaire.
* Livraisons payées en espèces au livreur : l'argent n'arrive en caisse qu'au retour du livreur.

**Stock**

* Achats par conditionnement (casier de 24, carton, sac de 25 kg, bidon de 20 L), ventes à l'unité ou à la portion.
* Bouteilles et casiers **consignés** chez les distributeurs de boissons (module optionnel).
* Les boissons sont le premier poste de pertes non expliquées : leur suivi à l'unité est prioritaire sur les recettes détaillées.

**Contrôle et confiance**

* La première attente du propriétaire est souvent de **limiter les pertes et les détournements** : articles servis mais non enregistrés, annulations après encaissement, remises non autorisées. Le logiciel doit rendre ces écarts visibles, sans devenir un outil policier pénible au quotidien.

---

## 3. MODES D'INSTALLATION

Prévoir trois modes. **Ils doivent partager le même code et la même base locale.** Passer du mode A au mode B consiste à activer le réseau et ajouter des postes, pas à migrer des données.

### Mode A — Mono-poste

Un seul ordinateur Windows. L'application embarque son serveur local et sa base SQLite. Toutes les fonctions locales sont disponibles.

### Mode B — Réseau local

Un ordinateur sert de **poste central** (idéalement le PC de caisse, sur onduleur ou portable). Il détient la base et expose l'interface sur le réseau local.

Postes possibles :

* caisse ;
* serveur (téléphone ou tablette via navigateur, sans installation) ;
* cuisine / bar (écran ou imprimante) ;
* administration ;
* tablettes.

Prévoir WebSocket pour les événements temps réel :

* nouvelle commande ;
* modification de commande ;
* envoi en cuisine / bar ;
* commande prête ;
* paiement ;
* changement de statut de table ;
* poste déconnecté / reconnecté.

À traiter explicitement :

* découverte du poste central sur le réseau (adresse IP fixe, QR de connexion affiché par le poste central, ou autre) ;
* enregistrement et révocation des appareils autorisés ;
* comportement d'un poste client quand le poste central est injoignable (message clair, pas de perte de saisie en cours) ;
* procédure de secours documentée si le poste central tombe en panne en plein service.

### Mode C — Local + Cloud

Le restaurant continue de fonctionner localement. Le poste central reste la **source de vérité**. Le cloud est un réplica et une boîte de réception.

Le cloud apporte :

* menu QR public ;
* commandes à distance (reçues dans une file que le personnel valide) ;
* livraison ;
* sauvegarde distante chiffrée ;
* consultation à distance par le propriétaire ;
* notifications, et résumé de fin de journée (SMS/WhatsApp) envoyé dès qu'une connexion est disponible.

Ne jamais transformer le produit en SaaS obligatoire.

---

## 4. RÈGLES TRANSVERSES (s'appliquent à tous les modules)

### Argent

* Tous les montants sont des **entiers en FCFA**. Jamais de nombres à virgule flottante.
* Arrondis configurables (ex. au multiple de 25 ou 50 FCFA) sur les remises et les divisions d'addition. L'écart d'arrondi est enregistré, pas perdu.

### Journal plutôt que solde modifiable

Les soldes suivants sont **calculés à partir de mouvements**, jamais saisis à la main :

* stock (mouvements de stock) ;
* caisse et comptes Mobile Money / banque (mouvements de trésorerie) ;
* dette client (ventes à crédit et règlements) ;
* dette fournisseur (achats et règlements) ;
* compte employé (avances, retenues, consommations, paiements).

Un solde peut être mis en cache pour la performance, mais il doit toujours pouvoir être recalculé et vérifié.

### Immutabilité des opérations financières

* Un paiement, un mouvement de caisse ou un ticket encaissé n'est **jamais modifié ni supprimé**.
* Une erreur se corrige par une **écriture d'annulation** (contre-passation) liée à l'originale, avec motif, auteur et heure.
* La suppression physique est interdite sur les données financières et de stock.

### Journée d'exploitation

* Une journée d'exploitation est ouverte et clôturée explicitement. Elle a une heure de bascule configurable (ex. 6 h).
* Toutes les ventes, dépenses et sessions de caisse sont rattachées à une journée, **indépendamment de l'horloge du PC**.
* Les rapports « journaliers » portent sur les journées d'exploitation.

### Horloge

* Horodatage stocké en UTC (le Mali est en UTC+0, sans heure d'été, mais c'est une bonne pratique).
* Au démarrage et à chaque écriture, détecter une horloge **antérieure au dernier événement enregistré**. Dans ce cas, bloquer et demander à un responsable de corriger la date.

### Numérotation

* Identifiants techniques globalement uniques (UUIDv7 ou ULID), pour permettre la synchronisation sans collision.
* Numéros lisibles (ticket, facture, bon cuisine) attribués par le poste central, séquentiels, **sans trou** pour les factures. Un ticket annulé garde son numéro.

### Transactions

* Toute opération métier qui touche plusieurs tables (encaissement, clôture, réception d'achat, paie) s'exécute dans **une seule transaction** base de données.
* Les règles métier vivent dans le backend, jamais dans l'interface. L'interface ne parle jamais directement à la base.

### Autorisation ponctuelle

* Une action sensible peut être autorisée ponctuellement par un responsable présent (saisie de son code PIN), sans changer d'utilisateur. L'autorisation est journalisée avec les deux identités.

---

## 5. MODULES PRINCIPAUX

Pour chaque module, décris : objectif, fonctions, dépendances, règles métier numérotées, données nécessaires, écrans nécessaires.

### Dashboard

Afficher simplement, pour la journée d'exploitation en cours :

* chiffre d'affaires ;
* nombre de commandes ;
* ventes sur place / à emporter / livraison ;
* encaissements espèces / Mobile Money (dont « à vérifier ») ;
* ventes à crédit du jour ;
* dépenses ;
* bénéfice estimé (formule affichée, voir section 18) ;
* stock critique ;
* commandes en attente ;
* annulations et remises du jour (nombre et montant) ;
* employés présents ;
* salaires et avances à venir.

Éviter les dashboards remplis de graphiques inutiles. Un propriétaire doit comprendre sa journée en 10 secondes.

### Tables et salle

Gestion :

* zones / salles, avec **grille de prix** optionnelle par zone ;
* tables et capacité ;
* statuts : libre, occupée, réservée, à nettoyer si pertinent.

Actions :

* ouvrir une table ;
* prendre commande ;
* ajouter des articles (nouvelle tournée) ;
* transférer une table ;
* fusionner des tables ;
* diviser une addition (par articles, en parts égales, par montant) ;
* fermer la table.

Prévoir aussi le mode sans tables pour comptoir, fast-food et vente rapide.

---

## 6. COMMANDES

Une commande peut être :

* sur place ;
* comptoir ;
* à emporter ;
* livraison ;
* commande QR / en ligne (V2).

### Deux notions à distinguer

* **L'addition** (ou note) : ce que le client doit payer. Elle reste ouverte pendant tout le repas.
* **L'envoi** (ou bon) : un groupe d'articles envoyé à un poste de préparation à un instant donné. Une addition contient plusieurs envois (tournées).

Le statut de préparation se suit **par envoi et par ligne**. Le statut de l'addition en découle.

### Workflow

Brouillon (en saisie)
→ envoyée (un ou plusieurs envois vers cuisine / bar)
→ en préparation
→ prête
→ servie / remise / livrée
→ payée (totalement ou partiellement)
→ clôturée.

Deux ordres de paiement doivent être possibles, configurables par type de commande :

* **payer d'abord** (comptoir, fast-food) : encaissement puis envoi en cuisine ;
* **payer à la fin** (table) : envois successifs puis encaissement.

Une commande peut être modifiée tant qu'elle n'est pas clôturée. Règles à préciser :

* modifier un article **non encore envoyé** : libre ;
* annuler un article **déjà envoyé** : motif obligatoire, autorisation selon le rôle, bon d'annulation imprimé pour la cuisine, journalisation ;
* modifier une commande **déjà encaissée** : uniquement par contre-passation.

Données par ligne ou par commande :

* quantité ;
* variantes (ex. taille) ;
* suppléments et options (groupes d'options avec min/max) ;
* commentaires (« poulet bien cuit ») ;
* remise (ligne ou total, motif, plafond selon rôle) ;
* annulation ;
* repas offert (sort du stock, sans recette, motif obligatoire) ;
* consommation personnelle d'un employé (imputable sur son compte) ;
* client ;
* serveur ;
* table ;
* livreur.

---

## 7. CUISINE ET POSTES DE PRÉPARATION

### Postes

Plusieurs postes configurables : cuisine, grill, bar, pâtisserie… Chaque produit est rattaché à un poste. Une commande est **répartie automatiquement** entre les postes concernés.

### Sortie par poste

Chaque poste reçoit ses envois :

* sur **imprimante ticket** (cas principal, MVP) ;
* et/ou sur **écran** (option).

Si l'imprimante d'un poste est en erreur, l'envoi n'est pas perdu : il est mis en attente, signalé à la caisse, et réimprimable.

### Regroupement

Exemple de ticket ou d'écran :

```
TABLE 12 — Envoi n°2 — 20:14 — Serveur : Awa

2 × Poulet braisé
1 × Riz sauce arachide

Note : "Poulet bien cuit"
```

Les boissons partent au bar, pas en cuisine.

### Statuts côté cuisine (écran)

reçu → en préparation → prêt, plus : problème (avec message au serveur), annulé.

Temps d'attente visible par envoi. Gros caractères, lisible à distance.

---

## 8. MENU ET PRODUITS

Un produit peut être :

* plat ;
* boisson ;
* dessert ;
* accompagnement ;
* supplément ;
* menu / combo.

Informations :

* nom (court pour les tickets, long pour le menu QR) ;
* description ;
* photo (utile pour un personnel peu à l'aise avec l'écrit) ;
* prix, et prix par zone si activé ;
* catégorie (avec couleur et icône) ;
* poste de préparation ;
* disponibilité (rupture du jour en un clic) ;
* code interne ;
* code-barres éventuel ;
* TVA si nécessaire (désactivée par défaut, taux configurable) ;
* mode de suivi du stock : aucun / article revendu (1 vente = 1 unité) / recette ;
* prix d'achat estimé.

Ne pas rendre les recettes obligatoires.

Exemples :

* Coca 750 FCFA → article revendu, stock suivi à la bouteille.
* Poulet braisé 3 500 FCFA → produit avec recette (V2) ou sans suivi (MVP).

Toute modification de prix est journalisée et **n'affecte pas les commandes déjà enregistrées** (le prix est copié dans la ligne de commande).

---

## 9. RECETTES ET STOCK

### Unités et conditionnements

Chaque article de stock a une unité de base (bouteille, kg, litre, pièce) et des conditionnements d'achat convertibles (casier de 24, carton de 12, sac de 25 kg…). Une réception d'un casier ajoute 24 bouteilles.

### Consommation

Lorsqu'un produit est vendu :

* article revendu → sortie d'une unité ;
* produit avec recette (V2) → calcul de la consommation théorique et sorties correspondantes.

Suivi optionnel des consommables d'énergie (charbon, gaz) si le restaurant le souhaite.

### Mouvements hors vente

Chaque mouvement a un type et un motif :

* achat / réception ;
* perte ;
* produit périmé ;
* casse ;
* repas du personnel ;
* repas offert ;
* consommation interne ;
* vol / perte constatée ;
* correction d'inventaire ;
* régularisation ;
* retour fournisseur.

### Inventaire

* Inventaire complet ou partiel (ex. « boissons » chaque soir).
* Saisie des quantités comptées, calcul de l'écart théorique / réel, validation par un responsable, génération des mouvements d'écart.

Le système doit permettre de comprendre **où part le stock** : rapport ventes vs sorties vs écarts d'inventaire, par article et par période.

### Consignes (option)

Suivi des emballages consignés (bouteilles, casiers) : consignes versées au fournisseur, emballages vides en stock, retours.

---

## 10. ACHATS ET FOURNISSEURS

Prévoir :

* fournisseurs ;
* articles achetés et conditionnements ;
* bon d'achat (optionnel) ;
* réception, y compris **achat direct au marché sans bon préalable** ;
* prix d'achat et historique des prix ;
* paiement comptant depuis la caisse, ou à crédit ;
* dette fournisseur ;
* règlement fournisseur (partiel ou total, par tout moyen de paiement) ;
* historique.

Les achats doivent pouvoir fonctionner sans dépendre des ventes.

---

## 11. CAISSE ET TRÉSORERIE

### Comptes de trésorerie

Modéliser l'argent par **comptes** :

* caisse espèces (une ou plusieurs) ;
* compte Orange Money ;
* compte Moov Money ;
* autres Mobile Money configurables ;
* banque ;
* coffre / propriétaire (option).

Transferts entre comptes : versement de la caisse au propriétaire, dépôt d'espèces sur un compte Mobile Money, retrait Mobile Money en espèces (avec frais éventuels).

### Session de caisse

* ouverture avec fond de caisse (billetage optionnel) ;
* ventes ;
* dépenses payées depuis la caisse ;
* entrées diverses ;
* retraits, dont **retrait propriétaire** distinct des dépenses ;
* passation entre caissiers (clôture d'une session, ouverture de la suivante) ;
* clôture avec comptage réel, calcul de l'écart, motif obligatoire au-delà d'un seuil ;
* historique et impression du rapport de clôture (Z).

### Moyens de paiement

* espèces (avec calcul du rendu monnaie) ;
* Orange Money ;
* Moov Money ;
* autres Mobile Money configurables ;
* virement ;
* carte si nécessaire ;
* crédit client (ardoise) ;
* paiement mixte.

Exemple :

```
Addition : 12 500 FCFA
Espèces : 5 000
Orange Money : 7 500 (référence de transaction saisie)
Total : 12 500
```

Le système enregistre précisément la répartition, chaque part étant affectée au bon compte de trésorerie.

### Mobile Money

* Saisie de la référence de transaction et du numéro payeur (obligatoire ou facultatif selon configuration).
* Statut « vérifié / à vérifier ». Liste des paiements à vérifier pour le responsable.
* V2 : rapprochement avec un relevé d'opérateur importé (CSV).

---

## 12. CLIENTS ET CRÉDIT

Module client simple.

Informations :

* nom ;
* téléphone (clé de recherche principale) ;
* adresse / repères ;
* historique des commandes ;
* historique des paiements ;
* dette.

Crédit :

* **désactivé par défaut** pour chaque client ; le restaurateur l'autorise individuellement ;
* limite de crédit ;
* dette actuelle ;
* dépassement de limite : refusé, sauf autorisation ponctuelle d'un responsable (journalisée) ;
* règlement partiel ou total, par tout moyen de paiement ;
* historique et relevé imprimable ;
* rapport des dettes par ancienneté.

Ne jamais supposer que tous les clients peuvent acheter à crédit.

---

## 13. LIVRAISON (V2)

Commande livraison :

Client → adresse / description de localisation → téléphone → montant → frais de livraison → statut → livreur.

Adresses flexibles :

* quartier (liste configurable) ;
* rue si connue ;
* point de repère ;
* téléphone ;
* commentaire ;
* localisation GPS optionnelle.

Frais de livraison par quartier ou par zone.

Statuts :

Nouvelle → confirmée → en préparation → prête → assignée au livreur → récupérée → en route → livrée / échec → annulée.

Livreurs : employés ou livreurs externes (moto-taxi), rémunération fixe ou par course.

**Rapprochement livreur** : l'argent encaissé par le livreur reste « à remettre » jusqu'à son retour ; la remise livreur le transfère en caisse et fait apparaître les écarts.

Suivi GPS temps réel : plus tard. Aucun service externe requis pour le fonctionnement de base.

---

## 14. MENU QR (V2, optionnel)

Chaque table peut avoir son QR code.

Parcours client : QR → menu → catégorie → produit → panier → commande.

Deux modes :

* **QR consultation** : le client consulte le menu. Peut fonctionner avec une simple page publique mise à jour depuis le poste central.
* **QR commande** : le client envoie sa commande. Elle arrive dans une **file de validation** ; un membre du personnel l'accepte avant tout envoi en cuisine (protection contre les commandes fantaisistes).

Un QR général pour les commandes à emporter et en livraison.

Préciser le chemin réseau : les clients utilisent généralement leurs données mobiles et non le Wi-Fi du restaurant, donc le QR commande passe par le cloud, et le poste central récupère les commandes dès qu'il est connecté.

---

## 15. EMPLOYÉS

Gestion RH légère adaptée aux petits restaurants.

**Un employé n'est pas un utilisateur.** Un plongeur n'a pas de compte ; un propriétaire a un compte sans être employé. Les deux entités sont liées quand c'est le cas.

Informations :

* nom ;
* téléphone ;
* fonction ;
* date d'embauche ;
* type de rémunération : mensuel, journalier, à la tâche / par course ;
* salaire ou taux ;
* statut (actif, suspendu, parti) ;
* horaires.

Fonctions possibles : gérant, serveur, caissier, cuisinier, aide-cuisinier, livreur, plongeur, responsable, autre.

Événements :

* présence ;
* absence (justifiée ou non) ;
* retard ;
* congé ;
* avance ;
* prime ;
* retenue ;
* consommation personnelle imputée ;
* paiement du salaire.

---

## 16. PAIE

La paie reste simple.

Exemple :

```
Salaire de base :        100 000
Prime :                   10 000
Avance :                 -20 000
Retenue :                 -5 000
Consommations :           -3 000
Net à payer :             82 000 FCFA
```

Règles à définir explicitement :

* plafond d'avance configurable (montant ou % du salaire) ;
* **net négatif** : le solde est reporté sur le mois suivant, jamais perdu ;
* déduction des absences : activable ou non ;
* paiement partiel du salaire possible ;
* une période de paie clôturée n'est plus modifiable (correction par régularisation sur la période suivante).

Chaque paiement génère un historique et un bulletin simple imprimable. Il est payé depuis un compte de trésorerie (espèces, Mobile Money, virement) et apparaît dans la caisse correspondante.

Ne pas chercher à reproduire un logiciel de paie complexe (pas de cotisations sociales en MVP).

---

## 17. DÉPENSES

Catégories configurables, par défaut :

* électricité ;
* eau ;
* gaz / charbon ;
* transport ;
* salaire ;
* entretien / réparation ;
* achat urgent ;
* téléphone / crédit / Internet ;
* loyer ;
* taxes ;
* autres dépenses.

Chaque dépense est catégorisée, porte un bénéficiaire et un justificatif optionnel (photo), et est liée au compte de trésorerie qui l'a payée (la caisse le plus souvent).

---

## 18. RAPPORTS

Prévoir :

* ventes par journée d'exploitation ;
* ventes mensuelles ;
* produits les plus vendus ;
* ventes par catégorie ;
* ventes par serveur ;
* ventes par type de commande ;
* ventes par moyen de paiement ;
* annulations, remises et repas offerts (par employé et par motif) ;
* écarts de caisse par caissier ;
* dépenses ;
* achats ;
* stock et valeur du stock ;
* pertes et écarts d'inventaire ;
* dettes clients ;
* dettes fournisseurs ;
* salaires et avances ;
* bénéfice estimé.

**Définir chaque indicateur par une formule écrite**, affichée dans le rapport. Exemple à préciser :

> Bénéfice estimé = Chiffre d'affaires − coût d'achat estimé des produits vendus − dépenses de la période − salaires de la période.

Un propriétaire qui ne sait pas comment un chiffre est calculé finit par ne plus le croire.

Export :

* PDF ;
* Excel / CSV ;
* impression (A4 et ticket 80 mm pour les rapports de clôture).

---

## 19. UTILISATEURS ET SÉCURITÉ

### Connexion

* Code PIN court (4 à 6 chiffres) par utilisateur pour les postes de service ; changement d'utilisateur rapide.
* Mot de passe pour les rôles d'administration.
* Verrouillage automatique après inactivité, configurable.

### RBAC

Rôles par défaut :

* propriétaire ;
* administrateur ;
* gérant ;
* caissier ;
* serveur ;
* cuisinier ;
* livreur ;
* stock ;
* RH.

Chaque rôle possède des permissions fines. Rôles modifiables par le propriétaire.

Exemples :

* Serveur : créer une commande, modifier sa commande avant envoi, voir les tables.
* Caissier : encaisser, ouvrir / fermer sa session de caisse.
* Gérant : rapports, dépenses, stock, employés, autoriser annulations et remises.
* Propriétaire : accès complet.

Produire une **matrice rôles × permissions** complète.

### Journal d'audit

Journaliser (qui, quand, depuis quel poste, avant/après, motif, autorisé par) :

* annulation ;
* remise ;
* repas offert ;
* suppression ou désactivation ;
* modification de prix ;
* correction de stock et inventaire ;
* modification de salaire ;
* avance ;
* ouverture / fermeture de caisse et écarts ;
* réouverture d'une addition clôturée ;
* restauration de sauvegarde ;
* changement de date détecté.

Le journal d'audit n'est pas modifiable depuis l'application.

### Données

* Sauvegardes envoyées au cloud chiffrées.
* Chiffrement de la base locale : à évaluer (vol de PC) contre le coût en support et en performance. Donner une recommandation argumentée.

---

## 20. RÉSILIENCE

Le logiciel doit être conçu pour les coupures. Donner des choix concrets, pas seulement des intentions.

* Toutes les opérations importantes sont transactionnelles (section 4).
* Configuration SQLite orientée durabilité (mode WAL, niveau de synchronisation adapté aux écritures financières) : préciser et justifier les réglages.
* **Vérification d'intégrité** au démarrage (rapide) et périodique (complète), avec message clair et procédure de restauration si elle échoue.
* Récupération après arrêt brutal : l'utilisateur retrouve les additions ouvertes, les envois en attente et sa session de caisse.
* Sauvegarde automatique : à la clôture de journée + périodiquement pendant le service, avec rotation (ex. 7 journalières, 4 hebdomadaires, 12 mensuelles).
* Sauvegarde vers un second emplacement : autre disque, clé USB, cloud si activé.
* Sauvegarde manuelle et export USB en un clic.
* Restauration guidée, y compris sur un nouveau PC.
* **Sauvegarde automatique avant chaque mise à jour** du logiciel.
* Alerte si aucune sauvegarde externe n'a réussi depuis N jours.
* Alerte disque presque plein.

Une coupure électrique ne doit jamais corrompre la base de données ni faire perdre un encaissement validé.

---

## 21. INTERFACE

L'interface doit être :

* très simple ;
* rapide (un encaissement standard en moins de 5 secondes) ;
* tactile et utilisable à la souris / au clavier ;
* lisible sur petits écrans (téléphone du serveur) ;
* en français, avec une architecture de traduction prête (bambara envisageable plus tard) ;
* avec montants en FCFA formatés à la française (12 500 FCFA) ;
* avec grands boutons pour la caisse et les commandes ;
* avec photos et couleurs de catégorie pour limiter la lecture ;
* avec peu de fenêtres inutiles ;
* utilisable en plein soleil (bon contraste) sur les terrasses.

Design moderne mais professionnel. Ne pas surcharger l'écran.

Pour les opérations fréquentes :

* peu de clics (prendre une commande de 3 articles : objectif ≤ 6 touches) ;
* raccourcis clavier en caisse ;
* recherche rapide (nom, code, téléphone client) ;
* catégories visuelles ;
* boutons clairement identifiés ;
* confirmations uniquement pour les actions irréversibles.

---

## 22. ARCHITECTURE TECHNIQUE

Proposer une architecture propre et maintenable, qui permet de commencer simplement puis d'évoluer.

### Proposition à évaluer (tu peux la contester, argument à l'appui)

**Un seul cœur applicatif, toujours en client-serveur, même en mono-poste.**

* **Poste central** : application Tauri 2. Le cœur Rust contient toute la logique métier, la base SQLite et un serveur HTTP + WebSocket local.
* **Interface** : React + TypeScript. La même interface est utilisée dans la fenêtre Tauri et servie aux autres postes du réseau via navigateur (tablettes, téléphones des serveurs, écran cuisine), sans installation.
* **Mode A** = poste central avec un seul client (lui-même). **Mode B** = on autorise d'autres clients sur le réseau. Aucun code spécifique à un mode.
* **Base locale** : SQLite dans les deux modes.
* **Cloud** : API séparée, PostgreSQL, authentification, synchronisation. Idéalement, le code métier Rust est partagé entre le poste central et le cloud.

### Point de contradiction à trancher

La v1 de ce cahier des charges prévoyait PostgreSQL pour le mode réseau local. Évalue explicitement :

* SQLite sur le poste central (un seul processus écrit, tous les postes passent par son API) ;
* contre PostgreSQL installé sur le poste central.

Critères : volume réel d'un restaurant (quelques centaines de commandes par jour), coût d'installation et de maintenance chez le client sans informaticien, comportement en coupure, passage mono-poste → réseau, double code de persistance, sauvegarde par simple copie de fichier.

### Contraintes Windows

* Windows 10/11 64 bits minimum (WebView2 n'est plus mis à jour sur Windows 7/8) : le confirmer et l'indiquer dans l'offre commerciale.
* Installateur **entièrement hors ligne**, WebView2 inclus.
* Démarrage automatique du poste central avec Windows ; exception pare-feu configurée à l'installation.
* Fonctionnement correct sur 4 Go de RAM.

### Ne pas introduire inutilement

* microservices ;
* Kubernetes ;
* architecture distribuée complexe ;
* réplication multi-maître ;
* dépendances cloud obligatoires.

---

## 23. STRATÉGIE OFFLINE ET SYNCHRONISATION

Décrire précisément, en partant de ces principes :

* **Un seul maître par restaurant** : le poste central. Pas de réplication multi-maître en local.
* Chaque enregistrement porte : identifiant UUIDv7/ULID, date de création, date de modification, poste d'origine, numéro de version.
* Une table **outbox** accumule les changements à envoyer au cloud ; l'envoi est idempotent et reprend après coupure.
* Les données financières étant en ajout seul, elles se synchronisent sans conflit.
* Données de référence modifiables à distance (prix, menu) : le cloud envoie une **demande de modification** que le poste central applique. Préciser la règle en cas de modification concurrente.
* Commandes cloud (QR, en ligne) : reçues dans une boîte de réception, acceptées ou refusées localement.
* Multi-établissements (un propriétaire, plusieurs restaurants) : chaque restaurant reste autonome ; le cloud agrège pour la consultation. V2.

---

## 24. LICENCE, MISES À JOUR ET SUPPORT

Le produit est vendu. Prévoir dès la conception :

### Licence

* Fichier de licence signé (clé publique embarquée), lié à l'installation.
* **Activation hors ligne** : code machine → code d'activation transmis par téléphone ou WhatsApp.
* Activation des modules optionnels (réseau, QR, livraison, cloud) par la licence.
* Procédure de transfert en cas de changement de PC.
* Rappel : une licence expirée ne bloque jamais les ventes.

### Mises à jour

* Paquet de mise à jour signé, installable en ligne **ou par clé USB**.
* Migrations de base versionnées, en avant uniquement ; retour arrière par restauration de la sauvegarde prise avant la mise à jour.
* En mode réseau, les clients navigateur récupèrent automatiquement la nouvelle interface.

### Support

* Écran « À propos / Diagnostic » : version, taille de base, dernière sauvegarde, dernier contrôle d'intégrité, postes connectés.
* Export d'un paquet de diagnostic (journaux applicatifs + rapport d'intégrité) à envoyer au support.
* Journaux applicatifs en rotation, sans données sensibles.

---

## 25. MATÉRIEL DE RÉFÉRENCE

Proposer une configuration matérielle recommandée à inclure dans l'offre d'installation, par exemple :

* PC ou portable Windows 10/11, SSD (fortement recommandé), 4 à 8 Go de RAM ;
* onduleur pour le poste central et le routeur ;
* imprimante thermique 80 mm ESC/POS (USB ou réseau), une par poste de préparation si pas d'écran ;
* tiroir-caisse piloté par l'imprimante ;
* routeur Wi-Fi dédié au réseau du restaurant (séparé du Wi-Fi clients) ;
* tablettes ou téléphones Android récents avec navigateur Chrome.

Préciser les pilotes et protocoles d'impression supportés, et la gestion de l'ouverture du tiroir.

---

## 26. MODÈLE DE DONNÉES

Avant de coder, concevoir les entités, leurs champs clés, relations, contraintes et index.

Liste de départ (à compléter, fusionner ou scinder en justifiant) :

**Organisation et accès**
Restaurant, Parametres, Appareil, Utilisateur, Role, Permission, Licence

**Personnel**
Employe, Presence, EvenementRH (avance, prime, retenue, consommation), PeriodePaie, BulletinPaie, PaiementSalaire

**Tiers**
Client, Fournisseur, Livreur (employé ou externe)

**Catalogue**
Categorie, Produit, GroupeOptions, Option, PrixZone, PosteProduction, Menu/Combo, Promotion (V2)

**Stock**
ArticleStock, Unite, Conditionnement, Recette, LigneRecette, MouvementStock, Inventaire, LigneInventaire, Consigne (option)

**Salle et ventes**
Zone, Table, JourneeExploitation, Commande (addition), LigneCommande, Envoi (bon de préparation), Paiement, PartPaiement, Remise, Annulation

**Trésorerie**
CompteTresorerie, SessionCaisse, MouvementTresorerie, Billetage, Depense, CategorieDepense

**Achats**
Achat, LigneAchat, ReglementFournisseur

**Crédit**
VenteCredit (ou mouvement du compte client), ReglementClient

**Livraison (V2)**
Livraison, AdresseLivraison, ZoneLivraison, RemiseLivreur

**Cloud (V2)**
QRMenu, CommandeEntrante, Outbox

**Système**
JournalAudit, Sauvegarde, Sequence (numérotation)

Pour chaque entité : champs principaux, type (montants en entier), clés étrangères, contraintes d'unicité, règles de cycle de vie (qui peut créer, modifier, annuler), et si elle est en ajout seul.

Définir clairement les relations et les règles métier avant d'écrire le code.

---

## 27. SCÉNARIOS D'ACCEPTATION

La conception doit permettre de passer ces scénarios. Complète la liste.

1. **Coupure pendant un encaissement mixte.** Au redémarrage, le paiement est soit entièrement enregistré (addition clôturée, caisse et compte Mobile Money à jour), soit pas du tout. Jamais à moitié.
2. **Paiement mixte.** Addition de 12 500 : 5 000 espèces + 7 500 Orange Money. La caisse espèces augmente de 5 000, le compte Orange Money de 7 500, le rapport par moyen de paiement l'indique.
3. **Tournées.** Table 4 : 3 bières, puis 2 bières, puis 2 brochettes. Une seule addition, deux tickets bar et un ticket cuisine.
4. **Après minuit.** Une vente à 1 h 30 apparaît dans la journée d'exploitation de la veille.
5. **Horloge remise à zéro.** Après une coupure, le PC affiche le 01/01/2000. L'application le détecte et bloque les ventes jusqu'à correction par un responsable.
6. **Annulation après envoi.** Un serveur annule un poulet déjà envoyé : motif obligatoire, PIN du gérant, bon d'annulation imprimé en cuisine, ligne dans le journal d'audit et dans le rapport des annulations.
7. **Crédit refusé.** Un client autorisé à 20 000 de crédit, déjà à 18 000, commande 5 000 : refus, sauf autorisation du gérant.
8. **Avance supérieure au salaire.** Le net négatif est reporté sur le mois suivant.
9. **Consommation d'un employé.** Un serveur prend un repas imputé sur son compte : sortie de stock, pas de chiffre d'affaires, retenue en fin de mois.
10. **Réception de boissons.** Achat de 2 casiers de 24 au marché, payé en espèces : +48 bouteilles, dépense de caisse, prix d'achat historisé.
11. **Inventaire du soir.** Stock théorique 30 Coca, compté 27 : écart de 3 validé par le gérant, visible dans le rapport des pertes.
12. **Retour du livreur (V2).** Trois livraisons payées en espèces : la remise livreur transfère le montant en caisse et fait apparaître tout écart.
13. **Poste central éteint en mode réseau.** Les téléphones des serveurs affichent un message clair ; aucune saisie en cours n'est perdue silencieusement ; reprise automatique au retour.
14. **Restauration sur un nouveau PC.** Installation hors ligne, restauration depuis la clé USB, réactivation de la licence, reprise du service en moins de 30 minutes.
15. **Imprimante cuisine en panne.** L'envoi est mis en attente, signalé en caisse et réimprimable ailleurs.

---

## 28. PRIORITÉ MVP ET PHASAGE

La v1 listait 15 modules pour le MVP. C'est beaucoup pour une première mise en service réelle. Propose un phasage réaliste selon les moyens de l'équipe (section 0), à partir de ceci :

### MVP 0 — Pilote vendable (un restaurant réel)

1. utilisateurs, rôles, PIN ;
2. catégories et produits (avec photos, postes de préparation) ;
3. tables, zones et mode comptoir ;
4. commandes, tournées, annulations et remises contrôlées ;
5. tickets cuisine / bar imprimés ;
6. caisse : sessions, paiements mixtes, Mobile Money avec référence, retraits propriétaire ;
7. dépenses ;
8. stock simple des **articles revendus** (boissons) + inventaire ;
9. rapport de clôture et rapport journalier ;
10. sauvegarde / restauration, contrôle d'intégrité ;
11. licence hors ligne.

Architecture client-serveur dès le MVP 0, même si le pilote est en mono-poste.

### MVP 1 — Version commerciale

12. clients et crédit ;
13. fournisseurs, achats et dettes fournisseurs ;
14. employés, présences, avances, paie simple ;
15. mode réseau local (téléphones des serveurs, écran cuisine) ;
16. rapports complets et exports.

### V2

* recettes et consommation théorique ;
* consignes ;
* QR menu (consultation puis commande) ;
* commandes en ligne ;
* livraison et rapprochement livreur ;
* application livreur ;
* suivi GPS ;
* cloud, synchronisation, sauvegarde distante ;
* accès distant et résumé quotidien du propriétaire ;
* rapprochement Mobile Money par relevé ;
* promotions et happy hours ;
* statistiques avancées ;
* multi-établissements.

---

## 29. APPROCHE PRODUIT

Le produit doit être conçu comme un logiciel commercialisable au Mali.

Formes de vente :

* installation mono-PC ;
* installation réseau ;
* installation + formation ;
* maintenance annuelle (mises à jour + support) ;
* sauvegarde cloud optionnelle ;
* module QR ;
* module livraison ;
* accès distant ;
* pack matériel (section 25).

Prévoir dans la conception :

* un **assistant de première configuration** (nom, zones, tables, catégories, produits, moyens de paiement, imprimantes) réalisable en moins d'une heure avec le restaurateur ;
* l'**import des produits** depuis un fichier Excel ;
* une **base de démonstration** pour la formation et la prospection, sans risque de mélange avec les vraies données.

L'objectif est d'avoir un logiciel que le restaurateur utilise réellement tous les jours, et non une démonstration technique.

---

## 30. LIVRABLES ATTENDUS, PAR PHASES

Produis la conception en quatre phases. **Arrête-toi à la fin de chaque phase** et attends ma validation.

### Phase 1 — Vision et architecture

1. vision produit (une page) ;
2. fonctionnalités MVP 0 / MVP 1 / V2 (ajuste le phasage de la section 28 si nécessaire) ;
3. architecture, avec fiches de décision (dont SQLite vs PostgreSQL en local) ;
4. structure du projet (arborescence des dossiers et modules du code) ;
5. stratégie offline / local et résilience ;
6. stratégie de synchronisation ;
7. hypothèses et questions ouvertes.

### Phase 2 — Données et règles

8. modèle de données complet (tables, champs clés, types, relations, contraintes, index) ;
9. règles métier numérotées, par module ;
10. flux métier et diagrammes d'états (commande, envoi, session de caisse, livraison, période de paie).

### Phase 3 — Accès, écrans et interfaces

11. matrice des permissions ;
12. liste des écrans avec, pour les écrans principaux (prise de commande, caisse, cuisine, clôture), une maquette textuelle ;
13. composants d'interface principaux ;
14. API : commandes du poste central, endpoints HTTP, événements WebSocket, format d'erreur.

### Phase 4 — Plan de réalisation

15. plan de développement par étapes, chaque étape produisant quelque chose de **testable en conditions réelles** ;
16. stratégie de tests (règles métier, scénarios de la section 27, tests de coupure) ;
17. risques principaux et parades.

Pour chaque module, explique :

* son objectif ;
* ses principales fonctions ;
* ses dépendances ;
* ses règles métier (numérotées) ;
* les données nécessaires ;
* les écrans nécessaires.

Commence maintenant par la **phase 1 uniquement**.
