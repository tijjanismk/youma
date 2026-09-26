-- RG-CAI-05 : une référence Mobile Money ne sert qu'une fois, pour toujours.
-- RG-CAI-15 : le numéro d'autorisation d'un TPE (carte) ne se répète pas dans la journée, mais peut revenir
-- des mois plus tard : l'unicité définitive est réservée au Mobile Money (contrôle de la journée dans le code).
DROP INDEX idx_parts_reference_mm;
CREATE UNIQUE INDEX idx_parts_reference_mm ON parts_paiement(compte_id, reference)
    WHERE moyen = 'mobile_money' AND reference IS NOT NULL AND reference <> '' AND montant > 0;
