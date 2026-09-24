/**
 * Client de l'API du poste central. L'interface ne parle jamais à la base :
 * tout passe par ici (cahier §4).
 */

export class ErreurApi extends Error {
  constructor(
    public code: string,
    message: string,
    public regle?: string,
    public permission?: string,
    public statut = 0,
  ) {
    super(message);
  }
  /** RG-AUT-03 : l'action est possible avec le PIN d'un responsable. */
  get autorisationRequise() {
    return this.code === "AUTORISATION_REQUISE";
  }
  get horsLigne() {
    return this.code === "HORS_LIGNE";
  }
}

const CLE_JETON = "youma.jeton";
const CLE_APPAREIL = "youma.appareil";

type Ecouteur = (enLigne: boolean) => void;
const ecouteursReseau = new Set<Ecouteur>();
let enLigne = true;

function signalerReseau(etat: boolean) {
  if (etat !== enLigne) {
    enLigne = etat;
    ecouteursReseau.forEach((e) => e(etat));
  }
}

export function surReseau(e: Ecouteur): () => void {
  ecouteursReseau.add(e);
  return () => ecouteursReseau.delete(e);
}

export function estEnLigne() {
  return enLigne;
}

function lire(cle: string): string | null {
  try {
    return localStorage.getItem(cle);
  } catch {
    return null;
  }
}

function ecrire(cle: string, v: string | null) {
  try {
    if (v === null) localStorage.removeItem(cle);
    else localStorage.setItem(cle, v);
  } catch {
    /* stockage indisponible : la session reste en mémoire */
  }
}

let jetonMemoire: string | null = lire(CLE_JETON);

export function jeton() {
  return jetonMemoire;
}

export function definirJeton(j: string | null) {
  jetonMemoire = j;
  ecrire(CLE_JETON, j);
}

export function definirJetonAppareil(j: string) {
  ecrire(CLE_APPAREIL, j);
}

type Options = {
  methode?: string;
  corps?: unknown;
  texte?: string;
  /** PIN d'un responsable pour une autorisation ponctuelle. */
  pin?: string;
};

export async function appel<T>(chemin: string, o: Options = {}): Promise<T> {
  const entetes: Record<string, string> = {};
  const j = jeton();
  if (j) entetes["Authorization"] = `Bearer ${j}`;
  const appareil = lire(CLE_APPAREIL);
  if (appareil) entetes["X-Appareil"] = appareil;
  if (o.pin) entetes["X-Autorisation-Pin"] = o.pin;
  let corps: BodyInit | undefined;
  if (o.corps !== undefined) {
    entetes["Content-Type"] = "application/json";
    corps = JSON.stringify(o.corps);
  } else if (o.texte !== undefined) {
    entetes["Content-Type"] = "text/plain; charset=utf-8";
    corps = o.texte;
  }
  let r: Response;
  try {
    r = await fetch(`/api${chemin}`, {
      method: o.methode ?? (corps !== undefined ? "POST" : "GET"),
      headers: entetes,
      body: corps,
    });
  } catch {
    // Scénario 13 : poste central injoignable. Rien n'est perdu : l'appelant garde la saisie.
    signalerReseau(false);
    throw new ErreurApi("HORS_LIGNE", "Poste central injoignable. Votre saisie est conservée.");
  }
  signalerReseau(true);
  const type = r.headers.get("content-type") ?? "";
  const donnees = type.includes("application/json") ? await r.json().catch(() => null) : await r.text();
  if (!r.ok) {
    const d = (donnees ?? {}) as { code?: string; message?: string; regle?: string; permission?: string };
    if (r.status === 401 && d.code === "NON_AUTHENTIFIE") evenementDeconnexion();
    throw new ErreurApi(d.code ?? "ERREUR", d.message ?? `Erreur ${r.status}`, d.regle, d.permission, r.status);
  }
  return donnees as T;
}

export const get = <T>(chemin: string) => appel<T>(chemin);
export const post = <T>(chemin: string, corps: unknown = {}, pin?: string) => appel<T>(chemin, { corps, pin });
export const put = <T>(chemin: string, corps: unknown) => appel<T>(chemin, { methode: "PUT", corps });

const ecouteursDeconnexion = new Set<() => void>();
export function surDeconnexion(f: () => void) {
  ecouteursDeconnexion.add(f);
  return () => ecouteursDeconnexion.delete(f);
}
function evenementDeconnexion() {
  ecouteursDeconnexion.forEach((f) => f());
}

/** Téléchargement d'un export (CSV) avec le jeton. */
export async function telecharger(chemin: string, nom: string) {
  const r = await fetch(`/api${chemin}`, { headers: { Authorization: `Bearer ${jeton() ?? ""}` } });
  if (!r.ok) throw new ErreurApi("ERREUR", "Export impossible");
  const url = URL.createObjectURL(await r.blob());
  const a = document.createElement("a");
  a.href = url;
  a.download = nom;
  a.click();
  URL.revokeObjectURL(url);
}
