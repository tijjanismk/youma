-- Avis des clients (RG-AVI-01 à 03, fiche 0043) : une note de 1 à 5 et un commentaire par commande terminée, donnés
-- depuis la page de suivi (poste ou relais). L'avis lui-même ne se modifie ni ne s'efface ; seule sa suite (avis
-- faible traité par le gérant) s'ajoute.
CREATE TABLE avis (
    id              TEXT PRIMARY KEY,
    commande_id     TEXT NOT NULL UNIQUE REFERENCES commandes(id),
    note            INTEGER NOT NULL CHECK (note BETWEEN 1 AND 5),
    commentaire     TEXT NOT NULL DEFAULT '',
    recu_le         INTEGER NOT NULL,
    traite_le       INTEGER,
    traite_par      TEXT REFERENCES utilisateurs(id),
    suite           TEXT
);
CREATE INDEX idx_avis_recu ON avis(recu_le);

CREATE TRIGGER avis_fige BEFORE UPDATE OF commande_id, note, commentaire, recu_le ON avis
BEGIN SELECT RAISE(ABORT, 'AJOUT_SEUL: avis'); END;
CREATE TRIGGER avis_suite_unique BEFORE UPDATE OF traite_le, traite_par, suite ON avis WHEN OLD.traite_le IS NOT NULL
BEGIN SELECT RAISE(ABORT, 'AJOUT_SEUL: avis'); END;
CREATE TRIGGER avis_d BEFORE DELETE ON avis BEGIN SELECT RAISE(ABORT, 'AJOUT_SEUL: avis'); END;
