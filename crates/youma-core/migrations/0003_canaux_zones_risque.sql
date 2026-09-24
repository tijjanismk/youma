-- 0003 : canaux de commande (papier, téléphone, QR en salle, en ligne), file de validation,
-- zones à risque, liste noire de numéros, suivi en direct (fiche 0013).

ALTER TABLE commandes ADD COLUMN canal TEXT NOT NULL DEFAULT 'serveur'
    CHECK (canal IN ('serveur','telephone','qr_table','en_ligne'));
-- NULL : commande saisie par le personnel. Sinon : en_attente → acceptee | refusee.
ALTER TABLE commandes ADD COLUMN validation TEXT CHECK (validation IN ('en_attente','acceptee','refusee'));
ALTER TABLE commandes ADD COLUMN validation_motif TEXT;
-- Commandes à distance : paiement Mobile Money d'avance ou à la livraison.
ALTER TABLE commandes ADD COLUMN paiement_mode TEXT CHECK (paiement_mode IN ('avance','a_la_livraison','sur_place'));
ALTER TABLE commandes ADD COLUMN client_telephone TEXT;
ALTER TABLE commandes ADD COLUMN client_nom_saisi TEXT;
-- QR en salle : table demandée (rattachée à l'acceptation, pour respecter « une addition ouverte par table »).
ALTER TABLE commandes ADD COLUMN table_demandee TEXT REFERENCES tables_salle(id);
-- Paiement d'avance annoncé par le client (vérifié à l'acceptation, RG-CAI-04/05).
ALTER TABLE commandes ADD COLUMN paiement_reference TEXT;
ALTER TABLE commandes ADD COLUMN paiement_operateur TEXT;
-- Validation par un responsable exigée (zone à risque « validation manuelle »).
ALTER TABLE commandes ADD COLUMN validation_responsable INTEGER NOT NULL DEFAULT 0;
-- Position GPS éventuelle de l'adresse de livraison (microdegrés, entiers).
ALTER TABLE commandes ADD COLUMN livraison_lat INTEGER;
ALTER TABLE commandes ADD COLUMN livraison_lon INTEGER;
-- Code de suivi public (page de suivi du client) et identifiant d'origine (idempotence du relais).
ALTER TABLE commandes ADD COLUMN code_suivi TEXT;
ALTER TABLE commandes ADD COLUMN origine_id TEXT;
-- Lien secret du livreur (distinct du code du client) : seul son téléphone envoie des positions.
ALTER TABLE commandes ADD COLUMN code_livreur TEXT;
CREATE UNIQUE INDEX idx_commandes_code_livreur ON commandes(code_livreur) WHERE code_livreur IS NOT NULL;
CREATE UNIQUE INDEX idx_commandes_code_suivi ON commandes(code_suivi) WHERE code_suivi IS NOT NULL;
CREATE UNIQUE INDEX idx_commandes_origine ON commandes(origine_id) WHERE origine_id IS NOT NULL;
CREATE INDEX idx_commandes_validation ON commandes(validation) WHERE validation = 'en_attente';

-- QR imprimé sur chaque table : code secret, pour que seuls les clients présents commandent.
ALTER TABLE tables_salle ADD COLUMN code_qr TEXT;
CREATE UNIQUE INDEX idx_tables_code_qr ON tables_salle(code_qr) WHERE code_qr IS NOT NULL;

-- Zones à risque : lieu (quartier et/ou cercle GPS) × plage horaire × jours → action.
CREATE TABLE zones_risque (
    id          TEXT PRIMARY KEY,
    nom         TEXT NOT NULL,
    quartier    TEXT,
    lat         INTEGER,          -- microdegrés
    lon         INTEGER,
    rayon_m     INTEGER,
    debut_min   INTEGER NOT NULL CHECK (debut_min BETWEEN 0 AND 1439),   -- minutes depuis minuit
    fin_min     INTEGER NOT NULL CHECK (fin_min BETWEEN 0 AND 1440),
    jours       INTEGER NOT NULL DEFAULT 127,  -- bits : lundi = 1 … dimanche = 64
    action      TEXT NOT NULL CHECK (action IN ('bloquer','paiement_avance','validation_manuelle')),
    message     TEXT NOT NULL DEFAULT '',
    actif       INTEGER NOT NULL DEFAULT 1,
    modifie_le  INTEGER NOT NULL,
    CHECK (quartier IS NOT NULL OR (lat IS NOT NULL AND lon IS NOT NULL AND rayon_m > 0))
);

-- Numéros bloqués (faux clients, commandes jamais récupérées).
CREATE TABLE numeros_bloques (
    telephone   TEXT PRIMARY KEY,
    motif       TEXT NOT NULL,
    cree_le     INTEGER NOT NULL,
    cree_par    TEXT
);

-- Position du livreur pendant une course (suivi en direct), ajout seul.
CREATE TABLE positions_livreur (
    id          TEXT PRIMARY KEY,
    commande_id TEXT NOT NULL REFERENCES commandes(id),
    lat         INTEGER NOT NULL,
    lon         INTEGER NOT NULL,
    horodatage  INTEGER NOT NULL
);
CREATE INDEX idx_positions_commande ON positions_livreur(commande_id, horodatage);
CREATE TRIGGER ajout_seul_positions_u BEFORE UPDATE ON positions_livreur BEGIN SELECT RAISE(ABORT, 'AJOUT_SEUL: positions_livreur'); END;

-- Bases existantes : nouvelles permissions.
INSERT OR IGNORE INTO role_permissions(role_id, permission)
    SELECT id, 'commande.valider_entrante' FROM roles WHERE code IN ('proprietaire','administrateur','gerant','caissier');
INSERT OR IGNORE INTO role_permissions(role_id, permission)
    SELECT id, 'zone.outrepasser' FROM roles WHERE code IN ('proprietaire','administrateur','gerant');
