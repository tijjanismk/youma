# 0004 — Réglages SQLite orientés durabilité

## Contexte
Coupures fréquentes, parfois sans onduleur ; un encaissement validé ne doit jamais être perdu.

## Décision
* `journal_mode=WAL` : lectures pendant les écritures, récupération automatique au redémarrage.
* `synchronous=FULL` : chaque commit est écrit sur disque avant d'être confirmé. Le coût (quelques ms
  par transaction) est négligeable au regard du volume.
* `foreign_keys=ON`, `busy_timeout=5000`.
* Triggers `RAISE(ABORT)` sur les tables financières et de stock : ajout seul garanti par la base.
* `PRAGMA quick_check` au démarrage, `integrity_check` quotidien ou à la demande.
* Sauvegarde par `VACUUM INTO` (copie cohérente pendant le service), rotation 7/4/12, copie externe.

## Alternatives écartées
`synchronous=NORMAL` en WAL : plus rapide, mais peut perdre les dernières transactions lors d'une coupure.

## Conséquences
Testé par `s01_coupure_processus_tue` : le processus est tué en pleine transaction, la base reste
intègre et aucun paiement partiel n'apparaît.
