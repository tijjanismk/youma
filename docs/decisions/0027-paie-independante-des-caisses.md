# 0027 — La paie est indépendante des caisses

## Contexte
Demande du porteur de projet (26/09/2026) : « la paie doit être indépendante des caisses ». Un salaire sans compte
choisi était pris dans le tiroir de la session de caisse ouverte, et même un salaire payé depuis le coffre restait
rattaché à la session du caissier. Les espèces attendues à la clôture de caisse et l'écart dépendaient donc de la
paie, et un gros salaire pouvait vider le tiroir en plein service.

## Décision
* **RG-PAI-09** : un salaire se paie depuis un **compte choisi** : coffre / propriétaire, banque ou Mobile Money.
  Le tiroir (compte « espèces ») et le compte d'un livreur sont refusés, et le paiement n'est rattaché à **aucune
  session de caisse**.
* Écran Paie → « Payer » : liste « Payé depuis » (coffre proposé en premier) ; sans compte possible, un message
  renvoie vers Administration → Moyens de paiement.
* Même logique pour une **avance payée depuis le coffre** (ou la banque) : elle n'est plus rattachée à la session de
  caisse ouverte ; seule une avance sortie du tiroir de la session y figure.
* Le mouvement reste tracé sur le compte payeur (`paiement_salaire`) et sur le compte de l'employé : soldes toujours
  calculés depuis les mouvements.

## Alternatives écartées
* **Garder le tiroir en option** : c'est justement le mélange caisse / paie que le porteur de projet refuse.
* **Caisse « paie » séparée avec sa propre session** : lourd pour un restaurant ; le coffre joue déjà ce rôle.

## Conséquences
* Le solde du coffre peut devenir négatif si le propriétaire paie de sa poche sans l'avoir alimenté : c'est lisible
  dans Caisse → Comptes et transferts, et se régularise par un transfert.
* **Avances : les deux possibilités** (confirmé par le porteur de projet, 26/09/2026). Dans la fiche de l'employé,
  « Payée depuis » propose le **tiroir de ma caisse** (si une session est ouverte : l'avance compte alors dans la
  clôture de cette caisse) ou un **compte hors caisse** (coffre, banque, Mobile Money : la clôture n'en dépend pas).
  L'écran dit l'effet du choix.
