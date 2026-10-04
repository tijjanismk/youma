-- Rupture automatique quand le stock d'un produit revendu tombe à 0 (RG-STK-08, fiche 0050) : on retient qu'elle vient
-- du stock, pour la lever d'elle-même au prochain achat ou inventaire, sans toucher aux ruptures décidées à la main.
ALTER TABLE produits ADD COLUMN rupture_auto INTEGER NOT NULL DEFAULT 0;
