// Icônes et écrans de démarrage des applications Youma Client et Youma Livreur (fiche 0041), depuis
// apps/mobile/<appli>/icone.svg. Usage (depuis ui/) : node outils/icones-applis.mjs — nécessite Chromium (Playwright).
import { readFileSync } from "node:fs";
import { chromium } from "playwright";

const APPLIS = { client: "#c45200", livreur: "#1c2150" };
const DENSITES = { mdpi: 1, hdpi: 1.5, xhdpi: 2, xxhdpi: 3, xxxhdpi: 4 };
const ECRANS = {
  "drawable/splash.png": [480, 320],
  "drawable-port-mdpi/splash.png": [320, 480],
  "drawable-port-hdpi/splash.png": [480, 800],
  "drawable-port-xhdpi/splash.png": [720, 1280],
  "drawable-port-xxhdpi/splash.png": [960, 1600],
  "drawable-port-xxxhdpi/splash.png": [1280, 1920],
  "drawable-land-mdpi/splash.png": [480, 320],
  "drawable-land-hdpi/splash.png": [800, 480],
  "drawable-land-xhdpi/splash.png": [1280, 720],
  "drawable-land-xxhdpi/splash.png": [1600, 960],
  "drawable-land-xxxhdpi/splash.png": [1920, 1280],
};

const nav = await chromium.launch(process.env.PLAYWRIGHT_CHROMIUM ? { executablePath: process.env.PLAYWRIGHT_CHROMIUM } : {});
const page = await nav.newPage();

async function rendre(html, l, h, chemin) {
  await page.setViewportSize({ width: l, height: h });
  await page.setContent(`<style>html,body{margin:0;background:transparent;overflow:hidden}svg{display:block}</style>${html}`);
  await page.screenshot({ path: chemin, omitBackground: true });
}

for (const [appli, fond] of Object.entries(APPLIS)) {
  const racine = new URL(`../../apps/mobile/${appli}/`, import.meta.url);
  const svg = readFileSync(new URL("icone.svg", racine), "utf8");
  const icone = (t, style = "") => svg.replace("<svg ", `<svg width="${t}" height="${t}" style="${style}" `);
  const res = new URL("android/app/src/main/res/", racine).pathname;
  for (const [d, k] of Object.entries(DENSITES)) {
    const t = 48 * k;
    await rendre(icone(t), t, t, `${res}mipmap-${d}/ic_launcher.png`);
    await rendre(icone(t, "border-radius:50%"), t, t, `${res}mipmap-${d}/ic_launcher_round.png`);
    // Icône adaptative : 108 dp, l'image entière en premier plan (le dessin tient dans la zone sûre).
    await rendre(icone(108 * k), 108 * k, 108 * k, `${res}mipmap-${d}/ic_launcher_foreground.png`);
  }
  for (const [f, [l, h]] of Object.entries(ECRANS)) {
    const t = Math.round(Math.min(l, h) * 0.35);
    await rendre(`<div style="width:${l}px;height:${h}px;background:${fond};display:grid;place-items:center">${icone(t, "border-radius:22%")}</div>`, l, h, res + f);
  }
  const ios = new URL("ios/App/App/Assets.xcassets/", racine).pathname;
  await rendre(icone(1024), 1024, 1024, `${ios}AppIcon.appiconset/AppIcon-512@2x.png`);
  for (const f of ["splash-2732x2732.png", "splash-2732x2732-1.png", "splash-2732x2732-2.png"])
    await rendre(`<div style="width:2732px;height:2732px;background:${fond};display:grid;place-items:center">${icone(700, "border-radius:22%")}</div>`, 2732, 2732, `${ios}Splash.imageset/${f}`);
  console.log(appli);
}
await nav.close();
