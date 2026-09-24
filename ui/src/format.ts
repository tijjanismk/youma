/** 12500 → « 12 500 » (espace simple, lisible sur tous les écrans et tickets). */
export function nombre(n: number): string {
  const signe = n < 0 ? "-" : "";
  const s = Math.abs(Math.trunc(n)).toString();
  return signe + s.replace(/\B(?=(\d{3})+(?!\d))/g, " ");
}

/** 12500 → « 12 500 FCFA ». Montants toujours entiers. */
export function fcfa(n: number): string {
  return `${nombre(n)} FCFA`;
}

/** Saisie utilisateur → entier FCFA (ignore espaces et séparateurs). */
export function lireMontant(texte: string): number {
  const chiffres = texte.replace(/[^\d-]/g, "");
  const n = parseInt(chiffres, 10);
  return Number.isFinite(n) ? n : 0;
}

export function heure(ms: number): string {
  const d = new Date(ms);
  return `${String(d.getUTCHours()).padStart(2, "0")}:${String(d.getUTCMinutes()).padStart(2, "0")}`;
}

export function dateHeure(ms: number): string {
  const d = new Date(ms);
  const j = String(d.getUTCDate()).padStart(2, "0");
  const m = String(d.getUTCMonth() + 1).padStart(2, "0");
  return `${j}/${m}/${d.getUTCFullYear()} ${heure(ms)}`;
}

/** « 2026-03-14 » → « 14/03/2026 ». */
export function dateFr(iso: string | null | undefined): string {
  if (!iso) return "—";
  const [a, m, j] = iso.split("-");
  return `${j}/${m}/${a}`;
}

export function minutesDepuis(ms: number, maintenant = Date.now()): number {
  return Math.max(0, Math.floor((maintenant - ms) / 60_000));
}

/** Date du jour AAAA-MM-JJ (UTC, le Mali est à UTC+0). */
export function aujourdhui(): string {
  return new Date().toISOString().slice(0, 10);
}

export function premierDuMois(iso = aujourdhui()): string {
  return iso.slice(0, 8) + "01";
}

export function finDuMois(iso = aujourdhui()): string {
  const [a, m] = iso.split("-").map(Number);
  const dernier = new Date(Date.UTC(a, m, 0)).getUTCDate();
  return `${iso.slice(0, 8)}${String(dernier).padStart(2, "0")}`;
}

/** 1290 → « 21:30 » (minutes depuis minuit ; 1440 → « 24:00 »). */
export function hhmm(minutes: number): string {
  return `${String(Math.floor(minutes / 60)).padStart(2, "0")}:${String(minutes % 60).padStart(2, "0")}`;
}

/** « 21:30 » → 1290 ; saisie invalide → null. */
export function lireHhmm(texte: string): number | null {
  const m = /^(\d{1,2})(?:[:hH](\d{2})?)?$/.exec(texte.trim());
  if (!m) return null;
  const v = Number(m[1]) * 60 + Number(m[2] ?? 0);
  return v <= 1440 && Number(m[2] ?? 0) < 60 ? v : null;
}

export const JOURS = ["Lun", "Mar", "Mer", "Jeu", "Ven", "Sam", "Dim"];

/** Masque de jours (lundi = 1 … dimanche = 64) → « Tous les jours », « Ven, Sam »… */
export function joursLibelle(bits: number): string {
  if ((bits & 127) === 127) return "Tous les jours";
  return JOURS.filter((_, i) => bits & (1 << i)).join(", ") || "Aucun jour";
}

/** Degrés décimaux ↔ microdegrés entiers (positions GPS stockées en entiers). */
export const versMicro = (degres: number) => Math.round(degres * 1_000_000);
export const depuisMicro = (micro: number) => micro / 1_000_000;
