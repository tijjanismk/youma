# 0001 — Développer toutes les parties sans attendre la validation de chaque phase

## Contexte
`CLAUDE.md` et la section 30 du cahier des charges demandent de livrer la conception par phases et de
s'arrêter à chaque phase. Le porteur de projet a ensuite demandé explicitement de « développer toutes
les parties », avec des tests et des tests d'interface.

**Contradiction signalée** : les deux consignes s'opposent. La demande la plus récente et la plus
explicite du porteur de projet a été suivie.

## Décision
Livrer d'un bloc une conception condensée (phases 1 à 4, `docs/conception/`) et le code correspondant
(MVP 0, MVP 1, livraison de base), avec les règles numérotées citées dans le code et les tests.

## Alternatives écartées
* S'arrêter après la phase 1 : contraire à la demande explicite.
* Coder sans conception écrite : les règles `RG-*` n'auraient pas de référence commune.

## Conséquences
* Les choix marqués **[HYPOTHÈSE]** et les questions ouvertes restent à valider par le porteur de projet.
* `CLAUDE.md` passe en phase « réalisation » ; les règles non négociables restent inchangées.
