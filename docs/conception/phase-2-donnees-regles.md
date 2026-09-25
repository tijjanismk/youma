# Phase 2 — Données et règles métier

Le schéma de référence est `crates/youma-core/migrations/0001_initial.sql` (source de vérité).
Ce document en donne la logique et numérote les règles citées dans le code et les tests.

## 1. Conventions du modèle

* Identifiants : UUIDv7 en `TEXT`. Horodatages : millisecondes UTC en `INTEGER`.
* Montants : `INTEGER` FCFA. Quantités de stock : `INTEGER` en unité de base.
* Tables en **ajout seul** (triggers `RAISE(ABORT)` sur `UPDATE`/`DELETE`) :
  `journal_audit`, `mouvements_tresorerie`, `paiements`, `parts_paiement`, `verifications_mm`,
  `mouvements_stock`, `mouvements_client`, `mouvements_fournisseur`, `mouvements_employe`,
  `annulations`, `remises`, `depenses`, `achats`, `lignes_achat`, `bulletins`, `historique_prix`,
  `presences`.
* Soldes toujours calculés : `SUM(montant)` / `SUM(quantite)` sur les mouvements.

## 2. Entités principales (résumé)

| Domaine | Tables |
|---|---|
| Système | `systeme`, `parametres`, `restaurant`, `sequences`, `journal_audit`, `outbox`, `sauvegardes` |
| Accès | `roles`, `role_permissions`, `utilisateurs`, `sessions`, `appareils` |
| Journée | `journees` |
| Catalogue | `postes_preparation`, `categories`, `produits`, `groupes_options`, `options`, `prix_zone`, `historique_prix` |
| Salle | `zones`, `tables_salle` |
| Ventes | `commandes`, `lignes_commande`, `envois`, `annulations`, `remises`, `impressions` |
| Trésorerie | `comptes_tresorerie`, `sessions_caisse`, `billetages`, `mouvements_tresorerie`, `paiements`, `parts_paiement`, `verifications_mm`, `categories_depense`, `depenses` |
| Stock | `articles_stock`, `conditionnements`, `mouvements_stock`, `inventaires`, `lignes_inventaire` |
| Tiers | `clients`, `mouvements_client`, `fournisseurs`, `mouvements_fournisseur`, `achats`, `lignes_achat` |
| Personnel | `employes`, `presences`, `mouvements_employe`, `bulletins` |
| Livraison | colonnes `livraison_*` de `commandes`, compte de trésorerie de type `livreur` |

## 3. Règles métier

### Système (SYS)
* **RG-SYS-01** Toute écriture est refusée si l'horloge du PC est antérieure de plus de 5 min au
  dernier événement enregistré. Seul un détenteur de `horloge.forcer` peut accepter la nouvelle heure (journalisé).
* **RG-SYS-02** Toute opération métier s'exécute dans une seule transaction `BEGIN IMMEDIATE`.
* **RG-SYS-03** Les tables financières et de stock sont en ajout seul (triggers).
* **RG-SYS-04** Les numéros lisibles (commande, reçu, achat) viennent de `sequences`, incrémentées
  dans la transaction : pas de trou, un ticket annulé garde son numéro.
* **RG-SYS-05** Chaque action sensible écrit une ligne dans `journal_audit` (qui, autorisé par, poste, avant/après, motif).
* **RG-SYS-06** Une licence absente ou expirée ne bloque jamais les ventes ; elle ne conditionne que les modules (réseau, cloud…).

### Accès (AUT)
* **RG-AUT-01** PIN de 4 à 6 chiffres, haché (Argon2). Deux utilisateurs actifs ne peuvent pas avoir le même PIN.
* **RG-AUT-02** 5 échecs de PIN consécutifs verrouillent l'utilisateur 5 minutes.
* **RG-AUT-03** Une action exigeant une permission absente renvoie `AUTORISATION_REQUISE` ; elle
  peut être autorisée ponctuellement par le PIN d'un responsable qui détient la permission. Les deux identités sont journalisées.
* **RG-AUT-04** Session expirée après l'inactivité configurée (défaut 15 min en poste de service).
* **RG-AUT-05** Le rôle `proprietaire` ne peut pas perdre ses permissions ni être supprimé.
* **RG-AUT-06** Les permissions d'administration (`utilisateur.gerer`, `parametre.gerer`, `licence.gerer`,
  `sauvegarde.gerer`, `appareil.gerer`) exigent une session confirmée par le **mot de passe personnel**
  (6 caractères au moins) en plus du PIN. Pas d'autorisation ponctuelle par PIN pour elles. Les échecs de
  mot de passe comptent pour le verrouillage (RG-AUT-02). Le propriétaire choisit son mot de passe à l'installation.

### Journée (JOU)
* **RG-JOU-01** Vente, dépense, session de caisse exigent une journée ouverte.
* **RG-JOU-02** Une seule journée ouverte à la fois. Sa date = date locale de l'ouverture décalée
  de l'heure de bascule (ouverte à 1 h 30 avec bascule 6 h → date de la veille), et strictement postérieure à la précédente.
* **RG-JOU-03** Tout est rattaché à la journée ouverte, quelle que soit l'heure (vente à 1 h 30 → journée de la veille).
* **RG-JOU-04** Clôture impossible s'il reste une addition ouverte ou une session de caisse ouverte.

### Catalogue (CAT)
* **RG-CAT-01** Le prix est copié dans la ligne de commande ; un changement de prix n'affecte pas les commandes existantes.
* **RG-CAT-02** Tout changement de prix est historisé (`historique_prix`) et audité.
* **RG-CAT-03** Prix par zone : si la zone de la table a un prix pour le produit, il s'applique ; sinon le prix de base.
* **RG-CAT-04** Groupe d'options : le nombre d'options choisies doit respecter `min`/`max`.
* **RG-CAT-05** Un produit indisponible (rupture du jour) ne peut pas être ajouté à une commande.
* **RG-CAT-06** Un produit « revendu » doit être lié à un article de stock.

### Commandes (CMD)
* **RG-CMD-01** Une table ne peut avoir qu'une addition ouverte ; ajouter des articles = nouvelle tournée sur la même addition.
* **RG-CMD-02** Une ligne non envoyée se modifie ou se supprime librement.
* **RG-CMD-03** L'envoi répartit les lignes brouillon par poste de préparation : un envoi par poste, un ticket par envoi.
* **RG-CMD-04** Annuler une ligne envoyée : motif obligatoire, permission `commande.annuler_envoye`
  (sinon autorisation ponctuelle), bon d'annulation imprimé au poste, audit.
* **RG-CMD-05** Une remise exige un motif et ne peut dépasser le plafond (%) du rôle, sauf autorisation ponctuelle.
* **RG-CMD-06** Un article offert exige un motif et la permission `commande.offrir` ; il sort du stock, sans chiffre d'affaires.
* **RG-CMD-07** Une commande « consommation employé » n'a pas de chiffre d'affaires : son montant est imputé sur le compte de l'employé.
* **RG-CMD-08** Une commande payée n'est plus modifiable ; la correction passe par une contre-passation du paiement (réouverture auditée).
* **RG-CMD-09** Transfert de table : la table cible doit être libre. Fusion : les lignes de la source passent sur la cible, la source est clôturée (annulée, sans montant).
* **RG-CMD-10** Total = Σ lignes non annulées (quantité effective × (prix + options)) − remises, hors lignes offertes.
* **RG-CMD-11** Ordre de paiement : `apres` (table) ou `avant` (comptoir) ; en mode `avant`, l'envoi en préparation n'est possible qu'après paiement complet.
* **RG-CMD-12** Division d'addition : parts égales ou montants libres ; les parts sont arrondies au multiple configuré, l'écart d'arrondi va sur la dernière part.

### Caisse et trésorerie (CAI)
* **RG-CAI-01** Encaisser exige une session de caisse ouverte par l'utilisateur sur le poste.
* **RG-CAI-02** Somme des parts = montant du paiement ; le paiement ne dépasse pas le reste dû (le rendu monnaie ne concerne que les espèces).
* **RG-CAI-03** Chaque part crédite le bon compte : espèces → caisse de la session, Mobile Money → compte de l'opérateur, crédit → compte client.
* **RG-CAI-04** Mobile Money : référence de transaction obligatoire si le paramètre l'exige ; statut initial « à vérifier ».
* **RG-CAI-05** Une référence Mobile Money ne peut pas être utilisée deux fois (anti-fraude par capture réutilisée).
* **RG-CAI-06** La commande passe à `payee` quand le reste dû atteint 0.
* **RG-CAI-07** Annuler un paiement = contre-passation (paiement négatif lié + mouvements opposés), motif et permission `caisse.annuler_paiement`.
* **RG-CAI-08** Ouverture de session : fond compté ; si différent du solde théorique de la caisse, un mouvement d'écart est enregistré.
* **RG-CAI-09** Clôture : écart = compté − théorique ; motif obligatoire si |écart| > seuil ; un mouvement d'écart aligne le solde sur la réalité.
* **RG-CAI-10** Retrait propriétaire ≠ dépense : type distinct, exclu des dépenses et du bénéfice.
* **RG-CAI-11** Un mouvement de caisse ne peut rendre le solde espèces négatif.
* **RG-CAI-12** Transfert entre comptes = deux mouvements liés ; frais éventuels = dépense séparée.
* **RG-CAI-13** Une dépense est catégorisée, liée à un compte et à la journée ; elle crée un mouvement de trésorerie négatif.
* **RG-CAI-14** Chaque paiement conserve les espèces reçues du client et la monnaie rendue (rendu = reçu − part en espèces). Ils figurent sur le ticket, l'écran de reçu, le rapport Z (total reçu, rendu, gardé) et le rapport d'activité. Sans part en espèces, reçu = rendu = 0.

### Relevés Mobile Money (RMM) — fiche 0016
* **RG-RMM-01** Import d'un relevé CSV d'opérateur sur un compte Mobile Money : séparateur et colonnes reconnus par
  leur nom (référence et montant obligatoires, date et payeur facultatifs) ; seules les lignes créditrices sont gardées ;
  montants lus en FCFA entiers (« 12 500 », « 12500,00 »).
* **RG-RMM-02** Même compte, même référence (sans tenir compte des majuscules) et même montant → le paiement passe
  « vérifié » (une vérification ajoutée, jamais modifiée). Montant différent → écart listé, le paiement reste à vérifier.
* **RG-RMM-03** Le bilan liste aussi les lignes du relevé inconnues en caisse (paiement non saisi) et les paiements
  « à vérifier » absents du relevé sur sa période (SMS douteux).
* **RG-RMM-04** Une référence n'est importée qu'une fois par compte : recharger un relevé ne compte rien deux fois.
* **RG-RMM-05** « Relancer le rapprochement » reprend les relevés déjà importés (paiement saisi après l'import).

### Stock (STK)
* **RG-STK-01** Vente d'un produit revendu → sortie de 1 unité × quantité (hors lignes annulées) au moment de l'envoi (ou du paiement en comptoir).
* **RG-STK-02** Annulation d'une ligne envoyée → retour en stock, sauf si « perdu » (préparé puis jeté) est indiqué.
* **RG-STK-03** Réception d'un conditionnement = quantité × contenance (2 casiers de 24 → +48).
* **RG-STK-04** Mouvement hors vente : type et motif obligatoires.
* **RG-STK-05** Inventaire validé par un détenteur de `stock.valider_inventaire` : un mouvement d'écart par article (compté − théorique au moment de la validation).
* **RG-STK-06** Le coût unitaire estimé d'un article = coût de la dernière réception.
* **RG-STK-07** Le stock peut devenir négatif (vente non bloquée) mais apparaît en alerte.

### Recettes (REC) — fiche 0014
* **RG-REC-01** Une recette (facultative) liste des ingrédients avec une quantité **entière** dans l'unité de base de
  l'article (g, ml, pièce) ; un ingrédient n'apparaît qu'une fois ; une option (« Grande », « avec œuf ») peut ajouter
  ses propres ingrédients. Définir une recette passe le produit en suivi « recette » ; la vider le repasse en « aucun ».
* **RG-REC-02** Vente d'un plat avec recette → sortie de chaque ingrédient (plat + options choisies) × quantité, au
  moment de l'envoi ; type « vente », « offert » ou « consommation employé » comme pour un article revendu. Le coût
  d'une unité est copié sur la ligne (bénéfice estimé).
* **RG-REC-03** Annulation après envoi : sans perte, les ingrédients reviennent au prorata (sortie totale ÷ quantité
  vendue × quantité annulée), jamais plus que ce qui est encore dehors ; avec perte, rien ne revient.
* **RG-REC-04** Coût matière = Σ quantité × coût unitaire de l'article ; affiché avec sa part du prix de vente.

### Consignes (CON) — fiche 0015
* **RG-CON-01** Un emballage consigné (bouteille, casier) a une consigne entière ≥ 0 ; il peut être lié aux articles
  dont une unité pleine le contient (bière 65 cl → bouteille 65 cl).
* **RG-CON-02** À la réception d'un achat : emballages reçus (+ détenus, + consigne versée au fournisseur) et vides
  rendus (− détenus, − consigne). La consigne nette (reçus − rendus) × valeur s'ajoute au total de l'achat, payée ou
  due comme la marchandise. On ne rend pas plus que ce qu'on détient ; un achat ne devient jamais négatif.
* **RG-CON-03** Détenus = Σ mouvements d'emballages ; vides = détenus − pleins en stock (articles liés). La vente ne
  change pas les détenus : la bouteille reste au restaurant et devient un vide.
* **RG-CON-04** Casse, perte, bouteille emportée ou rapportée par un client, régularisation : motif obligatoire,
  journal en ajout seul.
* **RG-CON-05** Comptage des vides (ou des emballages détenus, sans article lié) par un détenteur de
  `stock.valider_inventaire` : un mouvement d'écart (compté − attendu).
* **RG-CON-06** Vides rendus hors livraison : la consigne revient en caisse (entrée « remboursement_consigne ») ou
  réduit la dette du fournisseur ; jamais plus que la consigne qu'il détient pour cet emballage.

### Clients et crédit (CLI)
* **RG-CLI-01** Le crédit est désactivé par défaut pour chaque client.
* **RG-CLI-02** Vente à crédit refusée si dette + montant > limite, sauf autorisation ponctuelle (`client.depasser_limite`).
* **RG-CLI-03** Dette = Σ mouvements client ; règlement partiel ou total, par tout moyen, crée un mouvement de trésorerie.
* **RG-CLI-04** Téléphone unique parmi les clients.

### Achats (ACH)
* **RG-ACH-01** Achat sans fournisseur autorisé (marché).
* **RG-ACH-02** Achat comptant → mouvement de trésorerie négatif sur le compte choisi ; achat à crédit → dette fournisseur.
* **RG-ACH-03** Réception → mouvements de stock et mise à jour du coût unitaire (RG-STK-06).
* **RG-ACH-04** Règlement fournisseur ≤ dette.

### Employés (EMP) — réalités maliennes
* **RG-EMP-01** Seuls le nom et le mode de rémunération sont obligatoires. Téléphone, pièce
  d'identité, date d'embauche, contrat, numéros INPS/AMO sont facultatifs.
* **RG-EMP-02** Type de contrat : `aucun` (défaut), `verbal`, `journalier`, `essai`, `apprentissage`,
  `stage`, `cdd`, `cdi`. Aucun traitement n'est refusé faute de contrat écrit.
* **RG-EMP-03** Un employé n'est pas un utilisateur ; le lien est facultatif.
* **RG-EMP-04** Modes de rémunération : `mensuel`, `hebdomadaire`, `journalier` (payé au jour de présence),
  `tache` (par course, plat, service), `aucun` (aide familiale, bénévole : suivi des avances et repas uniquement).
* **RG-EMP-05** Avantages en nature (logé, nourri) : texte libre informatif, jamais déduits automatiquement.
* **RG-EMP-06** Un employé parti garde son historique ; son solde reste consultable et payable.
* **RG-EMP-07** Présence : une saisie par jour et par employé, la dernière saisie fait foi (corrections ajoutées, jamais écrasées).

### Paie (PAI)
* **RG-PAI-01** Le compte employé est un journal : + dû (salaire, prime, tâches), − avances, retenues, consommations, cotisations salarié, paiements.
* **RG-PAI-02** Une avance ne peut dépasser le plafond (montant ou % du salaire de base) sauf autorisation ponctuelle ; elle sort d'un compte de trésorerie.
* **RG-PAI-03** Clôture de paie : calcule le dû de la période (mensuel/hebdo : base − absences non justifiées si activé ; journalier : jours présents × taux),
  ajoute les cotisations si applicables, puis fige un bulletin (ajout seul) avec tous les mouvements depuis le bulletin précédent.
* **RG-PAI-04** Net à payer = solde du compte employé à la clôture. Un net négatif reste dû par l'employé et se reporte naturellement sur la période suivante.
* **RG-PAI-05** Paiement partiel autorisé ; il sort d'un compte de trésorerie et apparaît dans la caisse.
* **RG-PAI-06** Un bulletin clôturé n'est jamais modifié ; toute correction est un mouvement de régularisation qui apparaîtra sur le bulletin suivant.
* **RG-PAI-07** Cotisations INPS/AMO : appliquées seulement si (a) activées dans les paramètres ET (b) l'employé est déclaré/affilié.
  Désactivées par défaut. **Taux saisis à la main** par le restaurateur (aucun taux pré-rempli) ; une cotisation
  ne peut être activée sans taux salarié. La part employeur est affichée à titre informatif, jamais retenue sur le salaire.
* **RG-PAI-08** Déduction d'absence = base ÷ jours ouvrables du mois (paramètre, défaut 26) × absences non justifiées, arrondie à l'entier.

### Livraison (LIV)
* **RG-LIV-01** Une commande livraison porte adresse (quartier, repère, téléphone), frais et livreur.
* **RG-LIV-02** Un paiement espèces encaissé par le livreur va sur le compte « à remettre » du livreur, pas dans la caisse.
* **RG-LIV-03** Remise livreur : transfert du compte livreur vers la caisse ; l'écart (attendu − remis) est enregistré avec motif.
* **RG-LIV-04** Suivi en direct : la position du livreur (microdegrés entiers, ajout seul) n'est acceptée que
  pendant la course (livraison assignée ou en route) et avec le lien secret du livreur, distinct du code de
  suivi du client. Le client ne voit la position que pendant la course.

### Canaux de commande (CAN) — fiche 0013
* **RG-CAN-01** Le menu papier (commande saisie par le serveur) est toujours possible. Les autres canaux sont
  indépendants et facultatifs : téléphone (saisi par le personnel), QR sur la table, en ligne.
* **RG-CAN-02** Une commande QR ou en ligne arrive dans une file « à valider » : rien ne part en cuisine avant
  qu'un membre du personnel (`commande.valider_entrante`) ne l'accepte ; un refus exige un motif, visible du client.
  Les prix sont toujours recalculés par le poste central. Le QR d'une table porte un code secret ; une commande
  QR acceptée rejoint l'addition ouverte de la table (une addition par table). Une commande envoyée deux fois
  par le relais (même identifiant d'origine) n'est créée qu'une fois. Pas de clôture de journée avec des commandes en attente.
* **RG-CAN-03** Liste noire : un numéro bloqué ne peut pas commander en ligne ; saisi par le personnel, il faut
  l'accord d'un responsable (`zone.outrepasser`), journalisé.
* **RG-CAN-04** Vérification du numéro du client : rappel par la caisse (par défaut) ou code SMS envoyé par le
  serveur relais via Orange Mali (simulé tant que les identifiants Orange ne sont pas fournis). En mode SMS, une commande en ligne non vérifiée est refusée ; seul le relais peut l'attester.
* **RG-CAN-05** Paiement des commandes en ligne : Mobile Money d'avance (opérateur et référence obligatoires,
  encaissé à l'acceptation, « à vérifier », référence unique RG-CAI-05) et/ou paiement à la livraison, selon les
  paramètres. L'avance peut être imposée au nouveau client ou au-delà d'un plafond.

### Zones à risque (ZON) — fiche 0013
* **RG-ZON-01** Une zone à risque est un quartier et/ou un cercle GPS (centre, rayon), une plage horaire (qui peut
  passer minuit), des jours de la semaine et une action : bloquer, paiement d'avance obligatoire, accord d'un responsable.
* **RG-ZON-02** Pour une livraison, la zone applicable la plus stricte l'emporte (bloquer > paiement d'avance >
  accord d'un responsable). Une plage qui passe minuit appartient au jour où elle commence. En ligne, le client
  voit un message clair ; aucun blocage ne concerne la vente à emporter ou sur place.
* **RG-ZON-03** Saisie par le personnel dans une zone bloquée ou sous contrôle : accord d'un responsable
  (`zone.outrepasser`), journalisé ; zone « paiement d'avance » : l'addition est payée avant l'envoi.

### Bon de sortie (SOR)
* **RG-SOR-01** Pas de facture séparée : le ticket de caisse d'une addition entièrement payée sert de bon de sortie
  (« TICKET DE CAISSE », « BON DE SORTIE n° », « PAYÉ »). Il est imprimé d'office si une imprimante de caisse est configurée.
* **RG-SOR-02** Le bon porte un code de contrôle de 4 caractères, dérivé de l'installation et de la commande.
  Au contrôle, numéro et code doivent correspondre (un numéro inventé est refusé et journalisé).
* **RG-SOR-03** Chaque présentation d'un bon payé est enregistrée (ajout seul). Une deuxième présentation est
  signalée « déjà présenté » avec l'heure et le contrôleur, et journalisée.

### Rapports (RAP)
* **RG-RAP-01** Chaque indicateur est accompagné de sa formule écrite.
* **RG-RAP-02** CA = Σ totaux des commandes payées de la période (hors consommations employés, hors offerts).
* **RG-RAP-03** Bénéfice estimé = CA − coût d'achat estimé des produits vendus − dépenses − salaires dus de la période. Les retraits propriétaire n'y figurent pas.

## 4. Diagrammes d'états

```
Commande : ouverte ──(reste dû = 0)──► payee ──(fermeture)──► cloturee
              │                           │
              └──(fusion/abandon vide)──► annulee      payee ──(annulation paiement)──► ouverte
Ligne    : brouillon ──envoi──► envoyee ──► en_preparation ──► prete ──► servie   (+ annulée à tout moment, motif si envoyée)
Envoi    : recu ──► en_preparation ──► pret ──► servi   |  probleme  |  annule
Session  : ouverte ──(clôture avec comptage)──► fermee
Journée  : ouverte ──(clôture, aucune addition/session ouverte)──► cloturee
Livraison: nouvelle ► confirmee ► en_preparation ► prete ► assignee ► en_route ► livree | echec | annulee
Paie     : mouvements libres ──(clôture période)──► bulletin figé ──► paiements (partiels) ──► solde reporté
```
