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

describe("commandes à distance (fiche 0013)", async () => {
  const { hhmm, lireHhmm, joursLibelle, versMicro, depuisMicro } = await import("./format");
  const { cleOptions, distanceMetres, totalPanierClient } = await import("./public/panierClient");

  it("plages horaires en minutes depuis minuit", () => {
    expect(hhmm(21 * 60 + 30)).toBe("21:30");
    expect(hhmm(1440)).toBe("24:00");
    expect(lireHhmm("21:30")).toBe(1290);
    expect(lireHhmm("6h")).toBe(360);
    expect(lireHhmm("06h15")).toBe(375);
    expect(lireHhmm("25:00")).toBeNull();
    expect(lireHhmm("12:75")).toBeNull();
    expect(lireHhmm("n'importe")).toBeNull();
  });

  it("jours de la semaine en masque de bits", () => {
    expect(joursLibelle(127)).toBe("Tous les jours");
    expect(joursLibelle(16 + 32)).toBe("Ven, Sam");
    expect(joursLibelle(0)).toBe("Aucun jour");
  });

  it("positions GPS en microdegrés entiers", () => {
    expect(versMicro(12.639232)).toBe(12_639_232);
    expect(versMicro(-8.0029)).toBe(-8_002_900);
    expect(depuisMicro(12_639_232)).toBeCloseTo(12.639232);
    const d = distanceMetres([12_639_000, -8_002_000], [12_649_000, -8_002_000]);
    expect(d).toBeGreaterThan(1_100);
    expect(d).toBeLessThan(1_120);
  });

  it("panier du client : total indicatif avec suppléments", () => {
    const produits = [
      { id: "b", prix: 1000, groupes_options: [{ options: [{ id: "piment", supplement: 0 }, { id: "frites", supplement: 500 }] }] },
      { id: "j", prix: 500, groupes_options: [] },
    ];
    const total = totalPanierClient(
      [
        { produit_id: "b", quantite: 2, options: ["frites"], commentaire: "" },
        { produit_id: "j", quantite: 1, options: [], commentaire: "" },
        { produit_id: "disparu", quantite: 3, options: [], commentaire: "" },
      ],
      produits,
    );
    expect(total).toBe(2 * 1500 + 500);
    expect(cleOptions("b", ["y", "x"])).toBe(cleOptions("b", ["x", "y"]));
  });
});

describe("recettes (fiche 0014)", async () => {
  const { coutRecette, partBp, pourcentage, recetteValide } = await import("./recette");

  it("coût matière en FCFA entiers et part du prix", () => {
    const couts = { pdt: 1, huile: 2 };
    const lignes = [
      { article_id: "pdt", quantite: 250 },
      { article_id: "huile", quantite: 30 },
    ];
    expect(coutRecette(lignes, couts)).toBe(310);
    expect(partBp(310, 750)).toBe(4_133);
    expect(pourcentage(4_133)).toBe("41,33 %");
    expect(partBp(100, 0)).toBe(0);
  });

  it("recette valide : quantités entières positives, pas de doublon", () => {
    expect(recetteValide([{ article_id: "a", quantite: 10 }])).toBe(true);
    expect(recetteValide([{ article_id: "a", quantite: 0 }])).toBe(false);
    expect(recetteValide([{ article_id: "a", quantite: 1.5 }])).toBe(false);
    expect(recetteValide([{ article_id: "", quantite: 3 }])).toBe(false);
    expect(recetteValide([{ article_id: "a", quantite: 1 }, { article_id: "a", quantite: 2 }])).toBe(false);
  });
});

describe("consignes (fiche 0015)", async () => {
  const { consigneNette, consignesSaisies } = await import("./consigne");

  it("consigne nette d'une livraison : (reçus − rendus) × valeur", () => {
    const valeurs = { bouteille: 150, casier: 2_500 };
    const c = [
      { emballage_id: "bouteille", recus: 24, rendus: 12 },
      { emballage_id: "casier", recus: 2, rendus: 1 },
      { emballage_id: "casier-vide", recus: 0, rendus: 0 },
    ];
    expect(consigneNette(c, valeurs)).toBe(12 * 150 + 2_500);
    expect(consigneNette([{ emballage_id: "casier", recus: 0, rendus: 3 }], valeurs)).toBe(-7_500);
    expect(consignesSaisies(c)).toHaveLength(2);
  });
});

describe("promotions (fiche 0017)", () => {
  it("le prix du happy hour remplace celui de la zone", () => {
    const biere = produit({ id: "b", prix: 1000, prix_zones: [{ zone_id: "vip", prix: 1500 }] });
    expect(prixZone(biere, "vip")).toBe(1500);
    expect(prixZone(biere, "vip", { b: 750 })).toBe(750);
    expect(prixZone(biere, null, { autre: 10 })).toBe(1000);
    const p = ajouter([], biere, null, [], "", { b: 750 });
    expect(totalPanier(p)).toBe(750);
  });
});
