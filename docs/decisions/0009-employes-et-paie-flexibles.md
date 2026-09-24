# 0009 — Employés et paie adaptés aux réalités maliennes

## Contexte
Dans les maquis et petits restaurants, la plupart des employés travaillent sans contrat écrit, ne sont
ni déclarés à l'INPS ni affiliés à l'AMO, sont payés au mois, à la semaine, au jour de présence ou à la
course, reçoivent des avances fréquentes et sont parfois nourris ou logés. Des membres de la famille aident
sans être payés.

## Décision
* Seuls le **nom** et le **mode de rémunération** sont obligatoires (RG-EMP-01). Téléphone, pièce
  d'identité (NINA…), date d'arrivée, contact d'urgence, quartier : facultatifs.
* Type d'engagement : `aucun` par défaut, puis `verbal`, `journalier`, `essai`, `apprentissage`, `stage`,
  `cdd`, `cdi` (RG-EMP-02). Aucun traitement n'est refusé faute de contrat.
* Rémunération : `mensuel`, `hebdomadaire`, `journalier` (jours pointés présents × taux), `tache`
  (courses, services), `aucun` (aide familiale : suivi des avances et repas seulement).
* INPS et AMO : cases facultatives par employé **et** interrupteur global, désactivé par défaut
  (RG-PAI-07). Taux configurables ; part employeur affichée à titre informatif.
* Avantages en nature (logé, nourri) : texte informatif, jamais déduit automatiquement.
* Compte employé en journal : avances, retenues (casse), consommations, paiements partiels ; net négatif
  reporté automatiquement.
* Un employé n'est pas un utilisateur : le plongeur n'a pas de compte.

## Alternatives écartées
Paie « officielle » avec cotisations obligatoires : inapplicable à la majorité des établissements ciblés.

## Conséquences
* Taux INPS/AMO **saisis à la main** par le restaurateur (décision du porteur de projet) : aucun taux
  pré-rempli, et une cotisation ne peut être activée sans son taux.
* Pas de déclaration automatique à l'INPS ni de DAS : hors périmètre.
