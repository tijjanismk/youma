// Version à imprimer de la formation du personnel : captures d'écran sur une base de démonstration neuve,
// puis PDF A4 de docs/guides/formation/formation-personnel.html.
// Usage (depuis ui/, après `npm run build` et `cargo build -p youma-server`) : node outils/formation.mjs
// Chromium local plus ancien : PLAYWRIGHT_CHROMIUM=/chemin/vers/chrome node outils/formation.mjs
import { chromium } from "@playwright/test";
import { spawn } from "node:child_process";
import { mkdirSync, mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const ici = dirname(fileURLToPath(import.meta.url));
const racine = resolve(ici, "../..");
const dossier = join(racine, "docs", "guides", "formation");
const captures = join(dossier, "captures");
mkdirSync(captures, { recursive: true });

const port = "7981";
const U = `http://127.0.0.1:${port}`;
const binaire = join(racine, "target", "debug", process.platform === "win32" ? "youma-server.exe" : "youma-server");
const donnees = mkdtempSync(join(tmpdir(), "youma-formation-"));
const serveur = spawn(binaire, ["--demo", "--donnees", donnees, "--port", port, "--port-https", "0", "--ui", resolve(ici, "../dist")], {
  stdio: "ignore",
});
await new Promise((r) => setTimeout(r, 2500));

const navigateur = await chromium.launch(process.env.PLAYWRIGHT_CHROMIUM ? { executablePath: process.env.PLAYWRIGHT_CHROMIUM } : {});
const options = { serviceWorkers: "block", deviceScaleFactor: 1.5 };
const pc = await (await navigateur.newContext({ ...options, viewport: { width: 1180, height: 760 } })).newPage();
pc.setDefaultTimeout(15000);

async function capture(page, nom, pleinePage = false) {
  await page.waitForTimeout(350);
  await page.screenshot({ path: join(captures, `${nom}.png`), fullPage: pleinePage });
}
async function connexion(page, qui, pin, avant) {
  await page.goto(`${U}/`);
  const changer = page.getByRole("button", { name: "Changer d'utilisateur" });
  await changer
    .or(page.getByRole("button", { name: qui }))
    .first()
    .waitFor();
  if (await changer.isVisible()) await changer.click();
  if (avant) await capture(page, avant);
  await page.getByRole("button", { name: qui }).click();
  for (const c of pin) await page.getByRole("button", { name: `Chiffre ${c}` }).click();
  await page.getByRole("button", { name: "Entrer" }).click();
  await changer.waitFor();
}

try {
  // 1. Connexion.
  await pc.goto(`${U}/`);
  await pc.getByRole("button", { name: /Mariam/ }).waitFor();
  await capture(pc, "01-connexion");
  await pc.getByRole("button", { name: /Mariam/ }).click();
  for (const c of "12") await pc.getByRole("button", { name: `Chiffre ${c}` }).click();
  await capture(pc, "02-pin");
  for (const c of "34") await pc.getByRole("button", { name: `Chiffre ${c}` }).click();
  await pc.getByRole("button", { name: "Entrer" }).click();
  await pc.getByRole("button", { name: "Changer d'utilisateur" }).waitFor();

  // 2. Journée et caisse.
  await capture(pc, "03-ouvrir-journee");
  await pc.getByRole("button", { name: "Ouvrir la journée" }).click();
  await pc.getByText(/Journée d'exploitation du/).waitFor();
  await pc.goto(`${U}/caisse`);
  await pc.getByLabel("Fond de caisse compté").fill("10000");
  await pc.getByLabel("Motif de l'écart").fill("Fond de départ");
  await capture(pc, "04-ouvrir-caisse");
  await pc.getByRole("button", { name: /^Ouvrir (la caisse|ma session)/ }).click();
  await pc
    .getByText(/Session de/)
    .first()
    .waitFor();

  // 3. Commande à table : 2 bières, brochettes (quantité tapée).
  await pc.goto(`${U}/salle`);
  await pc.getByRole("button", { name: /^Table 4 Libre/ }).click();
  await pc.getByRole("tab", { name: /Bières/ }).click();
  const biere = pc.getByRole("button", { name: /^Bière blonde \d/ });
  await biere.click();
  await biere.click();
  await pc.getByRole("tab", { name: /Grillades/ }).click();
  await pc.getByRole("button", { name: /^Brochettes \(3\) \d/ }).click();
  const qte = pc.getByLabel("Quantité Brochettes (3)");
  await qte.fill("3");
  await qte.press("Enter");
  await capture(pc, "05-commande");
  await pc.getByRole("button", { name: /^Envoyer \(5\)/ }).click();
  await pc.locator(".ligne.envoyee").first().waitFor();

  // 4. Cuisine.
  await pc.goto(`${U}/cuisine`);
  await pc.locator(".carte-cuisine").first().waitFor();
  await capture(pc, "06-cuisine");
  await pc.getByRole("button", { name: "En préparation" }).first().click();

  // 5. Salle : table occupée, bouton Libérer.
  await pc.goto(`${U}/salle`);
  await pc.getByRole("button", { name: /^Table 4 Occupée/ }).waitFor();
  await capture(pc, "07-salle");

  // 6. Encaissement : espèces + Orange Money.
  await pc.getByRole("button", { name: /^Table 4 Occupée/ }).click();
  await pc.getByRole("button", { name: /^Encaisser / }).click();
  await pc
    .getByRole("button", { name: /Espèces$/ })
    .first()
    .click();
  const montants = pc.getByLabel("Montant", { exact: true });
  await montants.first().fill("2000");
  await pc.getByRole("button", { name: /Orange Money/ }).click();
  await pc.getByLabel("Référence de la transaction").fill("PP260926.1930.B4521");
  await pc.getByLabel("Espèces reçues du client").fill("5000");
  await pc.evaluate(() => (document.activeElement instanceof HTMLElement ? document.activeElement.blur() : undefined));
  await capture(pc, "08-encaisser", true);
  await pc.getByRole("button", { name: "Valider le paiement" }).click();
  await pc.getByText(/Reçu n°\d+/).waitFor();
  await capture(pc, "09-bon-de-sortie");
  const bon = pc.locator(".bon-sortie strong");
  const numero = (await bon.nth(0).textContent()) ?? "";
  const code = (await bon.nth(1).textContent()) ?? "";
  await pc.getByRole("button", { name: "Terminé" }).click();

  // 7. Contrôle de sortie.
  await pc.goto(`${U}/sortie`);
  await pc.getByLabel("N° du bon de sortie").fill(numero);
  await pc.getByLabel("Code de contrôle").fill(code);
  await pc.getByRole("button", { name: "Vérifier" }).click();
  await pc.getByText("PAYÉ — peut sortir").waitFor();
  await capture(pc, "10-sortie");

  // 8. Annulation d'un article envoyé (motif).
  await pc.goto(`${U}/salle`);
  await pc.getByRole("button", { name: /^Table T3 Libre/ }).click();
  await pc.getByRole("tab", { name: /Grillades/ }).click();
  await pc.getByRole("button", { name: /^Poulet braisé \d/ }).click();
  await pc.getByRole("button", { name: /^Envoyer \(1\)/ }).click();
  await pc.locator(".ligne.envoyee .ligne-bouton").click();
  await pc.getByRole("button", { name: /^Annuler/ }).click();
  await pc.getByRole("button", { name: "Client a changé d'avis" }).waitFor();
  await capture(pc, "11-annulation");
  await pc.getByRole("button", { name: "Client a changé d'avis" }).click();
  await pc.getByRole("button", { name: "Confirmer" }).click();
  await pc.getByText("Article annulé").waitFor();

  // 9. Stock : perte / sortie.
  await pc.goto(`${U}/stock`);
  await pc
    .getByRole("row", { name: /Bière blonde/ })
    .getByRole("button", { name: "Perte / sortie" })
    .click();
  const sortie = pc.getByRole("dialog");
  await sortie.getByLabel("Motif").selectOption("Cassé au service");
  await capture(pc, "12-stock-sortie");
  await pc.keyboard.press("Escape");

  // 10. Ma journée.
  await pc.goto(`${U}/tableau-de-bord`);
  await pc.locator(".indicateur").first().waitFor();
  await capture(pc, "13-ma-journee");

  // 11. Clôture de caisse (billetage), sans valider.
  await pc.goto(`${U}/caisse`);
  await pc.getByRole("button", { name: "Clôturer ma caisse" }).click();
  await pc.getByLabel("Nombre de 10000", { exact: true }).fill("1");
  await pc.getByLabel("Nombre de 2000", { exact: true }).fill("1");
  await capture(pc, "14-cloture-caisse");
  await pc.keyboard.press("Escape");

  // 12. Téléphone du serveur : commande en tiroir.
  const tel = await (await navigateur.newContext({ ...options, viewport: { width: 390, height: 780 }, isMobile: true, hasTouch: true })).newPage();
  await connexion(tel, /Awa/, "4444");
  await tel.goto(`${U}/salle`);
  await tel.getByRole("button", { name: /^Table 2 Libre/ }).click();
  await tel.getByRole("tab", { name: /Boissons/ }).click();
  await tel.getByRole("button", { name: /^Coca-Cola \d/ }).click();
  await tel.getByRole("button", { name: /^Coca-Cola \d/ }).click();
  await capture(tel, "15-telephone-carte");
  await tel.getByRole("button", { name: /^Voir la commande/ }).click();
  await capture(tel, "16-telephone-commande");

  // PDF A4 de la fiche.
  const doc = await (await navigateur.newContext()).newPage();
  await doc.goto(pathToFileURL(join(dossier, "formation-personnel.html")).href);
  await doc.waitForLoadState("networkidle");
  await doc.pdf({
    path: join(dossier, "formation-personnel.pdf"),
    format: "A4",
    printBackground: true,
    margin: { top: "14mm", bottom: "14mm", left: "14mm", right: "14mm" },
  });
  console.log("Captures et PDF dans", dossier);
} finally {
  await navigateur.close();
  serveur.kill();
}
