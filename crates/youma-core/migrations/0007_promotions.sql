-- 0007 : promotions et happy hours (fiche 0017).
CREATE TABLE promotions (
    id          TEXT PRIMARY KEY,
    nom         TEXT NOT NULL,
    produit_id  TEXT REFERENCES produits(id),
    categorie_id TEXT REFERENCES categories(id),
    type        TEXT NOT NULL CHECK (type IN ('prix','pourcentage')),
    valeur      INTEGER NOT NULL CHECK (valeur >= 0),   -- prix en FCFA, ou remise en points de base (2 500 = 25 %)
    debut_min   INTEGER NOT NULL CHECK (debut_min BETWEEN 0 AND 1439),
    fin_min     INTEGER NOT NULL CHECK (fin_min BETWEEN 0 AND 1440),
    jours       INTEGER NOT NULL DEFAULT 127,
    date_debut  TEXT,                                    -- AAAA-MM-JJ, facultatif
    date_fin    TEXT,
    actif       INTEGER NOT NULL DEFAULT 1,
    modifie_le  INTEGER NOT NULL,
    CHECK ((produit_id IS NULL) <> (categorie_id IS NULL))
);

-- Prix copié sur la ligne : promotion appliquée et prix normal (manque à gagner du happy hour).
ALTER TABLE lignes_commande ADD COLUMN promotion_id TEXT REFERENCES promotions(id);
ALTER TABLE lignes_commande ADD COLUMN prix_normal INTEGER;
