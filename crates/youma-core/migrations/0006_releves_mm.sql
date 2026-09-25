-- 0006 : rapprochement Mobile Money par relevé d'opérateur (fiche 0016).
CREATE TABLE releves_mm (
    id              TEXT PRIMARY KEY,
    compte_id       TEXT NOT NULL REFERENCES comptes_tresorerie(id),
    nom_fichier     TEXT NOT NULL DEFAULT '',
    debut           INTEGER,            -- première date du relevé (ms), si lisible
    fin             INTEGER,
    importe_le      INTEGER NOT NULL,
    utilisateur_id  TEXT
);

-- Lignes créditrices du relevé, telles que l'opérateur les donne. Une référence n'est importée qu'une fois par compte.
CREATE TABLE lignes_releve_mm (
    id              TEXT PRIMARY KEY,
    releve_id       TEXT NOT NULL REFERENCES releves_mm(id),
    compte_id       TEXT NOT NULL REFERENCES comptes_tresorerie(id),
    reference       TEXT NOT NULL,
    montant         INTEGER NOT NULL,
    date_operation  INTEGER,
    numero          TEXT NOT NULL DEFAULT ''
);
CREATE UNIQUE INDEX idx_lignes_releve_ref ON lignes_releve_mm(compte_id, reference);
CREATE TRIGGER ajout_seul_releves_u BEFORE UPDATE ON releves_mm BEGIN SELECT RAISE(ABORT, 'AJOUT_SEUL: releves_mm'); END;
CREATE TRIGGER ajout_seul_releves_d BEFORE DELETE ON releves_mm BEGIN SELECT RAISE(ABORT, 'AJOUT_SEUL: releves_mm'); END;
CREATE TRIGGER ajout_seul_lignes_releve_u BEFORE UPDATE ON lignes_releve_mm BEGIN SELECT RAISE(ABORT, 'AJOUT_SEUL: lignes_releve_mm'); END;
CREATE TRIGGER ajout_seul_lignes_releve_d BEFORE DELETE ON lignes_releve_mm BEGIN SELECT RAISE(ABORT, 'AJOUT_SEUL: lignes_releve_mm'); END;
