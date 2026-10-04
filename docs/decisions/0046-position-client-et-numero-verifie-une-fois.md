# 0046 — Position du client vue par le restaurant, numéro vérifié une fois par SMS ou WhatsApp

## Contexte

Demande du porteur de projet (04/10/2026) : « la position du client ne doit pas être partagée seulement au livreur ;
la couche inscription du client avec un OTP WhatsApp existe ; on inscrit nos livreurs nous-mêmes ». Réponses
précisées :

- position visible par **le restaurant et le livreur** ;
- code reçu par **plusieurs moyens selon les clients** (SMS ou WhatsApp, au choix) ;
- compte client **minimal par défaut** (numéro vérifié une fois), adresses et historique **plus tard** ;
- livreurs : téléphone + PIN donné par le restaurant (étape suivante, fiche à venir).

Jusqu'ici : la position du client n'était montrée qu'au livreur (page de course) ; le code SMS (RG-CAN-04) était
redemandé à chaque commande, par SMS seulement.

## Décision

1. **Position** : la commande expose `livraison_lat` / `livraison_lon` ; les commandes reçues et la fiche de la
   commande montrent un lien « Position du client » (OpenStreetMap). La case du client devient « Partager ma position
   avec le restaurant et le livreur ».
2. **Choix du canal** : `POST /api/public/verification` accepte `canal` = `sms` (par défaut) ou `whatsapp`. Le menu
   du relais publie `canaux_verification`. WhatsApp passe par l'API WhatsApp Cloud de Meta (modèle
   d'authentification `youma_code`, variables `YOUMA_WHATSAPP_*`). Il n'est proposé que s'il est configuré, sauf
   sur un relais tout en simulation (démonstration), où le code s'affiche à l'écran.
3. **Vérifié une fois** : `POST /api/public/verification/confirmer {telephone, code}` consomme le code et renvoie un
   `jeton_client` (32 caractères). Le relais n'en garde que l'empreinte SHA-256 (`clients_verifies`), liée au numéro,
   valable 180 jours après la dernière commande. La commande en ligne accepte `jeton_client` (même numéro) ou, comme
   avant, `code_verification` (applications déjà installées). Le poste reçoit `telephone_verifie` sans le jeton.
4. **Interface** : le téléphone du client garde `{telephone, jeton}` par relais ; le numéro est prérempli et marqué
   « Numéro vérifié » ; un autre numéro redemande un code. Le code est validé dès ses 4 chiffres tapés. Un jeton
   refusé (expiré, relais réinstallé) est oublié et le client redonne un code.

## Alternatives écartées

- **Compte client avec mot de passe** : un mot de passe de plus pour des clients occasionnels ; le téléphone vérifié
  suffit à la confiance du restaurant.
- **WhatsApp par lien wa.me** (fiche 0028) : le client devrait envoyer lui-même le message ; ne prouve pas la
  possession du numéro côté relais.
- **Fournisseurs tiers d'OTP** (Twilio Verify…) : coût et dépendance de plus ; Orange Mali et Meta couvrent les
  usages maliens.
- **Garder le jeton sur le poste central** : le poste n'est pas joignable d'Internet ; seul le relais vérifie.
- **Adresses enregistrées et historique** : demandés « plus tard » ; le jeton et le numéro suffisent à les rattacher.

## Conséquences

- Moins de SMS et de messages WhatsApp payés : un par client et par relais, puis plus rien tant qu'il commande.
- Un jeton volé sur le téléphone permet de commander avec ce numéro ; le restaurant garde la liste noire (RG-CAN-03)
  et le rappel. Réinstaller le relais (base perdue) redemande un code à tous.
- **[HYPOTHÈSE]** Format de l'API WhatsApp Cloud (v21, modèle d'authentification avec bouton « Copier le code ») à
  confirmer au premier envoi réel ; compte Meta Business et modèle validé nécessaires.
- La position du client est une donnée personnelle de plus montrée au personnel : seulement sur les commandes
  reçues et la fiche de la commande, pas dans les rapports.
