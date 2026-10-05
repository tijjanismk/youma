/**
 * Menu personnalisé (fiche 0051) : chaque utilisateur choisit, sur son appareil, les écrans affichés dans son menu et
 * leur ordre. Ce n'est pas un droit d'accès : les permissions restent celles du rôle, et un écran caché reste ouvrable.
 */

export type MenuPerso = { caches: string[]; ordre: string[] };

const cle = (utilisateur: string) => `youma.menu.${utilisateur}`;

export function lireMenuPerso(utilisateur: string): MenuPerso {
  try {
    const v = JSON.parse(localStorage.getItem(cle(utilisateur)) ?? "null");
    const liste = (x: unknown) => (Array.isArray(x) ? x.filter((c): c is string => typeof c === "string") : []);
    return { caches: liste(v?.caches), ordre: liste(v?.ordre) };
  } catch {
    return { caches: [], ordre: [] };
  }
}

export function ecrireMenuPerso(utilisateur: string, m: MenuPerso | null) {
  try {
    if (m) localStorage.setItem(cle(utilisateur), JSON.stringify(m));
    else localStorage.removeItem(cle(utilisateur));
  } catch {
    /* stockage indisponible : menu d'origine */
  }
}

/** Range les entrées dans l'ordre choisi (les nouvelles à leur place d'origine, à la fin), sans les cachées si demandé. */
export function appliquerMenuPerso<T extends { chemin: string }>(liens: T[], m: MenuPerso, avecCaches = false): T[] {
  const rang = (c: string) => {
    const i = m.ordre.indexOf(c);
    return i === -1 ? m.ordre.length : i;
  };
  return liens
    .map((l, i) => ({ l, i }))
    .sort((a, b) => rang(a.l.chemin) - rang(b.l.chemin) || a.i - b.i)
    .map((x) => x.l)
    .filter((l) => avecCaches || !m.caches.includes(l.chemin));
}
