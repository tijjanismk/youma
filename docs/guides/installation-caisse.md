# Installer Youma sur la caisse du restaurant

Pour l'installateur ou le technicien qui met Youma en place. Compter une heure, imprimantes comprises.
La caisse POS devient le **poste central** : elle garde la base, imprime, et les téléphones des serveurs s'y
connectent par le Wi-Fi du restaurant. **Internet n'est pas nécessaire pour vendre.**

## Avant de partir

- L'installateur `Youma_x.y.z_x64-setup.exe` sur une clé USB (onglet Actions du dépôt → « Installateur Windows »,
  ou page de la version). **Jamais un fichier qui finit par `-dev`** chez un client.
- Le nom du restaurant, le nom et le téléphone du propriétaire.
- Une seconde clé USB pour les sauvegardes, qui restera au restaurant.

## 1. Vérifier la caisse

- **Windows 10, 64 bits** (les caisses du pack ; Windows 11 convient aussi) : Paramètres → Système → Informations
  système → « Type du système » doit indiquer **64 bits**. Windows 7 ou 8, ou un Windows 32 bits, ne conviennent pas.
- Windows 10 à jour du composant **WebView2** (présent sur les Windows 10 récents ; sinon l'installateur le
  demande, avec Internet une seule fois).
- L'heure et la date de Windows sont justes (Youma bloque les ventes si l'horloge recule).
- La caisse est branchée sur un onduleur si possible : une coupure brutale pendant une vente est sans danger pour
  la base, mais la caisse doit redémarrer seule.

## 2. Installer

1. Double-cliquer sur l'installateur, accepter la demande d'administrateur, suivre les étapes.
   Tout est dans le fichier : aucune connexion Internet n'est demandée.
2. L'installation :
   - ajoute l'exception « Youma » au pare-feu (réseau privé) pour les téléphones ;
   - lance Youma au démarrage de Windows ;
   - range la base et les sauvegardes dans `C:\ProgramData\Youma` (jamais effacé, même en désinstallant).
3. Youma s'ouvre sur **« Bienvenue dans Youma »** : saisir le nom du restaurant, le nom du propriétaire, son **code
   PIN** (4 à 6 chiffres) et un **mot de passe d'administration** (6 caractères au moins). Les noter pour le
   propriétaire, sur papier, rangé à part.

## 3. Licence

Administration → **Licence** : envoyer le code affiché au fournisseur (téléphone, WhatsApp) ; il renvoie un code de
licence à coller dans « Code de licence ». Une licence expirée **ne bloque jamais les ventes**. Le module
« réseau » de la licence est nécessaire pour connecter des téléphones (étape 6).

## 4. Réglages du restaurant

Administration (mot de passe d'administration demandé) :

- **Restaurant et règles** : ville, adresse, téléphone, pied de ticket ; heure de bascule de la journée ;
  quartiers de livraison et leurs frais ; **largeur du papier** (58 mm = 32 caractères, 80 mm = 42 ou 48).
- **Produits** : catégories, plats, prix, photos (prises avec un téléphone, facultatives).
- **Salle et tables**, **Utilisateurs** (un PIN par employé), **Moyens de paiement** (Orange Money, Moov, Wave…).

## 5. Imprimantes Xprinter

Les imprimantes du pack sont des **Xprinter** (thermiques ESC/POS, 58 ou 80 mm). Youma leur parle directement :
il coupe le « mode chinois » d'origine et passe en page de caractères PC858 pour les accents, coupe le papier et
ouvre le tiroir-caisse branché sur l'imprimante.

**Page d'autotest** (avant tout) : imprimante éteinte, garder le bouton **FEED** appuyé, allumer, relâcher après
le bip. La page indique la largeur du papier, l'interface (USB, Ethernet) et, pour un modèle réseau, son
**adresse IP**. La garder avec le dossier du restaurant.

**Xprinter en USB (caisse ou cuisine près de la caisse)**

1. Brancher, allumer, installer le **pilote Xprinter** (CD livré, ou fichier fourni par le fournisseur du pack).
   À défaut : pilote **« Generic / Text Only »** de Windows.
2. Noter son nom exact : Paramètres → Périphériques → Imprimantes et scanners (par exemple `XP-80C`).
3. Dans Youma, destination : `windows:NOM EXACT` (par exemple `windows:XP-80C`), majuscules et espaces compris.

**Xprinter réseau (câble Ethernet, cuisine ou bar éloignés)**

1. Brancher le câble sur la box, allumer, lire l'adresse sur la page d'autotest.
2. Si cette adresse n'est pas dans le réseau de la box (par exemple l'imprimante dit `192.168.123.100` et la caisse
   est en `192.168.1.x`), la changer avec l'outil de réglage Xprinter fourni avec le pilote (onglet réseau) :
   choisir une adresse fixe hors de la plage distribuée par la box, par exemple `192.168.1.50`.
3. Destination : `tcp:ADRESSE:9100` (par exemple `tcp:192.168.1.50:9100`).

**Largeur du papier** (Administration → Restaurant et règles) : 58 mm → **32** caractères ; 80 mm → **48**
(ou **42** si les lignes débordent).

**Où la déclarer**

- Administration → **Postes et imprimantes** : une imprimante par poste de préparation (cuisine, grill, bar).
- Administration → **Restaurant et règles** : « Imprimante de caisse (tickets clients) » et « Ouvrir le
  tiroir-caisse à l'impression du ticket ».

**Essai** : ouvrir la journée, faire une vente à emporter avec un plat de chaque poste, l'envoyer, puis « Addition /
ticket », **sans encaisser**. Vérifier : les accents (é, è, à, ç) s'impriment en lettres (pas en idéogrammes ni en « ? ») ; les traits
de séparation tiennent sur une ligne (sinon, changer la largeur du papier) ; le papier est coupé ; le tiroir
s'ouvre. Annuler ensuite les lignes de cette commande d'essai (non encaissée : rien n'entre dans la caisse).

## 6. Téléphones des serveurs (mode réseau)

1. **Adresse fixe de la caisse sur le Wi-Fi** : indispensable, les téléphones la retiennent. Soit une réservation
   dans la box (adresse liée à la caisse), soit une adresse fixe dans Windows 10 : Paramètres → Réseau et Internet →
   Wi-Fi (ou Ethernet) → le nom du réseau → Paramètres IP → Modifier → Manuel → IPv4 activé (par exemple `192.168.1.10`, masque `255.255.255.0`, passerelle et
   DNS = adresse de la box). Choisir une adresse hors de la plage que la box distribue.
2. **Réseau « privé »** dans Windows (Windows 10 : Paramètres → Réseau et Internet → Wi-Fi (ou Ethernet) → le nom
   du réseau → Profil réseau → Privé), sinon le pare-feu bloque les téléphones.
3. **Activer le mode réseau** : Administration → **Téléphones et tablettes** → cocher « Mode réseau » (mot de
   passe d'administration demandé), puis fermer et rouvrir Youma, ou redémarrer la caisse.
4. Administration → **Téléphones et tablettes** → « Générer un code » : le téléphone, connecté au Wi-Fi du
   restaurant, scanne le QR code. Puis, dans le menu du navigateur : « Ajouter à l'écran d'accueil ».
   Aucun certificat à installer.

## 7. Sauvegardes

- Youma sauvegarde tout seul pendant que la journée est ouverte, dans `C:\ProgramData\Youma\sauvegardes`.
- Administration → Restaurant et règles → « Second emplacement » : la clé USB qui reste au restaurant (par exemple
  `E:\Youma`). Youma prévient quand elle n'a pas reçu de sauvegarde depuis longtemps.
- Facultatif : sauvegardes chiffrées sur Internet (Administration → Cloud, voir `relais-en-ligne.md`). Faire noter
  la **phrase de chiffrement** sur papier.

## 8. Remise au restaurant

Avec le propriétaire et le caissier, une fois :

1. **Ouvrir la journée** (Accueil) puis **ouvrir la caisse** (fond de caisse compté).
2. Une commande à table depuis un téléphone de serveur, son envoi en cuisine, l'encaissement (espèces et Mobile
   Money).
3. **Clôturer la caisse** (billetage) puis **clôturer la journée** ; imprimer le rapport.
4. Montrer « Changer d'utilisateur » : chacun travaille avec son propre PIN.

## En cas de problème

| Symptôme | À vérifier |
|---|---|
| Les téléphones ne trouvent pas la caisse | Même Wi-Fi ; « Mode réseau » coché et Youma redémarré ; réseau Windows « Privé » ; adresse de la caisse inchangée. |
| « Poste central injoignable » sur un téléphone | La caisse est éteinte ou a changé d'adresse : fixer l'adresse (étape 6) puis refaire le QR code. |
| Accents en idéogrammes ou en « ? » | Refaire la page d'autotest et envoyer la photo au fournisseur avec le modèle exact (page de code à ajuster). |
| Xprinter réseau introuvable | Adresse de l'autotest dans le même réseau que la caisse (`192.168.1.x`) ; câble branché sur la box. |
| Le tiroir ne s'ouvre pas | Tiroir branché sur la prise RJ11 de l'imprimante de caisse (pas sur la caisse) ; case « Ouvrir le tiroir-caisse » cochée. |
| Traits coupés sur deux lignes | Largeur du papier trop grande : passer à 32 (58 mm) ou 42. |
| Rien ne s'imprime | Le ticket est gardé et réessayé ; vérifier papier, câble, nom exact `windows:…` ou adresse `tcp:…`. |
| « L'horloge du PC est antérieure… » | Corriger la date et l'heure de Windows. |
