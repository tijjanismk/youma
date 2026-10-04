-- Accès des livreurs à l'application Youma Livreur (RG-LIV-05, fiche 0047) : téléphone de l'employé et PIN donné
-- par le restaurant, gardé haché (Argon2). Le poste publie l'empreinte au relais, qui vérifie le PIN.
ALTER TABLE employes ADD COLUMN pin_livreur_hash TEXT;
