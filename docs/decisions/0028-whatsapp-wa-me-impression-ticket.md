# 0028 — WhatsApp par liens wa.me ; ticket seul à l'impression du navigateur

## Contexte
Le porteur de projet (26/09/2026) veut commencer WhatsApp avec les liens **wa.me**, sans exclure l'API WhatsApp plus
tard. La fiche 0018 avait écarté WhatsApp faute de compte WhatsApp Business et de fournisseur agréé.
Il signale aussi qu'à l'impression par le navigateur (Win+P, Ctrl+P), le ticket sort avec toute la page autour
(recherche de produits, commande en cours, fenêtre), et qu'après paiement ce n'est même pas le vrai ticket qui sort.

## Décision
* **WhatsApp par wa.me** (`ui/src/whatsapp.ts`) : un lien ouvre WhatsApp sur l'appareil, message prêt, et la personne
  appuie sur « Envoyer ». Numéro malien mis au format international (8 chiffres → 223…) ; sans numéro, WhatsApp
  demande le destinataire. Utilisé pour :
  - le **ticket / addition** au client (addition et écran de reçu), en police à chasse fixe (```…```) pour garder
    les colonnes, au téléphone de livraison ou du client s'il est connu ;
  - le **lien de suivi** d'une livraison, au téléphone de livraison (la route des liens renvoie ce téléphone) ;
  - le **résumé de la journée** au propriétaire (écran « Ma journée », `GET /api/journee/resume`, même texte que le
    SMS de la fiche 0018, numéro du propriétaire des réglages Cloud).
* **Les textes viennent du poste central** (ticket, résumé) : une future API WhatsApp (WhatsApp Business Cloud API,
  appelée par le relais) enverra les mêmes messages sans changer leur contenu.
* **Impression navigateur du ticket** : le ticket est rendu dans une zone placée directement sous `<body>`
  (`composants/Ticket.tsx`, `.zone-ticket`), invisible à l'écran ; à l'impression, elle est seule imprimée. Police à
  chasse fixe réglée sur la plus longue ligne pour tenir dans 72 mm (rouleau de 80 mm) sans coupure. Après paiement,
  c'est le vrai ticket de caisse (bon de sortie) qui sort, plus la carte de l'écran.

## Alternatives écartées
* **API WhatsApp dès maintenant** : compte WhatsApp Business vérifié, modèles de messages validés, fournisseur et coût
  par conversation ; à reprendre quand l'usage le justifie.
* **Masquer l'écran par `visibility: hidden`** : la page cachée garde sa hauteur et imprime des pages blanches.

## Conséquences
L'envoi WhatsApp demande Internet sur l'appareil qui ouvre le lien (téléphone ou PC avec WhatsApp) ; la vente, elle,
n'en dépend pas. Sur une imprimante A4, le ticket sort en haut à gauche de la page, à la largeur d'un ticket.
