-- 0002 : session confirmée par mot de passe (RG-AUT-06), contrôle des bons de sortie (RG-SOR-*),
-- opérateurs Mobile Money supplémentaires.

ALTER TABLE sessions ADD COLUMN eleve INTEGER NOT NULL DEFAULT 0;

-- Chaque présentation d'un ticket payé à la sortie (ajout seul).
CREATE TABLE controles_sortie (
    id              TEXT PRIMARY KEY,
    commande_id     TEXT NOT NULL REFERENCES commandes(id),
    horodatage      INTEGER NOT NULL,
    utilisateur_id  TEXT,
    appareil_id     TEXT
);
CREATE INDEX idx_controles_sortie_commande ON controles_sortie(commande_id);
CREATE TRIGGER ajout_seul_sortie_u BEFORE UPDATE ON controles_sortie BEGIN SELECT RAISE(ABORT, 'AJOUT_SEUL: controles_sortie'); END;
CREATE TRIGGER ajout_seul_sortie_d BEFORE DELETE ON controles_sortie BEGIN SELECT RAISE(ABORT, 'AJOUT_SEUL: controles_sortie'); END;

-- Bases existantes : nouvelle permission pour les rôles de service
-- (une base neuve la reçoit de roles_par_defaut, les rôles n'existant pas encore ici).
INSERT OR IGNORE INTO role_permissions(role_id, permission)
    SELECT id, 'sortie.controler' FROM roles WHERE code IN ('proprietaire','administrateur','gerant','caissier','serveur');

-- Bases existantes : Wave et Sama Money (une base neuve les reçoit à l'initialisation).
INSERT INTO comptes_tresorerie(id, nom, type, operateur, ordre, modifie_le)
    SELECT lower(hex(randomblob(16))), 'Wave', 'mobile_money', 'Wave', 3, CAST(strftime('%s','now') AS INTEGER) * 1000
    WHERE EXISTS (SELECT 1 FROM restaurant) AND NOT EXISTS (SELECT 1 FROM comptes_tresorerie WHERE nom = 'Wave');
INSERT INTO comptes_tresorerie(id, nom, type, operateur, ordre, modifie_le)
    SELECT lower(hex(randomblob(16))), 'Sama Money', 'mobile_money', 'Sama', 4, CAST(strftime('%s','now') AS INTEGER) * 1000
    WHERE EXISTS (SELECT 1 FROM restaurant) AND NOT EXISTS (SELECT 1 FROM comptes_tresorerie WHERE nom = 'Sama Money');
