-- 0005 : consignes (bouteilles, casiers) — fiche 0015.
CREATE TABLE emballages (
    id          TEXT PRIMARY KEY,
    nom         TEXT NOT NULL,
    valeur      INTEGER NOT NULL CHECK (valeur >= 0),   -- consigne d'un emballage, FCFA
    actif       INTEGER NOT NULL DEFAULT 1,
    modifie_le  INTEGER NOT NULL
);

-- Une unité pleine de l'article contient un emballage (bouteille de bière consignée).
ALTER TABLE articles_stock ADD COLUMN emballage_id TEXT REFERENCES emballages(id);

-- Journal des emballages, ajout seul. quantite : effet sur les emballages détenus (pleins + vides) ;
-- montant : effet sur la consigne versée au fournisseur (ce qu'il doit rendre).
CREATE TABLE mouvements_emballages (
    id              TEXT PRIMARY KEY,
    emballage_id    TEXT NOT NULL REFERENCES emballages(id),
    fournisseur_id  TEXT REFERENCES fournisseurs(id),
    type            TEXT NOT NULL CHECK (type IN ('reception','retour','casse','perte','sortie_client','retour_client',
                        'inventaire','regularisation')),
    quantite        INTEGER NOT NULL,
    montant         INTEGER NOT NULL DEFAULT 0,
    motif           TEXT NOT NULL DEFAULT '',
    achat_id        TEXT REFERENCES achats(id),
    mouvement_tresorerie_id TEXT REFERENCES mouvements_tresorerie(id),
    journee_id      TEXT REFERENCES journees(id),
    horodatage      INTEGER NOT NULL,
    utilisateur_id  TEXT
);
CREATE INDEX idx_mvt_emballages ON mouvements_emballages(emballage_id);
CREATE INDEX idx_mvt_emballages_fournisseur ON mouvements_emballages(fournisseur_id);
CREATE TRIGGER ajout_seul_mvt_emballages_u BEFORE UPDATE ON mouvements_emballages BEGIN SELECT RAISE(ABORT, 'AJOUT_SEUL: mouvements_emballages'); END;
CREATE TRIGGER ajout_seul_mvt_emballages_d BEFORE DELETE ON mouvements_emballages BEGIN SELECT RAISE(ABORT, 'AJOUT_SEUL: mouvements_emballages'); END;
