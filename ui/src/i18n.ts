/**
 * Libellés de l'interface. Architecture prête pour une seconde langue (bambara) :
 * ajouter un dictionnaire et choisir la langue dans les paramètres.
 */
const fr = {
  // Types de commande
  sur_place: "Sur place",
  comptoir: "Comptoir",
  emporter: "À emporter",
  livraison: "Livraison",
  // Moyens de paiement
  especes: "Espèces",
  mobile_money: "Mobile Money",
  virement: "Virement",
  carte: "Carte",
  credit: "Crédit (ardoise)",
  // Statuts
  ouverte: "Ouverte",
  payee: "Payée",
  cloturee: "Clôturée",
  annulee: "Annulée",
  libre: "Libre",
  occupee: "Occupée",
  reservee: "Réservée",
  a_nettoyer: "À nettoyer",
  recu: "Reçu",
  en_preparation: "En préparation",
  pret: "Prêt",
  servi: "Servi",
  probleme: "Problème",
  brouillon: "À envoyer",
  envoyee: "Envoyé",
  prete: "Prêt",
  servie: "Servi",
  a_verifier: "À vérifier",
  verifie: "Vérifié",
  rejete: "Rejeté",
  // Employés
  aucun: "Aucun",
  verbal: "Accord verbal",
  journalier: "Journalier",
  essai: "Essai",
  apprentissage: "Apprentissage",
  stage: "Stage",
  cdd: "CDD écrit",
  cdi: "CDI écrit",
  mensuel: "Mensuel",
  hebdomadaire: "Hebdomadaire",
  tache: "À la tâche / course",
  present: "Présent",
  retard: "Retard",
  absent_justifie: "Absent (justifié)",
  absent_non_justifie: "Absent (non justifié)",
  conge: "Congé",
  repos: "Repos",
  actif: "Actif",
  suspendu: "Suspendu",
  parti: "Parti",
  // Mouvements
  salaire: "Salaire",
  deduction_absence: "Absences déduites",
  prime: "Prime",
  avance: "Avance",
  retenue: "Retenue",
  consommation: "Consommation",
  cotisation_inps: "Cotisation INPS",
  cotisation_amo: "Cotisation AMO",
  paiement: "Paiement",
  regularisation: "Régularisation",
  contrepassation: "Contre-passation",
  vente: "Vente",
  depense: "Dépense",
  retrait_proprietaire: "Retrait propriétaire",
  retrait: "Retrait",
  entree_diverse: "Entrée diverse",
  apport: "Apport",
  achat: "Achat",
  perte: "Perte",
  perime: "Périmé",
  casse: "Casse",
  repas_personnel: "Repas du personnel",
  offert: "Offert",
  consommation_interne: "Consommation interne",
  vol: "Vol / perte constatée",
  inventaire: "Écart d'inventaire",
  retour_fournisseur: "Retour fournisseur",
  annulation_vente: "Annulation",
  consommation_employe: "Consommation employé",
  // Livraison
  nouvelle: "Nouvelle",
  confirmee: "Confirmée",
  assignee: "Assignée",
  en_route: "En route",
  livree: "Livrée",
  echec: "Échec",
  // Canaux de commande et zones à risque (fiche 0013)
  serveur: "Serveur (menu papier)",
  telephone: "Téléphone",
  qr_table: "QR sur la table",
  en_ligne: "En ligne",
  a_la_livraison: "À la livraison",
  bloquer: "Bloquer",
  paiement_avance: "Paiement d'avance obligatoire",
  validation_manuelle: "Accord d'un responsable",
  recue: "Reçue",
  refusee: "Refusée",
  acceptee: "Acceptée",
} as const;

export type Cle = keyof typeof fr;

const dictionnaires: Record<string, Record<string, string>> = { fr };
let langue = "fr";

export function definirLangue(l: string) {
  if (dictionnaires[l]) langue = l;
}

/** Traduit une clé ; une clé inconnue est rendue lisible (« a_verifier » → « a verifier »). */
export function t(cle: string | null | undefined): string {
  if (!cle) return "";
  return dictionnaires[langue][cle] ?? dictionnaires.fr[cle] ?? cle.replace(/_/g, " ");
}
