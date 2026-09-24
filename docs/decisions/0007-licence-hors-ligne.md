# 0007 — Licence signée Ed25519, activation hors ligne

## Décision
* Le fournisseur signe `{numero, restaurant, code_machine, modules, emise_le, maintenance_jusqua}` avec
  sa clé privée (`youma-licence emettre`). Le poste vérifie avec la clé publique embarquée
  (`YOUMA_CLE_PUBLIQUE` à la compilation).
* Code machine = empreinte de l'identifiant d'installation et du nom du PC : une restauration sur un
  autre PC demande une nouvelle licence (procédure de transfert).
* **Une licence absente, expirée ou invalide ne bloque jamais les ventes** (RG-SYS-06) : seuls les
  modules (réseau, cloud, QR) en dépendent ; une maintenance échue ne coupe que cloud et QR.

## Contradiction signalée
Le cahier prévoit un code d'activation « transmis par téléphone ». Une signature Ed25519 fait 64 octets
(≈ 90 caractères en base64) : impossible à dicter. Le code de licence est donc transmis par WhatsApp,
SMS long ou clé USB. Un code court dictable exigerait un secret partagé dans le logiciel, donc falsifiable.

## Conséquences
La clé de développement (`outils/cle-dev.txt`) est publique : ne jamais l'utiliser en production.
