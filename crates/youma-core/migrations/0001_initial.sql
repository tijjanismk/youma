-- Youma — schéma initial.
-- Montants : INTEGER FCFA. Horodatages : INTEGER ms UTC. Identifiants : TEXT UUIDv7.
-- Tables marquées « ajout seul » : triggers en fin de fichier (RG-SYS-03).

-- ───────────── Système ─────────────
CREATE TABLE systeme (
    cle     TEXT PRIMARY KEY,
    valeur  TEXT NOT NULL
);

CREATE TABLE parametres (
    cle     TEXT PRIMARY KEY,
    valeur  TEXT NOT NULL  -- JSON
);

CREATE TABLE restaurant (
    id          TEXT PRIMARY KEY,
    nom         TEXT NOT NULL,
    adresse     TEXT NOT NULL DEFAULT '',
    telephone   TEXT NOT NULL DEFAULT '',
    ville       TEXT NOT NULL DEFAULT 'Bamako',
    nif         TEXT NOT NULL DEFAULT '',
    pied_ticket TEXT NOT NULL DEFAULT 'Merci de votre visite',
    modifie_le  INTEGER NOT NULL
);

CREATE TABLE sequences (
    nom     TEXT PRIMARY KEY,
    valeur  INTEGER NOT NULL
);

CREATE TABLE journal_audit (
    id           TEXT PRIMARY KEY,
    horodatage   INTEGER NOT NULL,
    utilisateur_id TEXT,
    autorise_par TEXT,
    appareil_id  TEXT,
    action       TEXT NOT NULL,
    entite       TEXT NOT NULL,
    entite_id    TEXT,
    avant        TEXT,
    apres        TEXT,
    motif        TEXT
);
CREATE INDEX idx_audit_horodatage ON journal_audit(horodatage);
CREATE INDEX idx_audit_action ON journal_audit(action);

CREATE TABLE outbox (
    seq         INTEGER PRIMARY KEY AUTOINCREMENT,
    id          TEXT NOT NULL UNIQUE,
    horodatage  INTEGER NOT NULL,
    entite      TEXT NOT NULL,
    entite_id   TEXT NOT NULL,
    operation   TEXT NOT NULL,
    envoye_le   INTEGER
);

CREATE TABLE sauvegardes (
    id          TEXT PRIMARY KEY,
    horodatage  INTEGER NOT NULL,
    chemin      TEXT NOT NULL,
    motif       TEXT NOT NULL,
    taille      INTEGER NOT NULL,
    externe     INTEGER NOT NULL DEFAULT 0,
    reussie     INTEGER NOT NULL,
    erreur      TEXT
);

-- ───────────── Accès ─────────────
CREATE TABLE roles (
    id              TEXT PRIMARY KEY,
    code            TEXT NOT NULL UNIQUE,
    nom             TEXT NOT NULL,
    plafond_remise_pct INTEGER NOT NULL DEFAULT 0,
    systeme         INTEGER NOT NULL DEFAULT 0,
    modifie_le      INTEGER NOT NULL
);

CREATE TABLE role_permissions (
    role_id     TEXT NOT NULL REFERENCES roles(id),
    permission  TEXT NOT NULL,
    PRIMARY KEY (role_id, permission)
);

CREATE TABLE employes (
    id                  TEXT PRIMARY KEY,
    nom                 TEXT NOT NULL,
    surnom              TEXT NOT NULL DEFAULT '',
    telephone           TEXT NOT NULL DEFAULT '',
    fonction            TEXT NOT NULL DEFAULT 'autre',
    date_embauche       TEXT,
    type_remuneration   TEXT NOT NULL CHECK (type_remuneration IN ('mensuel','hebdomadaire','journalier','tache','aucun')),
    montant_base        INTEGER NOT NULL DEFAULT 0 CHECK (montant_base >= 0),
    type_contrat        TEXT NOT NULL DEFAULT 'aucun'
                        CHECK (type_contrat IN ('aucun','verbal','journalier','essai','apprentissage','stage','cdd','cdi')),
    date_fin_contrat    TEXT,
    piece_identite      TEXT NOT NULL DEFAULT '',
    contact_urgence     TEXT NOT NULL DEFAULT '',
    quartier            TEXT NOT NULL DEFAULT '',
    declare_inps        INTEGER NOT NULL DEFAULT 0,
    numero_inps         TEXT NOT NULL DEFAULT '',
    affilie_amo         INTEGER NOT NULL DEFAULT 0,
    numero_amo          TEXT NOT NULL DEFAULT '',
    avantages_nature    TEXT NOT NULL DEFAULT '',
    plafond_avance      INTEGER,
    horaires            TEXT NOT NULL DEFAULT '',
    statut              TEXT NOT NULL DEFAULT 'actif' CHECK (statut IN ('actif','suspendu','parti')),
    date_depart         TEXT,
    notes               TEXT NOT NULL DEFAULT '',
    cree_le             INTEGER NOT NULL,
    modifie_le          INTEGER NOT NULL,
    version             INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE utilisateurs (
    id              TEXT PRIMARY KEY,
    nom             TEXT NOT NULL,
    role_id         TEXT NOT NULL REFERENCES roles(id),
    pin_hash        TEXT NOT NULL,
    mot_de_passe_hash TEXT,
    employe_id      TEXT REFERENCES employes(id),
    actif           INTEGER NOT NULL DEFAULT 1,
    echecs_pin      INTEGER NOT NULL DEFAULT 0,
    verrouille_jusqu_a INTEGER,
    cree_le         INTEGER NOT NULL,
    modifie_le      INTEGER NOT NULL,
    version         INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE appareils (
    id          TEXT PRIMARY KEY,
    nom         TEXT NOT NULL,
    type        TEXT NOT NULL DEFAULT 'navigateur',
    jeton_hash  TEXT NOT NULL UNIQUE,
    actif       INTEGER NOT NULL DEFAULT 1,
    cree_le     INTEGER NOT NULL,
    derniere_vue INTEGER
);

CREATE TABLE sessions (
    jeton_hash      TEXT PRIMARY KEY,
    utilisateur_id  TEXT NOT NULL REFERENCES utilisateurs(id),
    appareil_id     TEXT,
    cree_le         INTEGER NOT NULL,
    derniere_activite INTEGER NOT NULL,
    fermee          INTEGER NOT NULL DEFAULT 0
);

-- ───────────── Journée d'exploitation ─────────────
CREATE TABLE journees (
    id              TEXT PRIMARY KEY,
    date_exploitation TEXT NOT NULL UNIQUE,
    statut          TEXT NOT NULL CHECK (statut IN ('ouverte','cloturee')),
    ouverte_le      INTEGER NOT NULL,
    ouverte_par     TEXT NOT NULL,
    cloturee_le     INTEGER,
    cloturee_par    TEXT
);
CREATE UNIQUE INDEX idx_journee_unique_ouverte ON journees(statut) WHERE statut = 'ouverte';

-- ───────────── Catalogue ─────────────
CREATE TABLE postes_preparation (
    id          TEXT PRIMARY KEY,
    nom         TEXT NOT NULL,
    imprimante  TEXT NOT NULL DEFAULT '',   -- '', 'tcp:IP:PORT', 'fichier:CHEMIN', 'windows:NOM'
    ecran       INTEGER NOT NULL DEFAULT 0,
    actif       INTEGER NOT NULL DEFAULT 1,
    modifie_le  INTEGER NOT NULL
);

CREATE TABLE zones (
    id          TEXT PRIMARY KEY,
    nom         TEXT NOT NULL,
    ordre       INTEGER NOT NULL DEFAULT 0,
    actif       INTEGER NOT NULL DEFAULT 1,
    modifie_le  INTEGER NOT NULL
);

CREATE TABLE tables_salle (
    id          TEXT PRIMARY KEY,
    zone_id     TEXT NOT NULL REFERENCES zones(id),
    nom         TEXT NOT NULL,
    capacite    INTEGER NOT NULL DEFAULT 4,
    a_nettoyer  INTEGER NOT NULL DEFAULT 0,
    reservee    INTEGER NOT NULL DEFAULT 0,
    actif       INTEGER NOT NULL DEFAULT 1,
    modifie_le  INTEGER NOT NULL
);

CREATE TABLE categories (
    id          TEXT PRIMARY KEY,
    nom         TEXT NOT NULL,
    couleur     TEXT NOT NULL DEFAULT '#888888',
    icone       TEXT NOT NULL DEFAULT '',
    ordre       INTEGER NOT NULL DEFAULT 0,
    actif       INTEGER NOT NULL DEFAULT 1,
    modifie_le  INTEGER NOT NULL
);

CREATE TABLE articles_stock (
    id              TEXT PRIMARY KEY,
    nom             TEXT NOT NULL,
    unite           TEXT NOT NULL DEFAULT 'unité',
    seuil_alerte    INTEGER NOT NULL DEFAULT 0,
    cout_unitaire   INTEGER NOT NULL DEFAULT 0,
    famille         TEXT NOT NULL DEFAULT '',
    actif           INTEGER NOT NULL DEFAULT 1,
    modifie_le      INTEGER NOT NULL
);

CREATE TABLE conditionnements (
    id          TEXT PRIMARY KEY,
    article_id  TEXT NOT NULL REFERENCES articles_stock(id),
    nom         TEXT NOT NULL,
    contenance  INTEGER NOT NULL CHECK (contenance > 0),
    actif       INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE produits (
    id              TEXT PRIMARY KEY,
    categorie_id    TEXT NOT NULL REFERENCES categories(id),
    nom             TEXT NOT NULL,
    nom_court       TEXT NOT NULL DEFAULT '',
    description     TEXT NOT NULL DEFAULT '',
    photo           TEXT NOT NULL DEFAULT '',
    prix            INTEGER NOT NULL CHECK (prix >= 0),
    poste_id        TEXT REFERENCES postes_preparation(id),
    disponible      INTEGER NOT NULL DEFAULT 1,
    actif           INTEGER NOT NULL DEFAULT 1,
    code            TEXT NOT NULL DEFAULT '',
    code_barres     TEXT NOT NULL DEFAULT '',
    taux_tva_bp     INTEGER NOT NULL DEFAULT 0,
    suivi_stock     TEXT NOT NULL DEFAULT 'aucun' CHECK (suivi_stock IN ('aucun','revendu','recette')),
    article_stock_id TEXT REFERENCES articles_stock(id),
    prix_achat_estime INTEGER NOT NULL DEFAULT 0,
    ordre           INTEGER NOT NULL DEFAULT 0,
    modifie_le      INTEGER NOT NULL,
    version         INTEGER NOT NULL DEFAULT 1
);
CREATE INDEX idx_produits_categorie ON produits(categorie_id);

CREATE TABLE groupes_options (
    id          TEXT PRIMARY KEY,
    produit_id  TEXT NOT NULL REFERENCES produits(id),
    nom         TEXT NOT NULL,
    min_choix   INTEGER NOT NULL DEFAULT 0,
    max_choix   INTEGER NOT NULL DEFAULT 1,
    ordre       INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE options (
    id          TEXT PRIMARY KEY,
    groupe_id   TEXT NOT NULL REFERENCES groupes_options(id),
    nom         TEXT NOT NULL,
    supplement  INTEGER NOT NULL DEFAULT 0 CHECK (supplement >= 0),
    actif       INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE prix_zone (
    produit_id  TEXT NOT NULL REFERENCES produits(id),
    zone_id     TEXT NOT NULL REFERENCES zones(id),
    prix        INTEGER NOT NULL CHECK (prix >= 0),
    PRIMARY KEY (produit_id, zone_id)
);

CREATE TABLE historique_prix (
    id          TEXT PRIMARY KEY,
    produit_id  TEXT NOT NULL REFERENCES produits(id),
    zone_id     TEXT,
    ancien      INTEGER,
    nouveau     INTEGER NOT NULL,
    horodatage  INTEGER NOT NULL,
    utilisateur_id TEXT
);

-- ───────────── Tiers ─────────────
CREATE TABLE clients (
    id              TEXT PRIMARY KEY,
    nom             TEXT NOT NULL,
    telephone       TEXT,
    adresse         TEXT NOT NULL DEFAULT '',
    reperes         TEXT NOT NULL DEFAULT '',
    credit_autorise INTEGER NOT NULL DEFAULT 0,
    limite_credit   INTEGER NOT NULL DEFAULT 0 CHECK (limite_credit >= 0),
    actif           INTEGER NOT NULL DEFAULT 1,
    cree_le         INTEGER NOT NULL,
    modifie_le      INTEGER NOT NULL,
    version         INTEGER NOT NULL DEFAULT 1
);
CREATE UNIQUE INDEX idx_clients_telephone ON clients(telephone) WHERE telephone IS NOT NULL AND telephone <> '';

CREATE TABLE fournisseurs (
    id          TEXT PRIMARY KEY,
    nom         TEXT NOT NULL,
    telephone   TEXT NOT NULL DEFAULT '',
    notes       TEXT NOT NULL DEFAULT '',
    actif       INTEGER NOT NULL DEFAULT 1,
    cree_le     INTEGER NOT NULL,
    modifie_le  INTEGER NOT NULL
);

-- ───────────── Trésorerie ─────────────
CREATE TABLE comptes_tresorerie (
    id          TEXT PRIMARY KEY,
    nom         TEXT NOT NULL,
    type        TEXT NOT NULL CHECK (type IN ('especes','mobile_money','banque','coffre','livreur')),
    operateur   TEXT NOT NULL DEFAULT '',
    employe_id  TEXT REFERENCES employes(id),
    actif       INTEGER NOT NULL DEFAULT 1,
    ordre       INTEGER NOT NULL DEFAULT 0,
    modifie_le  INTEGER NOT NULL
);

CREATE TABLE sessions_caisse (
    id              TEXT PRIMARY KEY,
    compte_id       TEXT NOT NULL REFERENCES comptes_tresorerie(id),
    journee_id      TEXT NOT NULL REFERENCES journees(id),
    caissier_id     TEXT NOT NULL REFERENCES utilisateurs(id),
    appareil_id     TEXT,
    statut          TEXT NOT NULL CHECK (statut IN ('ouverte','fermee')),
    ouverte_le      INTEGER NOT NULL,
    fond_compte     INTEGER NOT NULL,
    theorique_ouverture INTEGER NOT NULL,
    fermee_le       INTEGER,
    compte_final    INTEGER,
    theorique_cloture INTEGER,
    ecart           INTEGER,
    motif_ecart     TEXT
);
CREATE UNIQUE INDEX idx_session_unique_compte ON sessions_caisse(compte_id) WHERE statut = 'ouverte';

CREATE TABLE billetages (
    id          TEXT PRIMARY KEY,
    session_id  TEXT NOT NULL REFERENCES sessions_caisse(id),
    moment      TEXT NOT NULL CHECK (moment IN ('ouverture','cloture')),
    coupure     INTEGER NOT NULL CHECK (coupure > 0),
    nombre      INTEGER NOT NULL CHECK (nombre >= 0)
);

CREATE TABLE mouvements_tresorerie (
    id              TEXT PRIMARY KEY,
    compte_id       TEXT NOT NULL REFERENCES comptes_tresorerie(id),
    session_id      TEXT REFERENCES sessions_caisse(id),
    journee_id      TEXT REFERENCES journees(id),
    type            TEXT NOT NULL,
    montant         INTEGER NOT NULL,
    reference_type  TEXT,
    reference_id    TEXT,
    libelle         TEXT NOT NULL DEFAULT '',
    contrepasse_id  TEXT REFERENCES mouvements_tresorerie(id),
    horodatage      INTEGER NOT NULL,
    utilisateur_id  TEXT,
    autorise_par    TEXT
);
CREATE INDEX idx_mvt_tres_compte ON mouvements_tresorerie(compte_id);
CREATE INDEX idx_mvt_tres_journee ON mouvements_tresorerie(journee_id);
CREATE INDEX idx_mvt_tres_session ON mouvements_tresorerie(session_id);

CREATE TABLE categories_depense (
    id      TEXT PRIMARY KEY,
    nom     TEXT NOT NULL UNIQUE,
    actif   INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE depenses (
    id              TEXT PRIMARY KEY,
    journee_id      TEXT NOT NULL REFERENCES journees(id),
    categorie_id    TEXT NOT NULL REFERENCES categories_depense(id),
    compte_id       TEXT NOT NULL REFERENCES comptes_tresorerie(id),
    mouvement_id    TEXT NOT NULL REFERENCES mouvements_tresorerie(id),
    montant         INTEGER NOT NULL CHECK (montant > 0),
    beneficiaire    TEXT NOT NULL DEFAULT '',
    libelle         TEXT NOT NULL DEFAULT '',
    justificatif    TEXT NOT NULL DEFAULT '',
    annule_depense_id TEXT REFERENCES depenses(id),
    horodatage      INTEGER NOT NULL,
    utilisateur_id  TEXT
);

-- ───────────── Ventes ─────────────
CREATE TABLE commandes (
    id              TEXT PRIMARY KEY,
    numero          INTEGER NOT NULL UNIQUE,
    journee_id      TEXT NOT NULL REFERENCES journees(id),
    type            TEXT NOT NULL CHECK (type IN ('sur_place','comptoir','emporter','livraison')),
    ordre_paiement  TEXT NOT NULL CHECK (ordre_paiement IN ('avant','apres')),
    table_id        TEXT REFERENCES tables_salle(id),
    zone_id         TEXT REFERENCES zones(id),
    serveur_id      TEXT REFERENCES utilisateurs(id),
    client_id       TEXT REFERENCES clients(id),
    employe_id      TEXT REFERENCES employes(id),  -- consommation personnelle (RG-CMD-07)
    couverts        INTEGER NOT NULL DEFAULT 0,
    note            TEXT NOT NULL DEFAULT '',
    statut          TEXT NOT NULL CHECK (statut IN ('ouverte','payee','cloturee','annulee')),
    livraison_statut TEXT,
    livraison_quartier TEXT,
    livraison_repere TEXT,
    livraison_telephone TEXT,
    livraison_frais INTEGER NOT NULL DEFAULT 0,
    livreur_id      TEXT REFERENCES employes(id),
    cree_le         INTEGER NOT NULL,
    cree_par        TEXT,
    modifie_le      INTEGER NOT NULL,
    payee_le        INTEGER,
    cloturee_le     INTEGER,
    version         INTEGER NOT NULL DEFAULT 1
);
CREATE INDEX idx_commandes_journee ON commandes(journee_id);
CREATE UNIQUE INDEX idx_commande_table_ouverte ON commandes(table_id) WHERE statut = 'ouverte' AND table_id IS NOT NULL;

CREATE TABLE envois (
    id          TEXT PRIMARY KEY,
    commande_id TEXT NOT NULL REFERENCES commandes(id),
    numero      INTEGER NOT NULL,          -- n° de tournée dans l'addition
    poste_id    TEXT REFERENCES postes_preparation(id),
    statut      TEXT NOT NULL CHECK (statut IN ('recu','en_preparation','pret','servi','probleme','annule')),
    message     TEXT NOT NULL DEFAULT '',
    cree_le     INTEGER NOT NULL,
    cree_par    TEXT,
    modifie_le  INTEGER NOT NULL
);
CREATE INDEX idx_envois_commande ON envois(commande_id);

CREATE TABLE lignes_commande (
    id              TEXT PRIMARY KEY,
    commande_id     TEXT NOT NULL REFERENCES commandes(id),
    produit_id      TEXT NOT NULL REFERENCES produits(id),
    libelle         TEXT NOT NULL,
    quantite        INTEGER NOT NULL CHECK (quantite > 0),
    quantite_annulee INTEGER NOT NULL DEFAULT 0,
    prix_unitaire   INTEGER NOT NULL CHECK (prix_unitaire >= 0),
    options_json    TEXT NOT NULL DEFAULT '[]',
    montant_options INTEGER NOT NULL DEFAULT 0,
    commentaire     TEXT NOT NULL DEFAULT '',
    poste_id        TEXT REFERENCES postes_preparation(id),
    envoi_id        TEXT REFERENCES envois(id),
    statut          TEXT NOT NULL CHECK (statut IN ('brouillon','envoyee','en_preparation','prete','servie')),
    offert          INTEGER NOT NULL DEFAULT 0,
    offert_motif    TEXT,
    cout_unitaire   INTEGER NOT NULL DEFAULT 0,
    stock_sorti     INTEGER NOT NULL DEFAULT 0,
    cree_le         INTEGER NOT NULL,
    cree_par        TEXT,
    CHECK (quantite_annulee >= 0 AND quantite_annulee <= quantite)
);
CREATE INDEX idx_lignes_commande ON lignes_commande(commande_id);

CREATE TABLE annulations (
    id              TEXT PRIMARY KEY,
    commande_id     TEXT NOT NULL REFERENCES commandes(id),
    ligne_id        TEXT REFERENCES lignes_commande(id),
    quantite        INTEGER NOT NULL,
    montant         INTEGER NOT NULL,
    apres_envoi     INTEGER NOT NULL,
    perte           INTEGER NOT NULL DEFAULT 0,
    motif           TEXT NOT NULL,
    horodatage      INTEGER NOT NULL,
    utilisateur_id  TEXT,
    autorise_par    TEXT
);

CREATE TABLE remises (
    id              TEXT PRIMARY KEY,
    commande_id     TEXT NOT NULL REFERENCES commandes(id),
    ligne_id        TEXT REFERENCES lignes_commande(id),
    montant         INTEGER NOT NULL,   -- négatif pour annuler une remise
    motif           TEXT NOT NULL,
    horodatage      INTEGER NOT NULL,
    utilisateur_id  TEXT,
    autorise_par    TEXT
);
CREATE INDEX idx_remises_commande ON remises(commande_id);

CREATE TABLE paiements (
    id              TEXT PRIMARY KEY,
    numero          INTEGER NOT NULL UNIQUE,
    commande_id     TEXT REFERENCES commandes(id),
    journee_id      TEXT NOT NULL REFERENCES journees(id),
    session_id      TEXT REFERENCES sessions_caisse(id),
    montant         INTEGER NOT NULL,
    recu            INTEGER NOT NULL DEFAULT 0,   -- espèces remises par le client
    rendu           INTEGER NOT NULL DEFAULT 0,
    annule_paiement_id TEXT REFERENCES paiements(id),
    motif           TEXT,
    horodatage      INTEGER NOT NULL,
    utilisateur_id  TEXT,
    autorise_par    TEXT
);
CREATE INDEX idx_paiements_commande ON paiements(commande_id);
CREATE INDEX idx_paiements_journee ON paiements(journee_id);

CREATE TABLE parts_paiement (
    id              TEXT PRIMARY KEY,
    paiement_id     TEXT NOT NULL REFERENCES paiements(id),
    moyen           TEXT NOT NULL CHECK (moyen IN ('especes','mobile_money','virement','carte','credit')),
    compte_id       TEXT REFERENCES comptes_tresorerie(id),
    client_id       TEXT REFERENCES clients(id),
    montant         INTEGER NOT NULL,
    reference       TEXT,
    numero_payeur   TEXT
);
CREATE INDEX idx_parts_paiement ON parts_paiement(paiement_id);
CREATE UNIQUE INDEX idx_parts_reference_mm ON parts_paiement(compte_id, reference)
    WHERE reference IS NOT NULL AND reference <> '' AND montant > 0;

CREATE TABLE verifications_mm (
    id              TEXT PRIMARY KEY,
    part_id         TEXT NOT NULL REFERENCES parts_paiement(id),
    statut          TEXT NOT NULL CHECK (statut IN ('a_verifier','verifie','rejete')),
    note            TEXT NOT NULL DEFAULT '',
    horodatage      INTEGER NOT NULL,
    utilisateur_id  TEXT
);
CREATE INDEX idx_verif_part ON verifications_mm(part_id);

CREATE TABLE impressions (
    id          TEXT PRIMARY KEY,
    poste_id    TEXT REFERENCES postes_preparation(id),
    destination TEXT NOT NULL,
    type        TEXT NOT NULL,
    reference_id TEXT,
    contenu     TEXT NOT NULL,
    statut      TEXT NOT NULL CHECK (statut IN ('en_attente','imprime','erreur')),
    tentatives  INTEGER NOT NULL DEFAULT 0,
    erreur      TEXT,
    cree_le     INTEGER NOT NULL,
    modifie_le  INTEGER NOT NULL
);
CREATE INDEX idx_impressions_statut ON impressions(statut);

-- ───────────── Stock ─────────────
CREATE TABLE mouvements_stock (
    id              TEXT PRIMARY KEY,
    article_id      TEXT NOT NULL REFERENCES articles_stock(id),
    type            TEXT NOT NULL CHECK (type IN ('achat','vente','annulation_vente','perte','perime','casse',
                        'repas_personnel','offert','consommation_interne','vol','inventaire','regularisation',
                        'retour_fournisseur','consommation_employe')),
    quantite        INTEGER NOT NULL,
    cout_unitaire   INTEGER NOT NULL DEFAULT 0,
    motif           TEXT NOT NULL DEFAULT '',
    reference_type  TEXT,
    reference_id    TEXT,
    journee_id      TEXT REFERENCES journees(id),
    horodatage      INTEGER NOT NULL,
    utilisateur_id  TEXT
);
CREATE INDEX idx_mvt_stock_article ON mouvements_stock(article_id);
CREATE INDEX idx_mvt_stock_journee ON mouvements_stock(journee_id);

CREATE TABLE inventaires (
    id          TEXT PRIMARY KEY,
    journee_id  TEXT REFERENCES journees(id),
    libelle     TEXT NOT NULL,
    statut      TEXT NOT NULL CHECK (statut IN ('en_cours','valide','abandonne')),
    cree_le     INTEGER NOT NULL,
    cree_par    TEXT,
    valide_le   INTEGER,
    valide_par  TEXT
);

CREATE TABLE lignes_inventaire (
    inventaire_id TEXT NOT NULL REFERENCES inventaires(id),
    article_id    TEXT NOT NULL REFERENCES articles_stock(id),
    compte        INTEGER NOT NULL,
    theorique     INTEGER,
    PRIMARY KEY (inventaire_id, article_id)
);

-- ───────────── Crédit client, achats ─────────────
CREATE TABLE mouvements_client (
    id              TEXT PRIMARY KEY,
    client_id       TEXT NOT NULL REFERENCES clients(id),
    type            TEXT NOT NULL CHECK (type IN ('vente_credit','reglement','contrepassation','regularisation')),
    montant         INTEGER NOT NULL,   -- + augmente la dette
    commande_id     TEXT REFERENCES commandes(id),
    paiement_id     TEXT REFERENCES paiements(id),
    mouvement_tresorerie_id TEXT REFERENCES mouvements_tresorerie(id),
    motif           TEXT NOT NULL DEFAULT '',
    horodatage      INTEGER NOT NULL,
    utilisateur_id  TEXT,
    autorise_par    TEXT
);
CREATE INDEX idx_mvt_client ON mouvements_client(client_id);

CREATE TABLE achats (
    id              TEXT PRIMARY KEY,
    numero          INTEGER NOT NULL UNIQUE,
    fournisseur_id  TEXT REFERENCES fournisseurs(id),
    journee_id      TEXT REFERENCES journees(id),
    mode            TEXT NOT NULL CHECK (mode IN ('comptant','credit')),
    compte_id       TEXT REFERENCES comptes_tresorerie(id),
    total           INTEGER NOT NULL CHECK (total >= 0),
    note            TEXT NOT NULL DEFAULT '',
    horodatage      INTEGER NOT NULL,
    utilisateur_id  TEXT
);

CREATE TABLE lignes_achat (
    id              TEXT PRIMARY KEY,
    achat_id        TEXT NOT NULL REFERENCES achats(id),
    article_id      TEXT NOT NULL REFERENCES articles_stock(id),
    conditionnement_id TEXT REFERENCES conditionnements(id),
    quantite_saisie INTEGER NOT NULL CHECK (quantite_saisie > 0),
    quantite_base   INTEGER NOT NULL CHECK (quantite_base > 0),
    prix_total      INTEGER NOT NULL CHECK (prix_total >= 0),
    cout_unitaire   INTEGER NOT NULL
);

CREATE TABLE mouvements_fournisseur (
    id              TEXT PRIMARY KEY,
    fournisseur_id  TEXT NOT NULL REFERENCES fournisseurs(id),
    type            TEXT NOT NULL CHECK (type IN ('achat_credit','reglement','regularisation')),
    montant         INTEGER NOT NULL,   -- + augmente la dette envers le fournisseur
    achat_id        TEXT REFERENCES achats(id),
    mouvement_tresorerie_id TEXT REFERENCES mouvements_tresorerie(id),
    motif           TEXT NOT NULL DEFAULT '',
    horodatage      INTEGER NOT NULL,
    utilisateur_id  TEXT
);
CREATE INDEX idx_mvt_fournisseur ON mouvements_fournisseur(fournisseur_id);

-- ───────────── Personnel ─────────────
CREATE TABLE presences (
    id              TEXT PRIMARY KEY,
    employe_id      TEXT NOT NULL REFERENCES employes(id),
    date            TEXT NOT NULL,   -- AAAA-MM-JJ (journée d'exploitation)
    statut          TEXT NOT NULL CHECK (statut IN ('present','retard','absent_justifie','absent_non_justifie','conge','repos')),
    minutes_retard  INTEGER NOT NULL DEFAULT 0,
    note            TEXT NOT NULL DEFAULT '',
    horodatage      INTEGER NOT NULL,
    utilisateur_id  TEXT
);
CREATE INDEX idx_presences_employe_date ON presences(employe_id, date);

CREATE TABLE mouvements_employe (
    seq             INTEGER PRIMARY KEY AUTOINCREMENT,
    id              TEXT NOT NULL UNIQUE,
    employe_id      TEXT NOT NULL REFERENCES employes(id),
    type            TEXT NOT NULL CHECK (type IN ('salaire','deduction_absence','prime','tache','avance','retenue',
                        'consommation','cotisation_inps','cotisation_amo','paiement','regularisation','contrepassation')),
    montant         INTEGER NOT NULL,  -- + dû à l'employé, − dû par l'employé / versé
    quantite        INTEGER,
    date            TEXT NOT NULL,
    compte_id       TEXT REFERENCES comptes_tresorerie(id),
    mouvement_tresorerie_id TEXT REFERENCES mouvements_tresorerie(id),
    commande_id     TEXT REFERENCES commandes(id),
    bulletin_id     TEXT,
    motif           TEXT NOT NULL DEFAULT '',
    horodatage      INTEGER NOT NULL,
    utilisateur_id  TEXT,
    autorise_par    TEXT
);
CREATE INDEX idx_mvt_employe ON mouvements_employe(employe_id);

CREATE TABLE bulletins (
    id              TEXT PRIMARY KEY,
    numero          INTEGER NOT NULL UNIQUE,
    employe_id      TEXT NOT NULL REFERENCES employes(id),
    debut           TEXT NOT NULL,
    fin             TEXT NOT NULL,
    type_remuneration TEXT NOT NULL,
    type_contrat    TEXT NOT NULL,
    base            INTEGER NOT NULL,
    jours_presents  INTEGER NOT NULL,
    jours_absence_nj INTEGER NOT NULL,
    report_precedent INTEGER NOT NULL,
    total_gains     INTEGER NOT NULL,
    total_retenues  INTEGER NOT NULL,
    cotisations_salarie INTEGER NOT NULL,
    charges_employeur INTEGER NOT NULL,
    deja_paye       INTEGER NOT NULL,
    net_a_payer     INTEGER NOT NULL,
    premier_seq     INTEGER NOT NULL,
    dernier_seq     INTEGER NOT NULL,
    detail_json     TEXT NOT NULL,
    horodatage      INTEGER NOT NULL,
    utilisateur_id  TEXT
);
CREATE INDEX idx_bulletins_employe ON bulletins(employe_id);

-- ───────────── Ajout seul (RG-SYS-03) ─────────────
CREATE TRIGGER ajout_seul_journal_audit_u BEFORE UPDATE ON journal_audit BEGIN SELECT RAISE(ABORT, 'AJOUT_SEUL: journal_audit'); END;
CREATE TRIGGER ajout_seul_journal_audit_d BEFORE DELETE ON journal_audit BEGIN SELECT RAISE(ABORT, 'AJOUT_SEUL: journal_audit'); END;
CREATE TRIGGER ajout_seul_mvt_tres_u BEFORE UPDATE ON mouvements_tresorerie BEGIN SELECT RAISE(ABORT, 'AJOUT_SEUL: mouvements_tresorerie'); END;
CREATE TRIGGER ajout_seul_mvt_tres_d BEFORE DELETE ON mouvements_tresorerie BEGIN SELECT RAISE(ABORT, 'AJOUT_SEUL: mouvements_tresorerie'); END;
CREATE TRIGGER ajout_seul_paiements_u BEFORE UPDATE ON paiements BEGIN SELECT RAISE(ABORT, 'AJOUT_SEUL: paiements'); END;
CREATE TRIGGER ajout_seul_paiements_d BEFORE DELETE ON paiements BEGIN SELECT RAISE(ABORT, 'AJOUT_SEUL: paiements'); END;
CREATE TRIGGER ajout_seul_parts_u BEFORE UPDATE ON parts_paiement BEGIN SELECT RAISE(ABORT, 'AJOUT_SEUL: parts_paiement'); END;
CREATE TRIGGER ajout_seul_parts_d BEFORE DELETE ON parts_paiement BEGIN SELECT RAISE(ABORT, 'AJOUT_SEUL: parts_paiement'); END;
CREATE TRIGGER ajout_seul_verif_u BEFORE UPDATE ON verifications_mm BEGIN SELECT RAISE(ABORT, 'AJOUT_SEUL: verifications_mm'); END;
CREATE TRIGGER ajout_seul_verif_d BEFORE DELETE ON verifications_mm BEGIN SELECT RAISE(ABORT, 'AJOUT_SEUL: verifications_mm'); END;
CREATE TRIGGER ajout_seul_mvt_stock_u BEFORE UPDATE ON mouvements_stock BEGIN SELECT RAISE(ABORT, 'AJOUT_SEUL: mouvements_stock'); END;
CREATE TRIGGER ajout_seul_mvt_stock_d BEFORE DELETE ON mouvements_stock BEGIN SELECT RAISE(ABORT, 'AJOUT_SEUL: mouvements_stock'); END;
CREATE TRIGGER ajout_seul_mvt_client_u BEFORE UPDATE ON mouvements_client BEGIN SELECT RAISE(ABORT, 'AJOUT_SEUL: mouvements_client'); END;
CREATE TRIGGER ajout_seul_mvt_client_d BEFORE DELETE ON mouvements_client BEGIN SELECT RAISE(ABORT, 'AJOUT_SEUL: mouvements_client'); END;
CREATE TRIGGER ajout_seul_mvt_fourn_u BEFORE UPDATE ON mouvements_fournisseur BEGIN SELECT RAISE(ABORT, 'AJOUT_SEUL: mouvements_fournisseur'); END;
CREATE TRIGGER ajout_seul_mvt_fourn_d BEFORE DELETE ON mouvements_fournisseur BEGIN SELECT RAISE(ABORT, 'AJOUT_SEUL: mouvements_fournisseur'); END;
CREATE TRIGGER ajout_seul_mvt_emp_u BEFORE UPDATE ON mouvements_employe BEGIN SELECT RAISE(ABORT, 'AJOUT_SEUL: mouvements_employe'); END;
CREATE TRIGGER ajout_seul_mvt_emp_d BEFORE DELETE ON mouvements_employe BEGIN SELECT RAISE(ABORT, 'AJOUT_SEUL: mouvements_employe'); END;
CREATE TRIGGER ajout_seul_annulations_u BEFORE UPDATE ON annulations BEGIN SELECT RAISE(ABORT, 'AJOUT_SEUL: annulations'); END;
CREATE TRIGGER ajout_seul_annulations_d BEFORE DELETE ON annulations BEGIN SELECT RAISE(ABORT, 'AJOUT_SEUL: annulations'); END;
CREATE TRIGGER ajout_seul_remises_u BEFORE UPDATE ON remises BEGIN SELECT RAISE(ABORT, 'AJOUT_SEUL: remises'); END;
CREATE TRIGGER ajout_seul_remises_d BEFORE DELETE ON remises BEGIN SELECT RAISE(ABORT, 'AJOUT_SEUL: remises'); END;
CREATE TRIGGER ajout_seul_depenses_u BEFORE UPDATE ON depenses BEGIN SELECT RAISE(ABORT, 'AJOUT_SEUL: depenses'); END;
CREATE TRIGGER ajout_seul_depenses_d BEFORE DELETE ON depenses BEGIN SELECT RAISE(ABORT, 'AJOUT_SEUL: depenses'); END;
CREATE TRIGGER ajout_seul_achats_u BEFORE UPDATE ON achats BEGIN SELECT RAISE(ABORT, 'AJOUT_SEUL: achats'); END;
CREATE TRIGGER ajout_seul_achats_d BEFORE DELETE ON achats BEGIN SELECT RAISE(ABORT, 'AJOUT_SEUL: achats'); END;
CREATE TRIGGER ajout_seul_lignes_achat_u BEFORE UPDATE ON lignes_achat BEGIN SELECT RAISE(ABORT, 'AJOUT_SEUL: lignes_achat'); END;
CREATE TRIGGER ajout_seul_lignes_achat_d BEFORE DELETE ON lignes_achat BEGIN SELECT RAISE(ABORT, 'AJOUT_SEUL: lignes_achat'); END;
CREATE TRIGGER ajout_seul_bulletins_u BEFORE UPDATE ON bulletins BEGIN SELECT RAISE(ABORT, 'AJOUT_SEUL: bulletins'); END;
CREATE TRIGGER ajout_seul_bulletins_d BEFORE DELETE ON bulletins BEGIN SELECT RAISE(ABORT, 'AJOUT_SEUL: bulletins'); END;
CREATE TRIGGER ajout_seul_hist_prix_u BEFORE UPDATE ON historique_prix BEGIN SELECT RAISE(ABORT, 'AJOUT_SEUL: historique_prix'); END;
CREATE TRIGGER ajout_seul_hist_prix_d BEFORE DELETE ON historique_prix BEGIN SELECT RAISE(ABORT, 'AJOUT_SEUL: historique_prix'); END;
CREATE TRIGGER ajout_seul_presences_u BEFORE UPDATE ON presences BEGIN SELECT RAISE(ABORT, 'AJOUT_SEUL: presences'); END;
CREATE TRIGGER ajout_seul_presences_d BEFORE DELETE ON presences BEGIN SELECT RAISE(ABORT, 'AJOUT_SEUL: presences'); END;

-- Session de caisse fermée : figée.
CREATE TRIGGER session_fermee_figee BEFORE UPDATE ON sessions_caisse
    WHEN OLD.statut = 'fermee' BEGIN SELECT RAISE(ABORT, 'AJOUT_SEUL: session de caisse fermée'); END;
CREATE TRIGGER session_caisse_d BEFORE DELETE ON sessions_caisse BEGIN SELECT RAISE(ABORT, 'AJOUT_SEUL: sessions_caisse'); END;
-- Lignes envoyées : jamais supprimées (RG-CMD-02/04).
CREATE TRIGGER ligne_envoyee_d BEFORE DELETE ON lignes_commande
    WHEN OLD.statut <> 'brouillon' BEGIN SELECT RAISE(ABORT, 'AJOUT_SEUL: ligne envoyée'); END;
CREATE TRIGGER commandes_d BEFORE DELETE ON commandes BEGIN SELECT RAISE(ABORT, 'AJOUT_SEUL: commandes'); END;
CREATE TRIGGER journees_d BEFORE DELETE ON journees BEGIN SELECT RAISE(ABORT, 'AJOUT_SEUL: journees'); END;
