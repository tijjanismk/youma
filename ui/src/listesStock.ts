/**
 * Listes proposées dans le stock (fiche 0034) : choisir plutôt que taper. « Autre… » reste possible partout.
 * Unités en entiers (règle des montants et quantités entiers) : g et ml pour les ingrédients pesés ou mesurés.
 */

export const UNITES: { nom: string; options: string[] }[] = [
  { nom: "À l'unité", options: ["bouteille", "canette", "pièce", "sachet", "boîte", "portion"] },
  { nom: "Au poids ou au volume", options: ["g", "ml", "kg", "litre"] },
];

export const FAMILLES = [
  "Boissons",
  "Bières",
  "Eaux et jus",
  "Cuisine",
  "Viandes et volailles",
  "Poissons",
  "Légumes et fruits",
  "Céréales et féculents",
  "Épicerie et condiments",
  "Emballages",
  "Gaz et charbon",
  "Entretien",
];

export const CONDITIONNEMENTS = [
  "Casier de 12",
  "Casier de 20",
  "Casier de 24",
  "Carton de 6",
  "Carton de 12",
  "Carton de 24",
  "Pack de 6",
  "Pack de 12",
  "Sac de 1 kg",
  "Sac de 5 kg",
  "Sac de 25 kg",
  "Sac de 50 kg",
  "Bidon de 5 litres",
  "Bidon de 20 litres",
];

/** Contenance déduite du nom et de l'unité de base : « Sac de 25 kg » en g → 25 000 ; « Casier de 24 » → 24. */
export function contenanceSuggeree(nom: string, unite: string): number | null {
  const m = /de\s+(\d+)\s*(kg|litres?|l)?\s*$/i.exec(nom.trim());
  if (!m) return null;
  const n = Number(m[1]);
  const grand = (m[2] ?? "").toLowerCase();
  if (grand === "kg") return unite === "g" ? n * 1000 : unite === "kg" ? n : null;
  if (grand.startsWith("l")) return unite === "ml" ? n * 1000 : unite === "litre" ? n : null;
  return n;
}

/** Motifs habituels d'une sortie de stock, selon son type. */
export const MOTIFS_SORTIE: Record<string, string[]> = {
  casse: ["Cassé au service", "Cassé à la livraison", "Cassé au rangement"],
  perte: ["Introuvable au comptage", "Renversé", "Mal conservé"],
  perime: ["Date dépassée", "Abîmé, jeté"],
  vol: ["Constaté au comptage", "Constaté par le gérant"],
  repas_personnel: ["Repas du midi", "Repas du soir"],
  offert: ["Offert à un client fidèle", "Geste commercial"],
  consommation_interne: ["Utilisé en cuisine", "Nettoyage"],
  retour_fournisseur: ["Produit défectueux", "Erreur de livraison"],
  regularisation: ["Erreur de saisie", "Correction après comptage"],
};

export const EMBALLAGES = [
  "Bouteille bière 65 cl",
  "Bouteille bière 33 cl",
  "Bouteille soda 30 cl",
  "Bouteille eau 1,5 L",
  "Casier bière (12)",
  "Casier bière (24)",
  "Casier soda (24)",
  "Bouteille de gaz 6 kg",
  "Bouteille de gaz 12 kg",
];

export const MOTIFS_EMBALLAGE: Record<string, string[]> = {
  casse: ["Cassée au service", "Cassée au rangement"],
  perte: ["Introuvable au comptage"],
  sortie_client: ["Emportée avec la commande"],
  retour_client: ["Rapportée par le client"],
};
