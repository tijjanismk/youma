# 0023 — Terrain : caisses POS, imprimantes thermiques chinoises, téléphones sans certificat

## Contexte
Réponses du porteur de projet (26/09/2026) aux questions ouvertes :
* les restaurants utilisent des **imprimantes thermiques chinoises** (type Xprinter, 58 ou 80 mm, ESC/POS) ;
* le poste central est le plus souvent une **caisse POS tout-en-un** ; elle n'aura **pas d'IP publique fixe** ;
* **installer un certificat sur chaque téléphone n'est pas acceptable** (fiche 0020, hypothèse infirmée) ;
* la liste des quartiers convient **puisqu'on peut la modifier** (fiche 0021).

## Décision
* **Imprimantes chinoises** : chaque ticket commence par `FS .` (annule le mode caractères chinois, actif d'origine
  sur beaucoup de ces imprimantes : sans lui, les accents sortent en idéogrammes), puis la page de code PC858.
  **Largeur du papier réglable** (Administration → Règles → Impression) : 58 mm = 32 caractères, 80 mm = 42 ou 48.
  Les traits de séparation sont faits à cette largeur dès la création du ticket (un ancien ticket en file garde 42).
* **Téléphones en HTTP, sans certificat** : l'appairage et le guide proposent l'adresse HTTP du Wi-Fi du restaurant ;
  l'étape « installer le certificat » disparaît. À la place de l'installation de l'application : le raccourci
  « Ajouter à l'écran d'accueil » du navigateur. Le serveur HTTPS local reste (désactivable par `--port-https 0`)
  mais n'est plus proposé. Sur le relais (HTTPS public), menu client, suivi et espace propriétaire s'installent
  toujours comme une application (fiche 0020).
* **Pas d'IP publique** : aucune n'est nécessaire (le poste appelle le relais, fiche 0013). En revanche l'**adresse
  locale** de la caisse doit rester la même (les téléphones l'enregistrent) : IP fixe réglée sur la caisse
  elle-même ou bail réservé dans la box. C'est le guide d'installation qui le dit.

## Alternatives écartées
* **Certificat public** pour un nom pointant vers l'IP locale (type `plex.direct`) : sans Internet, le nom ne se
  résout plus et les téléphones perdent la caisse ; contraire au fonctionnement hors ligne.
* **Page de code choisie par imprimante** : pas de besoin prouvé tant que PC858 n'a pas été essayée sur le matériel ;
  à ajouter si une imprimante du pack ne la connaît pas.
* **Application Android** (coquille autour de l'interface, HTTP local autorisé) : pour plus tard si le raccourci
  du navigateur ne suffit pas (plein écran, ouverture rapide) ; demande une distribution de l'APK.

## Conséquences
* Sur les téléphones du personnel : pas de mise en cache de l'application (service worker inactif en HTTP) ; elle se
  recharge depuis la caisse, ce qui est rapide sur le Wi-Fi local. La position GPS du livreur n'est possible que
  par le relais (déjà le cas, fiche 0013).
* **[HYPOTHÈSE]** Les caisses POS tournent sous **Windows 10/11 64 bits** (le poste central y est installé comme sur
  un PC). Une caisse **Android** ne peut pas être le poste central : il faudrait alors un petit PC à côté.
* **[HYPOTHÈSE]** Imprimante intégrée à la caisse POS : pilote Windows du fabricant ou « Generic / Text Only »,
  destination `windows:NOM` (fiche 0006, jamais essayée sur matériel).
* **[HYPOTHÈSE]** Les imprimantes chinoises du pack acceptent PC858 (`ESC t 19`) : à vérifier avec un ticket de
  test sur le vrai matériel.
