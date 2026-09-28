-- Suivi du client (fiche 0036) : une commande QR ajoutée à l'addition déjà ouverte d'une table garde ses lignes
-- reconnaissables, pour que son suivi avance avec la cuisine.
ALTER TABLE lignes_commande ADD COLUMN origine_commande_id TEXT REFERENCES commandes(id);
CREATE INDEX idx_lignes_origine ON lignes_commande(origine_commande_id) WHERE origine_commande_id IS NOT NULL;

-- Menu du jour (RG-CAT-07) : un produit « selon le jour » n'est proposé que s'il est coché pour la journée.
ALTER TABLE produits ADD COLUMN selon_jour INTEGER NOT NULL DEFAULT 0;
CREATE TABLE menu_du_jour (
    journee_id  TEXT NOT NULL REFERENCES journees(id),
    produit_id  TEXT NOT NULL REFERENCES produits(id),
    PRIMARY KEY (journee_id, produit_id)
);

-- Modification par le client avant acceptation (RG-CAN-06) : nombre de modifications déjà faites.
ALTER TABLE commandes ADD COLUMN modifications_client INTEGER NOT NULL DEFAULT 0;
