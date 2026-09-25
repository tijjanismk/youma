-- 0004 : recettes et consommation théorique (fiche 0014).
-- Une ligne de recette appartient à un produit (plat de base) ou à une option (supplément frites, œuf…).
-- Quantités entières dans l'unité de base de l'article (g, ml, pièce) : jamais de flottants.
CREATE TABLE recettes (
    id          TEXT PRIMARY KEY,
    produit_id  TEXT REFERENCES produits(id),
    option_id   TEXT REFERENCES options(id),
    article_id  TEXT NOT NULL REFERENCES articles_stock(id),
    quantite    INTEGER NOT NULL CHECK (quantite > 0),
    modifie_le  INTEGER NOT NULL,
    CHECK ((produit_id IS NULL) <> (option_id IS NULL))
);
CREATE INDEX idx_recettes_produit ON recettes(produit_id);
CREATE INDEX idx_recettes_option ON recettes(option_id);
