-- youma:cles-etrangeres-coupees
-- Fiche 0039 : fidélité par points, cartes cadeaux et bons d'avoir, contrats société, clients privilégiés (VIP).
-- La table parts_paiement est reconstruite (procédure SQLite en 12 étapes) pour accepter le moyen « carte_cadeau »
-- et rattacher une part au contrat société qui la paie : le lanceur coupe les clés étrangères le temps de la
-- migration puis les vérifie (PRAGMA foreign_key_check).

-- ───────────── Contrats société (RG-SOC-01 à 04) ─────────────
CREATE TABLE contrats_societe (
    id              TEXT PRIMARY KEY,
    client_id       TEXT NOT NULL REFERENCES clients(id),   -- la société, cliente à crédit : sa part va sur son compte
    nom             TEXT NOT NULL,
    type_prise      TEXT NOT NULL CHECK (type_prise IN ('pourcentage','montant')),
    valeur          INTEGER NOT NULL CHECK (valeur > 0),     -- % (1 à 100) ou FCFA par repas
    plafond_repas   INTEGER NOT NULL DEFAULT 0 CHECK (plafond_repas >= 0),  -- 0 = pas de plafond
    actif           INTEGER NOT NULL DEFAULT 1,
    cree_le         INTEGER NOT NULL,
    modifie_le      INTEGER NOT NULL
);

-- ───────────── parts_paiement reconstruite ─────────────
CREATE TABLE parts_paiement_nouvelle (
    id              TEXT PRIMARY KEY,
    paiement_id     TEXT NOT NULL REFERENCES paiements(id),
    moyen           TEXT NOT NULL CHECK (moyen IN ('especes','mobile_money','virement','carte','credit','carte_cadeau')),
    compte_id       TEXT REFERENCES comptes_tresorerie(id),
    client_id       TEXT REFERENCES clients(id),
    montant         INTEGER NOT NULL,
    reference       TEXT,
    numero_payeur   TEXT,
    contrat_id      TEXT REFERENCES contrats_societe(id)
);
INSERT INTO parts_paiement_nouvelle(id, paiement_id, moyen, compte_id, client_id, montant, reference, numero_payeur)
    SELECT id, paiement_id, moyen, compte_id, client_id, montant, reference, numero_payeur FROM parts_paiement;
DROP TRIGGER ajout_seul_parts_u;
DROP TRIGGER ajout_seul_parts_d;
DROP TABLE parts_paiement;
ALTER TABLE parts_paiement_nouvelle RENAME TO parts_paiement;
CREATE INDEX idx_parts_paiement ON parts_paiement(paiement_id);
CREATE UNIQUE INDEX idx_parts_reference_mm ON parts_paiement(compte_id, reference)
    WHERE moyen = 'mobile_money' AND reference IS NOT NULL AND reference <> '' AND montant > 0;
CREATE INDEX idx_parts_contrat ON parts_paiement(contrat_id) WHERE contrat_id IS NOT NULL;
CREATE TRIGGER ajout_seul_parts_u BEFORE UPDATE ON parts_paiement BEGIN SELECT RAISE(ABORT, 'AJOUT_SEUL: parts_paiement'); END;
CREATE TRIGGER ajout_seul_parts_d BEFORE DELETE ON parts_paiement BEGIN SELECT RAISE(ABORT, 'AJOUT_SEUL: parts_paiement'); END;

-- ───────────── Fidélité (RG-FID-01 à 05) ─────────────
CREATE TABLE mouvements_fidelite (
    id              TEXT PRIMARY KEY,
    client_id       TEXT NOT NULL REFERENCES clients(id),
    type            TEXT NOT NULL CHECK (type IN ('gain','utilisation','annulation_gain','annulation_utilisation')),
    points          INTEGER NOT NULL,          -- + gagnés, − utilisés
    commande_id     TEXT REFERENCES commandes(id),
    remise_id       TEXT REFERENCES remises(id),
    motif           TEXT NOT NULL DEFAULT '',
    horodatage      INTEGER NOT NULL,
    utilisateur_id  TEXT
);
CREATE INDEX idx_fidelite_client ON mouvements_fidelite(client_id);
CREATE INDEX idx_fidelite_commande ON mouvements_fidelite(commande_id);
CREATE TRIGGER ajout_seul_fidelite_u BEFORE UPDATE ON mouvements_fidelite BEGIN SELECT RAISE(ABORT, 'AJOUT_SEUL: mouvements_fidelite'); END;
CREATE TRIGGER ajout_seul_fidelite_d BEFORE DELETE ON mouvements_fidelite BEGIN SELECT RAISE(ABORT, 'AJOUT_SEUL: mouvements_fidelite'); END;

-- ───────────── Cartes cadeaux et bons d'avoir (RG-CAD-01 à 05) ─────────────
CREATE TABLE cartes_cadeaux (
    id              TEXT PRIMARY KEY,
    code            TEXT NOT NULL UNIQUE,
    type            TEXT NOT NULL CHECK (type IN ('cadeau','avoir')),
    montant_initial INTEGER NOT NULL CHECK (montant_initial > 0),
    client_id       TEXT REFERENCES clients(id),
    beneficiaire    TEXT NOT NULL DEFAULT '',
    motif           TEXT NOT NULL DEFAULT '',
    expire_le       TEXT,                      -- date AAAA-MM-JJ incluse ; NULL = sans fin
    cree_le         INTEGER NOT NULL,
    utilisateur_id  TEXT,
    autorise_par    TEXT
);
CREATE TRIGGER ajout_seul_cartes_u BEFORE UPDATE ON cartes_cadeaux BEGIN SELECT RAISE(ABORT, 'AJOUT_SEUL: cartes_cadeaux'); END;
CREATE TRIGGER ajout_seul_cartes_d BEFORE DELETE ON cartes_cadeaux BEGIN SELECT RAISE(ABORT, 'AJOUT_SEUL: cartes_cadeaux'); END;

CREATE TABLE mouvements_carte (
    id              TEXT PRIMARY KEY,
    carte_id        TEXT NOT NULL REFERENCES cartes_cadeaux(id),
    type            TEXT NOT NULL CHECK (type IN ('emission','utilisation','annulation_utilisation')),
    montant         INTEGER NOT NULL,          -- + crédite la carte, − la débite
    paiement_id     TEXT REFERENCES paiements(id),
    mouvement_tresorerie_id TEXT REFERENCES mouvements_tresorerie(id),
    horodatage      INTEGER NOT NULL,
    utilisateur_id  TEXT
);
CREATE INDEX idx_mvt_carte ON mouvements_carte(carte_id);
CREATE TRIGGER ajout_seul_mvt_carte_u BEFORE UPDATE ON mouvements_carte BEGIN SELECT RAISE(ABORT, 'AJOUT_SEUL: mouvements_carte'); END;
CREATE TRIGGER ajout_seul_mvt_carte_d BEFORE DELETE ON mouvements_carte BEGIN SELECT RAISE(ABORT, 'AJOUT_SEUL: mouvements_carte'); END;

-- ───────────── Clients privilégiés (RG-VIP-01 à 03) ─────────────
ALTER TABLE clients ADD COLUMN vip INTEGER NOT NULL DEFAULT 0;
ALTER TABLE clients ADD COLUMN vip_jusqu_au TEXT;   -- date AAAA-MM-JJ incluse ; NULL = sans fin
