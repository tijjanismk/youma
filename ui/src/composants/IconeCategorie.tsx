import type { LucideIcon } from "lucide-react";
import {
  Apple,
  Beef,
  Beer,
  CakeSlice,
  Coffee,
  CookingPot,
  CupSoda,
  Drumstick,
  EggFried,
  Fish,
  GlassWater,
  Hamburger,
  IceCreamCone,
  Pizza,
  Salad,
  Sandwich,
  Soup,
  Utensils,
  Wine,
} from "lucide-react";

/**
 * Icônes des catégories (fiche 0035) : choisies dans une liste, dessinées (lucide), plus d'émoji.
 * `emojis` : anciennes valeurs saisies avant cette fiche, reconnues pour les bases existantes.
 */
export const ICONES_CATEGORIE: { cle: string; libelle: string; Icone: LucideIcon; emojis: string[] }[] = [
  { cle: "boissons", libelle: "Boissons", Icone: CupSoda, emojis: ["🥤", "🧃", "🧋"] },
  { cle: "bieres", libelle: "Bières", Icone: Beer, emojis: ["🍺", "🍻"] },
  { cle: "vins", libelle: "Vins et alcools", Icone: Wine, emojis: ["🍷", "🍸", "🥃", "🍹"] },
  { cle: "eau", libelle: "Eau", Icone: GlassWater, emojis: ["💧", "🚰"] },
  { cle: "cafe", libelle: "Café, thé", Icone: Coffee, emojis: ["☕", "🍵"] },
  { cle: "grillades", libelle: "Grillades", Icone: Drumstick, emojis: ["🍗", "🍖", "🔥"] },
  { cle: "viandes", libelle: "Viandes", Icone: Beef, emojis: ["🥩"] },
  { cle: "poissons", libelle: "Poissons", Icone: Fish, emojis: ["🐟", "🐠", "🦐"] },
  { cle: "plats", libelle: "Plats", Icone: CookingPot, emojis: ["🍛", "🍲", "🥘"] },
  { cle: "soupes", libelle: "Soupes", Icone: Soup, emojis: ["🍜"] },
  { cle: "accompagnements", libelle: "Accompagnements, salades", Icone: Salad, emojis: ["🍟", "🥗", "🍚"] },
  { cle: "sandwichs", libelle: "Sandwichs", Icone: Sandwich, emojis: ["🥪", "🌯"] },
  { cle: "burgers", libelle: "Burgers", Icone: Hamburger, emojis: ["🍔"] },
  { cle: "pizzas", libelle: "Pizzas", Icone: Pizza, emojis: ["🍕"] },
  { cle: "petit_dejeuner", libelle: "Petit-déjeuner", Icone: EggFried, emojis: ["🍳", "🥚"] },
  { cle: "desserts", libelle: "Desserts", Icone: CakeSlice, emojis: ["🍰", "🎂", "🍮"] },
  { cle: "glaces", libelle: "Glaces", Icone: IceCreamCone, emojis: ["🍦", "🍨"] },
  { cle: "fruits", libelle: "Fruits", Icone: Apple, emojis: ["🍎", "🍊", "🍉", "🥭"] },
  { cle: "autre", libelle: "Autre (couverts)", Icone: Utensils, emojis: ["🍽️", "🍽"] },
];

/** Icône d'une catégorie : clé de la liste, ancien émoji reconnu, sinon les couverts. */
export function iconeCategorie(icone?: string | null): LucideIcon {
  const v = (icone ?? "").trim().replace(/\uFE0F/g, "");
  return ICONES_CATEGORIE.find((i) => i.cle === v || i.emojis.some((e) => e.replace(/\uFE0F/g, "") === v))?.Icone ?? Utensils;
}

/** Clé de la liste correspondant à une valeur enregistrée (pour pré-remplir le choix). */
export function cleIcone(icone?: string | null): string {
  const v = (icone ?? "").trim().replace(/\uFE0F/g, "");
  return ICONES_CATEGORIE.find((i) => i.cle === v || i.emojis.some((e) => e.replace(/\uFE0F/g, "") === v))?.cle ?? "autre";
}

export function IconeCategorie({ icone, taille = 18 }: { icone?: string | null; taille?: number | string }) {
  const Icone = iconeCategorie(icone);
  return <Icone size={taille} strokeWidth={1.9} aria-hidden />;
}
