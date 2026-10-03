# Générer les licences Youma

Une licence dit quels **modules** un restaurant a payés (réseau, livraison, cloud, QR, plusieurs établissements) et
jusqu'à quand court la **maintenance**. Elle ne bloque jamais la vente : sans licence, la caisse vend, encaisse et
imprime (RG-SYS-06). Signature Ed25519, vérifiée hors ligne par la caisse (fiche 0007).

Deux clés :

- la **clé privée** : elle signe les licences. **Vous seul l'avez.** Jamais dans GitHub, jamais envoyée à un client ;
- la **clé publique** : elle vérifie. Elle est compilée dans la caisse (secret GitHub `YOUMA_CLE_PUBLIQUE`).

## 1. Une seule fois : créer vos clés

Sur votre PC (Rust installé, dépôt cloné) :

```sh
cargo build --release -p youma-licence
./target/release/youma-licence generer-cles --fichier cle-privee-youma.txt
```

- Le fichier `cle-privee-youma.txt` contient la clé privée : **faire deux copies hors du dépôt** (clé USB rangée,
  coffre de mots de passe). La perdre = ne plus pouvoir émettre de licence acceptée par les caisses déjà installées.
  Ne pas la laisser dans le dossier du dépôt.
- La ligne « Clé publique » s'affiche : la copier dans GitHub → **Settings** → **Secrets and variables** →
  **Actions** → **New repository secret**, nom `YOUMA_CLE_PUBLIQUE`.

Les installateurs Windows construits ensuite (onglet Actions, ou étiquette `v…`) acceptent vos licences. Un poste compilé
en production **sans** cette clé est désormais refusé à la compilation (fiche 0042) : la clé de développement
(`outils/cle-dev.txt`) est publique et ne doit jamais servir chez un client.

## 2. Pour chaque restaurant : émettre une licence

1. Le restaurant ouvre **Administration → Licence** sur la caisse et vous envoie le **code de ce PC**
   (`XXXX-XXXX-XXXX-XXXX`, par WhatsApp).
2. Vous émettez la licence :

   ```sh
   ./target/release/youma-licence emettre --cle-privee-fichier /chemin/vers/cle-privee-youma.txt \
     --restaurant "Maquis Le Fromager" --machine XXXX-XXXX-XXXX-XXXX \
     --modules reseau,livraison,cloud,qr --maintenance 2027-10-31 --numero L-0001
   ```

   - `--modules` : parmi `reseau`, `livraison`, `cloud`, `qr`, `multi_etablissement` (rien = caisse seule) ;
   - `--maintenance` : fin de la maintenance (sans elle : illimitée) ; une maintenance échue ne coupe que cloud et QR ;
   - `--numero` : votre numéro de licence (tenez une liste : numéro, restaurant, code PC, modules, date, prix).
3. Le texte affiché (une longue ligne) est la licence : l'envoyer par WhatsApp. Le restaurant la colle dans
   **Administration → Licence → Installer la licence**.

À la place de `--cle-privee-fichier`, la variable `YOUMA_CLE_PRIVEE` marche aussi. Évitez `--cle-privee <clé>` : la clé
resterait dans l'historique du terminal.

## 3. Cas particuliers

- **Changement de PC** (panne, restauration sur un autre PC) : le code du PC change ; émettre une nouvelle licence
  pour le nouveau code (même numéro, noté « transfert » dans votre liste).
- **Renouvellement de la maintenance** ou **nouveau module** : émettre une nouvelle licence pour le même code de PC ;
  elle remplace l'ancienne.
- **Mot de passe d'administration oublié** sans code de secours : le restaurant vous envoie le code de demande
  affiché ; vous répondez avec `youma-licence secours --cle-privee-fichier … --demande XXXX-XXXX-XXXX`.
