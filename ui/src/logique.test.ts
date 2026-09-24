import { beforeEach, describe, expect, it } from "vitest";
import { dateFr, fcfa, finDuMois, lireMontant, nombre } from "./format";
import { t } from "./i18n";
import { ajouter, changerQuantite, chargerPanier, optionsValides, prixZone, sauverPanier, totalPanier, versLignes } from "./panier";
import { billetsProposes, rendu, sommeParts, verifierPaiement } from "./paiement";
import type { Produit } from "./types";

const produit = (p: Partial<Produit>): Produit => ({
  id: "p1",
  categorie_id: "c1",
  nom: "Bière blonde",
  nom_court: "",
  description: "",
  photo: "",
  prix: 1000,
  poste_id: null,
  disponible: true,
  actif: true,
  code: "",
  code_barres: "",
  taux_tva_bp: 0,
  suivi_stock: "aucun",
  article_stock_id: null,
  prix_achat_estime: 0,
  ordre: 0,
  prix_zones: [],
  groupes_options: [],
  ...p,
});

describe("format FCFA", () => {
  it("formate les montants entiers à la française", () => {
    expect(fcfa(12500)).toBe("12 500 FCFA");
    expect(nombre(1000000)).toBe("1 000 000");
    expect(nombre(-3000)).toBe("-3 000");
    expect(nombre(750)).toBe("750");
  });
  it("lit une saisie avec espaces et séparateurs", () => {
    expect(lireMontant("12 500")).toBe(12500);
    expect(lireMontant("7.500 F")).toBe(7500);
    expect(lireMontant("")).toBe(0);
    expect(lireMontant("-2000")).toBe(-2000);
  });
  it("dates", () => {
    expect(dateFr("2026-03-14")).toBe("14/03/2026");
    expect(finDuMois("2026-02-10")).toBe("2026-02-28");
    expect(finDuMois("2028-02-10")).toBe("2028-02-29");
  });
});

describe("traductions", () => {
  it("traduit et rend lisible une clé inconnue", () => {
    expect(t("especes")).toBe("Espèces");
    expect(t("aucun")).toBe("Aucun");
    expect(t("cle_inconnue")).toBe("cle inconnue");
    expect(t(null)).toBe("");
  });
});

describe("panier local (scénario 13 : rien n'est perdu)", () => {
  beforeEach(() => localStorage.clear());

  it("regroupe les mêmes articles et applique la grille de zone", () => {
    const biere = produit({ prix_zones: [{ zone_id: "vip", prix: 1500 }] });
    let p = ajouter([], biere, "vip");
    p = ajouter(p, biere, "vip");
    p = ajouter(p, biere, "vip");
    expect(p).toHaveLength(1);
    expect(p[0].quantite).toBe(3);
    expect(totalPanier(p)).toBe(4500);
    expect(prixZone(biere, null)).toBe(1000);
  });

  it("sépare les articles avec options ou commentaire", () => {
    const frites = produit({ id: "f", nom: "Frites", prix: 750 });
    let p = ajouter([], frites, null, [{ id: "g", nom: "Grande", supplement: 500 }]);
    p = ajouter(p, frites, null);
    p = ajouter(p, frites, null, [], "sans sel");
    expect(p).toHaveLength(3);
    expect(totalPanier(p)).toBe(1250 + 750 + 750);
    expect(versLignes(p)[0]).toEqual({ produit_id: "f", quantite: 1, options: ["g"], commentaire: "" });
  });

  it("retire un article quand sa quantité tombe à zéro", () => {
    let p = ajouter([], produit({}), null);
    p = changerQuantite(p, p[0].cle, -1);
    expect(p).toHaveLength(0);
  });

  it("survit à un rechargement de la page", () => {
    const p = ajouter([], produit({}), null);
    sauverPanier("cmd-1", p);
    expect(chargerPanier("cmd-1")).toEqual(p);
    sauverPanier("cmd-1", []);
    expect(chargerPanier("cmd-1")).toEqual([]);
  });

  it("vérifie les options obligatoires (RG-CAT-04)", () => {
    const frites = produit({ groupes_options: [{ id: "g", nom: "Taille", min_choix: 1, max_choix: 1, options: [{ id: "n", nom: "Normale", supplement: 0 }, { id: "gr", nom: "Grande", supplement: 500 }] }] });
    expect(optionsValides(frites, [])).toMatch(/Taille/);
    expect(optionsValides(frites, ["n", "gr"])).toMatch(/Taille/);
    expect(optionsValides(frites, ["gr"])).toBeNull();
  });
});

describe("encaissement", () => {
  it("paiement mixte 5 000 espèces + 7 500 Orange Money (scénario 2)", () => {
    const parts = [
      { moyen: "especes" as const, montant: 5000 },
      { moyen: "mobile_money" as const, montant: 7500, compte_id: "om", reference: "PP1" },
    ];
    expect(sommeParts(parts)).toBe(12500);
    expect(verifierPaiement(parts, 12500, 10000, true)).toBeNull();
    expect(rendu(parts, 10000)).toBe(5000);
  });

  it("refuse les paiements incohérents", () => {
    expect(verifierPaiement([], 1000, 0, true)).toMatch(/moyen/);
    expect(verifierPaiement([{ moyen: "especes", montant: 2000 }], 1000, 0, true)).toMatch(/dépasse/);
    expect(verifierPaiement([{ moyen: "mobile_money", montant: 1000, compte_id: "om" }], 1000, 0, true)).toMatch(/référence/);
    expect(verifierPaiement([{ moyen: "mobile_money", montant: 1000, compte_id: "om" }], 1000, 0, false)).toBeNull();
    expect(verifierPaiement([{ moyen: "credit", montant: 1000 }], 1000, 0, true)).toMatch(/client/);
    expect(verifierPaiement([{ moyen: "especes", montant: 1000 }], 1000, 500, true)).toMatch(/insuffisantes/);
  });

  it("propose les billets usuels", () => {
    expect(billetsProposes(5500)).toEqual([5500, 6000, 10000]);
    expect(billetsProposes(750)).toEqual([750, 1000, 2000, 5000, 10000]);
  });
});
