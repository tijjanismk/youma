# 0003 — SQLite sur le poste central (et non PostgreSQL en réseau local)

## Contexte
La v1 du cahier des charges prévoyait PostgreSQL pour le mode réseau. La v2 demande de trancher.

## Décision
SQLite dans tous les modes, sur le poste central, avec **un seul processus écrivain** (le serveur).
Les autres postes passent par l'API ; la connexion est protégée par un verrou et chaque opération
métier est une transaction `BEGIN IMMEDIATE`.

| Critère | SQLite | PostgreSQL local |
|---|---|---|
| Volume (quelques centaines de commandes/jour) | largement suffisant | surdimensionné |
| Installation chez un client sans informaticien | aucun service à installer | service Windows, mot de passe, mises à jour |
| Coupure de courant | WAL + `synchronous=FULL` : commit durable | robuste, mais service à redémarrer, plus de RAM |
| Passage mono-poste → réseau | rien à migrer | migration de données |
| Code de persistance | un seul | deux (SQLite en mono-poste) |
| Sauvegarde | `VACUUM INTO` = une copie de fichier | `pg_dump`, outils à fournir |
| 4 Go de RAM | quelques Mo | plusieurs centaines de Mo |

## Alternatives écartées
PostgreSQL local (voir tableau). Base embarquée clé-valeur (pas de SQL pour les rapports).

## Conséquences
* Le débit d'écriture est limité par un seul écrivain : sans objet à cette échelle.
* Le cloud (V2) peut utiliser PostgreSQL : il ne reçoit que l'outbox, pas le même schéma d'écriture.
