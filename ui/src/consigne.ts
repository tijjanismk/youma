import type { ConsigneAchat } from "./types";

/** RG-CON-02 : consigne nette d'une livraison = Σ (reçus − rendus) × valeur ; s'ajoute au total de l'achat. */
export function consigneNette(consignes: ConsigneAchat[], valeurs: Record<string, number>): number {
  return consignes.reduce((s, c) => s + (c.recus - c.rendus) * (valeurs[c.emballage_id] ?? 0), 0);
}

/** Lignes utiles d'une livraison : au moins un emballage reçu ou rendu, nombres entiers positifs. */
export function consignesSaisies(consignes: ConsigneAchat[]): ConsigneAchat[] {
  return consignes.filter((c) => c.recus > 0 || c.rendus > 0);
}
