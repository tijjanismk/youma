# 0040 — PIN d'autorisation bloqué après 5 essais, position du livreur gardée, application mobile reportée

## Contexte

Revue du 30/09/2026 (`AI_CONTEXT/ETUDE-MOBILE.md`) :

- **C1** : le PIN d'autorisation ponctuelle (RG-AUT-03, en-tête `X-Autorisation-Pin`) n'était jamais bloqué. Un
  employé connecté pouvait essayer les 10 000 PIN à 4 chiffres et se faire « autoriser » par le PIN deviné d'un
  responsable. La connexion, elle, se bloque après 5 échecs (RG-AUT-02).
- **L1/L2** : sur une page en `http`, le navigateur refuse la position du client ; le message était le même pour
  toutes les erreurs (« activez la localisation »), même quand le client avait refusé la permission.
- **L4** : quand le livreur verrouille son écran, le navigateur arrête d'envoyer sa position ; le client ne le suit plus.

Réponses du porteur de projet (03/10/2026) :

- le raccourci du navigateur a été essayé par le personnel ;
- les livreurs sont surtout occasionnels, mais le restaurant les connaît ;
- C1 : bloquer celui qui se trompe, pas le responsable ;
- la position sert à **rassurer le client** sans que le restaurant suive chaque commande lui-même.
  **[HYPOTHÈSE]** Compris comme : le client suit le livreur sur sa page de suivi. Il n'y a donc pas besoin
  d'envoyer au restaurant la précision de la position du client (constat L3).

## Décision

1. **RG-AUT-08** : 5 PIN faux en 15 minutes bloquent pendant 5 minutes les autorisations demandées par
   l'utilisateur qui les saisit (`utilisateurs.echecs_autorisation`, `premier_echec_autorisation`,
   `autorisation_bloquee_jusqu_a`, migration 0011). Le responsable visé n'est pas bloqué et peut toujours se
   connecter. Un PIN juste n'efface pas les échecs : sinon il suffirait de glisser son propre PIN tous les quatre
   essais. Le PIN est vérifié dans sa propre transaction (`auth::autoriser_par_pin`, appelée par
   `Db::executer`), pour que l'échec reste compté même quand l'opération est refusée. Le blocage est journalisé
   (`autorisation.bloquee`).
2. **Menu client** : sur une page non sécurisée, la case « Partager ma position » est remplacée par une phrase qui
   demande un point de repère. Si la position échoue, le message dit pourquoi : refus du navigateur (avec la marche
   à suivre pour la réautoriser), GPS coupé, ou recherche trop longue (`ui/src/public/position.ts`).
3. **Page du livreur** : pendant la course, l'écran reste allumé (Screen Wake Lock, repris au retour sur la page)
   et un message rappelle de ne pas verrouiller l'écran. Les erreurs de position sont expliquées comme pour le client.
4. **Application mobile** : pas pour l'instant. Le personnel reste sur le navigateur (option 1 de l'étude). Si
   le pilote montre que le suivi du livreur se coupe encore, on fera une petite application Android pour le
   livreur seul (option 4). Les livreurs étant connus du restaurant, on peut leur envoyer l'APK par WhatsApp.

## Alternatives écartées

- Bloquer l'utilisateur dont le PIN est essayé : un employé pourrait bloquer exprès son responsable.
- Bloquer seulement la session : il suffirait de se reconnecter pour recommencer.
- Remettre les échecs à zéro après un PIN juste : contourné en tapant son propre PIN entre deux essais.
- Application native tout de suite (Capacitor pour le personnel, ou réécriture) : aucun besoin du personnel ne
  l'exige aujourd'hui. Le besoin du livreur est d'abord couvert par l'écran gardé allumé.
- Envoyer au poste la précision de la position du client (L3) : pas demandé, puisque le restaurant ne suit pas
  chaque commande lui-même.

## Conséquences

- Un serveur qui tape 5 PIN faux de suite attend 5 minutes, ou le responsable fait l'action depuis sa propre session.
- Le Wake Lock exige `https` (le relais) ; Chrome Android et Safari iOS récents le gèrent. Sans lui, le livreur voit
  le message « Ne verrouillez pas l'écran ». Une batterie faible peut faire refuser le verrou.
- L3 (précision envoyée au poste pour les zones à risque) reste ouvert, à reprendre si une zone en cercle GPS se
  trompe en pratique.
