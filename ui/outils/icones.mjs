// Produit les icônes PNG de l'application installable depuis ui/public/icone*.svg.
// Usage (depuis ui/) : node outils/icones.mjs   — nécessite Chromium (Playwright).
import { readFileSync } from "node:fs";
import { chromium } from "playwright";

const DOSSIER = new URL("../public/", import.meta.url);
const SORTIES = [
  ["icone.svg", "icone-192.png", 192],
  ["icone.svg", "icone-512.png", 512],
  ["icone-masquable.svg", "icone-masquable-512.png", 512],
  ["icone-masquable.svg", "apple-touch-icon.png", 180],
];

const nav = await chromium.launch(process.env.PLAYWRIGHT_CHROMIUM ? { executablePath: process.env.PLAYWRIGHT_CHROMIUM } : {});
const page = await nav.newPage();
for (const [source, cible, taille] of SORTIES) {
  const svg = readFileSync(new URL(source, DOSSIER), "utf8");
  await page.setViewportSize({ width: taille, height: taille });
  await page.setContent(`<style>html,body{margin:0;background:transparent}svg{width:${taille}px;height:${taille}px;display:block}</style>${svg}`);
  await page.screenshot({ path: new URL(cible, DOSSIER).pathname, omitBackground: true });
  console.log(cible);
}
await nav.close();
