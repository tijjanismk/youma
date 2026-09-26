/** Thème « Mali vivant » : clair (le jour, au soleil) ou sombre (cuisine, service du soir), choisi et mémorisé par poste. */

export type Theme = "clair" | "sombre";

const CLE = "youma.theme";
/** Couleur `fond` de chaque thème, reprise par la barre du navigateur et de l'application installée. */
const COULEUR_BARRE: Record<Theme, string> = { clair: "#f5f6fa", sombre: "#12152b" };

export function lireTheme(): Theme {
  try {
    return localStorage.getItem(CLE) === "sombre" ? "sombre" : "clair";
  } catch {
    return "clair";
  }
}

export function appliquerTheme(theme: Theme) {
  document.documentElement.dataset.theme = theme;
  document.querySelector('meta[name="theme-color"]')?.setAttribute("content", COULEUR_BARRE[theme]);
}

export function choisirTheme(theme: Theme) {
  appliquerTheme(theme);
  try {
    localStorage.setItem(CLE, theme);
  } catch {
    /* stockage indisponible : thème pour cette visite seulement */
  }
}
