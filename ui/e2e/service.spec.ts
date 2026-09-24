import { expect, Page, test } from "@playwright/test";

// Tests d'interface de bout en bout : vrai poste central (base de démonstration), vrai navigateur.
// Les tests s'enchaînent sur la même journée, comme un vrai service.
test.describe.configure({ mode: "serial" });

async function connexion(page: Page, nom: RegExp, pin: string) {
  await page.goto("/");
  const changer = page.getByRole("button", { name: "Changer d'utilisateur" });
  if (await changer.isVisible().catch(() => false)) await changer.click();
  await page.getByRole("button", { name: nom }).click();
  for (const c of pin) await page.getByRole("button", { name: `Chiffre ${c}` }).click();
  await page.getByRole("button", { name: "Entrer" }).click();
  await expect(page.getByRole("button", { name: "Changer d'utilisateur" })).toBeVisible();
}

async function taperPin(page: Page, pin: string) {
  for (const c of pin) await page.getByRole("dialog").getByRole("button", { name: `Chiffre ${c}` }).click();
}

test("ouverture de la journée et de la caisse", async ({ page }) => {
  await connexion(page, /Kadi/, "3333");
  await expect(page.getByText("Aucune journée ouverte")).toBeVisible();
  await page.getByRole("button", { name: "Ouvrir la journée" }).click();
  await expect(page.getByText(/Journée d'exploitation du/)).toBeVisible();
  await page.getByRole("link", { name: /Caisse/ }).first().click();
  await page.getByLabel("Fond de caisse compté").fill("10000");
  await page.getByLabel("Motif de l'écart").fill("Fond de départ");
  await page.getByRole("button", { name: "Ouvrir la caisse" }).click();
  await expect(page.getByText("Session de Kadi (caisse)")).toBeVisible();
  await expect(page.getByText("10 000 FCFA").first()).toBeVisible();
});

test("service à table : tournées, envoi, encaissement mixte et rendu monnaie", async ({ page }) => {
  await connexion(page, /Kadi/, "3333");
  await page.goto("/salle");
  await page.getByRole("button", { name: /^Table 4 Libre/ }).click();
  // Première tournée : 3 articles en 5 touches (table + 3 produits + Envoyer).
  await page.getByRole("tab", { name: /Bières/ }).click();
  const biere = page.getByRole("button", { name: /^Bière blonde \d/ });
  await biere.click();
  await biere.click();
  await biere.click();
  await page.getByRole("button", { name: /^Envoyer \(3\)/ }).click();
  await expect(page.locator(".ligne.envoyee")).toHaveCount(1);
  // Deuxième tournée sur la même addition.
  await page.getByRole("tab", { name: /Grillades/ }).click();
  await page.getByRole("button", { name: /^Brochettes \(3\) \d/ }).click();
  await page.getByRole("button", { name: /^Brochettes \(3\) \d/ }).click();
  await page.getByRole("button", { name: /^Envoyer \(2\)/ }).click();
  await expect(page.locator(".total").getByText("6 000 FCFA")).toBeVisible();
  // Paiement mixte : 2 000 espèces + 4 000 Orange Money.
  await page.getByRole("button", { name: /^Encaisser 6 000/ }).click();
  await page.getByRole("button", { name: /Espèces$/ }).first().click();
  const montants = page.getByLabel("Montant", { exact: true });
  await montants.first().fill("2000");
  await page.getByRole("button", { name: /Orange Money/ }).click();
  await expect(montants.nth(1)).toHaveValue("4 000");
  // Référence Mobile Money obligatoire.
  await expect(page.getByText("Saisissez la référence de la transaction Mobile Money")).toBeVisible();
  await expect(page.getByRole("button", { name: "Valider le paiement" })).toBeDisabled();
  await page.getByLabel("Référence de la transaction").fill("PP260314.1830.A12345");
  await page.getByLabel("Espèces reçues du client").fill("5000");
  await expect(page.getByText("Rendu : 3 000 FCFA")).toBeVisible();
  await page.getByRole("button", { name: "Valider le paiement" }).click();
  await expect(page.getByText(/Reçu n°\d+/)).toBeVisible();
  await expect(page.getByText("Rendre : 3 000 FCFA")).toBeVisible();
  await page.getByRole("button", { name: "Terminé" }).click();
  await expect(page.getByRole("button", { name: /^Table 4 Libre/ })).toBeVisible();
});

test("écran cuisine : les envois arrivent par poste et passent à « prêt »", async ({ page }) => {
  await connexion(page, /Moussa/, "5555");
  await page.goto("/cuisine");
  await page.getByRole("tab", { name: "Grill" }).click();
  // Le grill ne voit que ses envois (les bières partent au bar).
  await expect(page.locator(".carte-cuisine")).toHaveCount(1);
  const carte = page.locator(".carte-cuisine").filter({ hasText: "TABLE 4" });
  await expect(carte.getByText("Brochettes (3)")).toBeVisible();
  await expect(carte).not.toContainText("Bière");
  await carte.getByRole("button", { name: "Prêt ✓" }).click();
  await expect(carte.getByRole("button", { name: "Servi" })).toBeVisible();
});

test("annulation après envoi : motif puis PIN du gérant", async ({ page }) => {
  await connexion(page, /Awa/, "4444");
  await page.goto("/salle");
  await page.getByRole("button", { name: /^Table 7 Libre/ }).click();
  await page.getByRole("tab", { name: /Grillades/ }).click();
  await page.getByRole("button", { name: /^Poulet braisé \d/ }).click();
  await page.getByRole("button", { name: /^Envoyer \(1\)/ }).click();
  await page.locator(".ligne.envoyee .ligne-bouton").click();
  await page.getByRole("button", { name: /^Annuler/ }).click();
  await page.getByRole("button", { name: "Client a changé d'avis" }).click();
  await page.getByRole("button", { name: "Confirmer" }).click();
  // La serveuse n'a pas le droit : le PIN d'un responsable est demandé.
  await expect(page.getByRole("dialog", { name: "PIN du responsable" })).toBeVisible();
  await taperPin(page, "2222");
  await page.getByRole("button", { name: "Autoriser" }).click();
  await expect(page.getByText("Article annulé")).toBeVisible();
  await expect(page.locator(".total").getByText("0 FCFA")).toBeVisible();
});

test("poste central injoignable : la saisie en cours est conservée", async ({ page }) => {
  await connexion(page, /Awa/, "4444");
  await page.goto("/salle");
  await page.getByRole("button", { name: /^Table 2 Libre/ }).click();
  await page.getByRole("tab", { name: /Boissons/ }).click();
  await page.getByRole("button", { name: /^Coca-Cola \d/ }).click();
  await page.getByRole("button", { name: /^Coca-Cola \d/ }).click();
  // Coupure du poste central.
  await page.route("**/api/**", (r) => r.abort("connectionrefused"));
  await page.getByRole("button", { name: /^Envoyer \(2\)/ }).click();
  await expect(page.getByText(/Poste central injoignable/).first()).toBeVisible();
  await expect(page.locator(".ligne.panier")).toHaveCount(1);
  // Même après rechargement de la page, le panier est toujours là.
  await page.unroute("**/api/**");
  await page.reload();
  await expect(page.locator(".ligne.panier")).toContainText("2×");
  await page.getByRole("button", { name: /^Envoyer \(2\)/ }).click();
  await expect(page.locator(".ligne.envoyee")).toHaveCount(1);
  await expect(page.locator(".ligne.panier")).toHaveCount(0);
});

test("employé sans contrat, payé à la journée : seul le nom et le taux sont demandés", async ({ page }) => {
  await connexion(page, /Adama/, "2222");
  await page.goto("/employes");
  await page.getByRole("button", { name: "+ Employé" }).click();
  await page.getByLabel("Nom", { exact: true }).fill("Bakary Diallo");
  await page.getByLabel("Rémunération").selectOption("journalier");
  await page.getByLabel("Paye par jour travaillé").fill("2500");
  await page.getByRole("button", { name: "Enregistrer" }).click();
  await expect(page.getByText("Employé enregistré")).toBeVisible();
  const ligne = page.getByRole("row").filter({ hasText: "Bakary Diallo" });
  await expect(ligne).toContainText("Sans contrat");
  await expect(ligne).toContainText("Journalier");
  // Pointage du jour puis aperçu de paie : jours présents × taux.
  await page.getByRole("tab", { name: "Présences du jour" }).click();
  await page.locator(".ligne-pointage").filter({ hasText: "Bakary Diallo" }).getByRole("button", { name: "Présent" }).click();
  await page.goto("/paie");
  await expect(page.getByRole("row").filter({ hasText: "Bakary Diallo" })).toContainText("2 500 FCFA");
});

test("cotisations INPS/AMO : désactivées par défaut, activables", async ({ page }) => {
  await connexion(page, /Mariam/, "1234");
  await page.goto("/administration");
  await page.getByRole("tab", { name: "Restaurant et règles" }).click();
  const inps = page.getByRole("checkbox", { name: "Prélever la cotisation INPS" });
  await expect(inps).not.toBeChecked();
  await expect(page.getByRole("checkbox", { name: "Prélever la cotisation AMO" })).not.toBeChecked();
  await inps.check();
  await expect(page.getByLabel("INPS part salarié (%)")).toHaveValue("3,60");
});

test("tableau de bord : chiffres de la journée avec leurs formules", async ({ page }) => {
  await connexion(page, /Adama/, "2222");
  await page.goto("/tableau-de-bord");
  const ca = page.locator(".indicateur").filter({ hasText: "Chiffre d'affaires" });
  await expect(ca).toContainText("6 000 FCFA");
  await ca.locator("summary").click();
  await expect(ca.locator(".formule")).toContainText("CA =");
  await expect(page.getByText(/paiement\(s\) Mobile Money à vérifier/)).toBeVisible();
});

test("clôture de caisse avec billetage et écart motivé", async ({ page }) => {
  await connexion(page, /Kadi/, "3333");
  await page.goto("/caisse");
  await page.getByRole("button", { name: "🔒 Clôturer ma caisse" }).click();
  // Attendu : 10 000 + 2 000 = 12 000. On compte 11 000 : écart au-delà du seuil, motif obligatoire.
  await page.getByLabel("Nombre de 10000", { exact: true }).fill("1");
  await page.getByLabel("Nombre de 1000", { exact: true }).fill("1");
  await expect(page.getByRole("dialog").getByText("-1 000 FCFA")).toBeVisible();
  const cloturer = page.getByRole("button", { name: "Clôturer", exact: true });
  await expect(cloturer).toBeDisabled();
  await page.getByLabel("Motif de l'écart (obligatoire)").fill("Erreur de rendu");
  await cloturer.click();
  await expect(page.getByRole("dialog", { name: "Rapport de clôture (Z)" })).toContainText("RAPPORT Z");
});

test("interface utilisable sur le téléphone d'un serveur", async ({ browser }) => {
  const contexte = await browser.newContext({ viewport: { width: 390, height: 780 }, isMobile: true, hasTouch: true });
  const page = await contexte.newPage();
  await connexion(page, /Awa/, "4444");
  await page.goto("/salle");
  await expect(page.getByRole("button", { name: /^Table 1 / })).toBeVisible();
  // Pas de débordement horizontal.
  const largeur = await page.evaluate(() => document.documentElement.scrollWidth);
  expect(largeur).toBeLessThanOrEqual(390);
  await page.getByRole("button", { name: "Menu" }).click();
  await expect(page.getByRole("navigation", { name: "Menu principal" })).toBeVisible();
  await contexte.close();
});
