// Applications Android « Youma Client » et « Youma Livreur » (fiche 0041) : la même interface, compilée avec
// VITE_APPLI = client | livreur et embarquée par Capacitor. Elles parlent au relais du restaurant (https), jamais
// au poste central. Sur le web, rien ne change : VITE_APPLI est vide et l'API reste relative.

export type Appli = "client" | "livreur";

/** Application compilée (constante remplacée à la compilation : le code des applications disparaît du web). */
export const APPLI: Appli | null = import.meta.env.VITE_APPLI === "client" || import.meta.env.VITE_APPLI === "livreur" ? import.meta.env.VITE_APPLI : null;

export type Restaurant = { nom: string; relais: string };
export type Course = { relais: string; code: string; nom: string; ajoutee: number };

const CLE_RELAIS = "youma.appli.relais";
const CLE_RESTAURANTS = "youma.appli.restaurants";
const CLE_COURSES = "youma.appli.courses";
const CLE_SUIVIS = "youma.appli.suivis";

function lireJson<T>(cle: string, defaut: T): T {
  try {
    const v = JSON.parse(localStorage.getItem(cle) ?? "null");
    return v ?? defaut;
  } catch {
    return defaut;
  }
}

function ecrireJson(cle: string, v: unknown) {
  try {
    localStorage.setItem(cle, JSON.stringify(v));
  } catch {
    /* stockage indisponible */
  }
}

/**
 * Relais partagé (fiche 0049) : sur le web, les pages d'un restaurant inscrit sont sous `/r/<nom>/…` ; ses appels à
 * l'API et ses liens gardent ce début. Vide à la racine (restaurant historique du relais, poste central) et dans les
 * applications (l'adresse du relais y comprend déjà `/r/<nom>`).
 */
export function prefixeWeb(): string {
  if (APPLI) return "";
  return /^\/r\/[a-z0-9-]{2,40}(?=\/|$)/.exec(location.pathname)?.[0] ?? "";
}

/** Adresse d'une page publique du même restaurant (`/suivi/…` → `/r/<nom>/suivi/…` sur le relais partagé). */
export function cheminPublic(p: string): string {
  return `${prefixeWeb()}${p}`;
}

/** Page publique affichée, sans le début `/r/<nom>`. */
export function cheminCourant(): string {
  return location.pathname.slice(prefixeWeb().length) || "/";
}

/**
 * Adresse du relais d'un lien collé ou partagé : « https://resto.up.railway.app/menu », « resto.up.railway.app »…
 * Toujours en https : la position et le paiement passent par Internet (fiche 0013).
 */
export function adresseRelais(saisie: string): string | null {
  let s = saisie.trim();
  if (!s) return null;
  if (!/^[a-z]+:\/\//i.test(s)) s = `https://${s}`;
  try {
    const u = new URL(s);
    if (u.protocol !== "https:" || !u.hostname.includes(".")) return null;
    // Relais partagé : le restaurant fait partie de l'adresse (`https://relais/r/<nom>`, fiche 0049).
    return u.origin + (/^\/r\/[a-z0-9-]{2,40}(?=\/|$)/.exec(u.pathname)?.[0] ?? "");
  } catch {
    return null;
  }
}

/** Relais utilisé par les appels de l'API (vide sur le web : appels relatifs). */
export function baseApi(): string {
  if (!APPLI) return prefixeWeb();
  try {
    return localStorage.getItem(CLE_RELAIS) ?? "";
  } catch {
    return "";
  }
}

export function choisirRelais(relais: string) {
  try {
    localStorage.setItem(CLE_RELAIS, relais);
  } catch {
    /* stockage indisponible */
  }
}

export function lireRestaurants(): Restaurant[] {
  const l = lireJson<Restaurant[]>(CLE_RESTAURANTS, []);
  return Array.isArray(l) ? l.filter((r) => r && typeof r.relais === "string") : [];
}

export function retenirRestaurant(r: Restaurant) {
  ecrireJson(CLE_RESTAURANTS, [r, ...lireRestaurants().filter((x) => x.relais !== r.relais)].slice(0, 20));
}

export function oublierRestaurant(relais: string) {
  ecrireJson(
    CLE_RESTAURANTS,
    lireRestaurants().filter((x) => x.relais !== relais),
  );
}

/** Restaurant de chaque code de suivi (un client peut commander dans plusieurs restaurants). */
export function relaisDuSuivi(code: string): string | null {
  return lireJson<Record<string, string>>(CLE_SUIVIS, {})[code] ?? null;
}

export function associerSuivi(code: string) {
  const relais = baseApi();
  if (!relais) return;
  const m = lireJson<Record<string, string>>(CLE_SUIVIS, {});
  m[code] = relais;
  ecrireJson(CLE_SUIVIS, m);
}

export function lireCourses(): Course[] {
  const l = lireJson<Course[]>(CLE_COURSES, []);
  return Array.isArray(l) ? l.filter((c) => c && typeof c.code === "string" && typeof c.relais === "string") : [];
}

export function retenirCourse(c: Course) {
  ecrireJson(CLE_COURSES, [c, ...lireCourses().filter((x) => x.code !== c.code)].slice(0, 30));
}

export function oublierCourse(code: string) {
  ecrireJson(
    CLE_COURSES,
    lireCourses().filter((x) => x.code !== code),
  );
}

/**
 * Lien d'ouverture reçu par l'application (bouton de la page web ou lien WhatsApp) :
 * `youma-livreur://course?relais=https://…&code=…` ou `youma-client://menu?relais=https://…`.
 * Accepte aussi un lien web du relais (`https://…/livreur/CODE`, `https://…/menu`, `https://…/suivi/CODE`).
 */
export function lireLien(lien: string): { relais: string; code?: string; page: "menu" | "suivi" | "course" } | null {
  let u: URL;
  try {
    u = new URL(lien.trim());
  } catch {
    const relais = adresseRelais(lien);
    return relais ? { relais, page: "menu" } : null;
  }
  if (u.protocol === "youma-livreur:" || u.protocol === "youma-client:") {
    const relais = adresseRelais(u.searchParams.get("relais") ?? "");
    if (!relais) return null;
    const code = u.searchParams.get("code") ?? undefined;
    const quoi = u.hostname || u.pathname.replace(/^\/+/, "");
    if (quoi === "course") return code ? { relais, code, page: "course" } : null;
    if (quoi === "suivi") return code ? { relais, code, page: "suivi" } : null;
    return { relais, page: "menu" };
  }
  const relais = adresseRelais(`${u.origin}${u.pathname}`);
  if (!relais) return null;
  const [, quoi, code] = u.pathname.replace(/^\/r\/[a-z0-9-]{2,40}(?=\/|$)/, "").split("/");
  if (quoi === "livreur" && code) return { relais, code: decodeURIComponent(code), page: "course" };
  if (quoi === "suivi" && code) return { relais, code: decodeURIComponent(code), page: "suivi" };
  return { relais, page: "menu" };
}
