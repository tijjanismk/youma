/**
 * Panier local d'une addition : les articles tapés avant « Envoyer ».
 * Conservé dans le navigateur : si le poste central tombe, rien n'est perdu (scénario 13).
 */
import type { Produit } from "./types";

export type ArticlePanier = {
  cle: string;
  produit_id: string;
  libelle: string;
  prix: number;
  quantite: number;
  options: { id: string; nom: string; supplement: number }[];
  commentaire: string;
};

const prefixe = "youma.panier.";

export function chargerPanier(commandeId: string): ArticlePanier[] {
  try {
    const brut = localStorage.getItem(prefixe + commandeId);
    return brut ? (JSON.parse(brut) as ArticlePanier[]) : [];
  } catch {
    return [];
  }
}

export function sauverPanier(commandeId: string, panier: ArticlePanier[]) {
  try {
    if (panier.length) localStorage.setItem(prefixe + commandeId, JSON.stringify(panier));
    else localStorage.removeItem(prefixe + commandeId);
  } catch {
    /* stockage indisponible */
  }
}

/** Prix affiché selon la grille de la zone (RG-CAT-03), ou celui du happy hour en cours (RG-PRO-02). Le serveur fait foi. */
export function prixZone(p: Produit, zoneId: string | null, promos: Record<string, number> = {}): number {
  if (promos[p.id] !== undefined) return promos[p.id];
  const pz = zoneId ? p.prix_zones.find((z) => z.zone_id === zoneId) : undefined;
  return pz ? pz.prix : p.prix;
}

/** Ajoute un article ; même produit, mêmes options, sans commentaire → on incrémente. */
export function ajouter(
  panier: ArticlePanier[],
  p: Produit,
  zoneId: string | null,
  options: ArticlePanier["options"] = [],
  commentaire = "",
  promos: Record<string, number> = {},
): ArticlePanier[] {
  const cleOptions = options.map((o) => o.id).sort().join(",");
  const existant = panier.find(
    (a) => a.produit_id === p.id && a.options.map((o) => o.id).sort().join(",") === cleOptions && !a.commentaire && !commentaire,
  );
  if (existant) return panier.map((a) => (a === existant ? { ...a, quantite: a.quantite + 1 } : a));
  return [
    ...panier,
    {
      cle: `${p.id}-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`,
      produit_id: p.id,
      libelle: p.nom_court || p.nom,
      prix: prixZone(p, zoneId, promos),
      quantite: 1,
      options,
      commentaire,
    },
  ];
}

export function changerQuantite(panier: ArticlePanier[], cle: string, delta: number): ArticlePanier[] {
  return panier.map((a) => (a.cle === cle ? { ...a, quantite: a.quantite + delta } : a)).filter((a) => a.quantite > 0);
}

export function totalPanier(panier: ArticlePanier[]): number {
  return panier.reduce((s, a) => s + a.quantite * (a.prix + a.options.reduce((x, o) => x + o.supplement, 0)), 0);
}

/** Format attendu par POST /commandes/{id}/lignes. */
export function versLignes(panier: ArticlePanier[]) {
  return panier.map((a) => ({ produit_id: a.produit_id, quantite: a.quantite, options: a.options.map((o) => o.id), commentaire: a.commentaire }));
}

/** Options valides pour un produit (RG-CAT-04, contrôlé aussi par le serveur). */
export function optionsValides(p: Produit, choisies: string[]): string | null {
  for (const g of p.groupes_options) {
    const n = g.options.filter((o) => choisies.includes(o.id)).length;
    if (n < g.min_choix || n > g.max_choix) {
      return g.min_choix === g.max_choix ? `${g.nom} : choisissez ${g.min_choix} option(s)` : `${g.nom} : entre ${g.min_choix} et ${g.max_choix}`;
    }
  }
  return null;
}
