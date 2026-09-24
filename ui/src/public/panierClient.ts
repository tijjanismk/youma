/** Panier du client (menu QR ou en ligne), conservé sur son téléphone. */

export type LigneClient = { produit_id: string; quantite: number; options: string[]; commentaire: string };
export type PanierClient = LigneClient[];

type ProduitPrix = { id: string; prix: number; groupes_options: { options: { id: string; supplement: number }[] }[] };

export function cleOptions(produitId: string, options: string[]): string {
  return `${produitId}|${[...options].sort().join(",")}`;
}

/** Total indicatif (le poste central recalcule toujours les prix). */
export function totalPanierClient(panier: PanierClient, produits: ProduitPrix[]): number {
  return panier.reduce((s, l) => {
    const p = produits.find((x) => x.id === l.produit_id);
    if (!p) return s;
    const supplements = l.options.reduce((t, o) => t + (p.groupes_options.flatMap((g) => g.options).find((x) => x.id === o)?.supplement ?? 0), 0);
    return s + l.quantite * (p.prix + supplements);
  }, 0);
}

export function lirePanierClient(cle: string): PanierClient {
  try {
    const v = JSON.parse(localStorage.getItem(cle) ?? "[]");
    return Array.isArray(v) ? v.filter((l) => l && typeof l.produit_id === "string" && l.quantite > 0) : [];
  } catch {
    return [];
  }
}

const CLE_SUIVIS = "youma.suivis";

/** Codes de suivi des dernières commandes du client (pour y revenir). */
export function retenirSuivi(code: string) {
  try {
    const l: string[] = JSON.parse(localStorage.getItem(CLE_SUIVIS) ?? "[]");
    localStorage.setItem(CLE_SUIVIS, JSON.stringify([code, ...l.filter((c) => c !== code)].slice(0, 5)));
  } catch {
    /* stockage indisponible */
  }
}

/** Distance approximative en mètres entre deux points en microdegrés. */
export function distanceMetres(a: [number, number], b: [number, number]): number {
  const r = 6_371_000;
  const rad = (m: number) => (m / 1_000_000) * (Math.PI / 180);
  const dlat = rad(b[0] - a[0]);
  const dlon = rad(b[1] - a[1]);
  const h = Math.sin(dlat / 2) ** 2 + Math.cos(rad(a[0])) * Math.cos(rad(b[0])) * Math.sin(dlon / 2) ** 2;
  return Math.round(2 * r * Math.asin(Math.sqrt(h)));
}
