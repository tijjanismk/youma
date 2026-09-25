import type { LigneRecette } from "./types";

/** RG-REC-04 : coût matière = Σ quantité × coût unitaire de l'article (FCFA entiers). */
export function coutRecette(lignes: LigneRecette[], couts: Record<string, number>): number {
  return lignes.reduce((s, l) => s + l.quantite * (couts[l.article_id] ?? l.cout_unitaire ?? 0), 0);
}

/** Part du prix en points de base (3 000 = 30 %), arrondie vers le bas comme le poste central. */
export function partBp(cout: number, prix: number): number {
  return prix > 0 ? Math.floor((cout * 10_000) / prix) : 0;
}

/** 4133 → « 41,33 % ». */
export function pourcentage(bp: number): string {
  return `${(bp / 100).toFixed(2).replace(".", ",")} %`;
}

/** Lignes valides : un article choisi, quantité entière positive, pas de doublon (RG-REC-01). */
export function recetteValide(lignes: LigneRecette[]): boolean {
  const ids = lignes.map((l) => l.article_id);
  return lignes.every((l) => l.article_id && Number.isInteger(l.quantite) && l.quantite > 0) && new Set(ids).size === ids.length;
}
