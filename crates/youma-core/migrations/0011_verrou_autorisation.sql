-- RG-AUT-08 (fiche 0040) : PIN faux saisis pour une autorisation ponctuelle (RG-AUT-03), comptés pour l'utilisateur
-- qui demande l'autorisation (pas pour le responsable dont on essaie le PIN, qu'on pourrait sinon bloquer exprès),
-- sur une fenêtre glissante qui part du premier échec.
ALTER TABLE utilisateurs ADD COLUMN echecs_autorisation INTEGER NOT NULL DEFAULT 0;
ALTER TABLE utilisateurs ADD COLUMN premier_echec_autorisation INTEGER;
ALTER TABLE utilisateurs ADD COLUMN autorisation_bloquee_jusqu_a INTEGER;
