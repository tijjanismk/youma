import { expect, Page, test } from "@playwright/test";

// Tests d'interface de bout en bout : vrai poste central (base de démonstration), vrai navigateur.
// Les tests s'enchaînent sur la même journée, comme un vrai service.
test.describe.configure({ mode: "serial" });

// Bon de sortie du service à table, contrôlé plus loin.
let bonSortie: { numero: string; code: string } | null = null;

async function connexion(page: Page, nom: RegExp, pin: string) {
  await page.goto("/");
  const changer = page.getByRole("button", { name: "Changer d'utilisateur" });
  // Attendre que l'application ait chargé la session : connectée (changer) ou écran de connexion.
  await expect(changer.or(page.getByRole("button", { name: nom }))).toBeVisible();
  if (await changer.isVisible()) await changer.click();
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
  await expect(page.getByText("Argent reçu du client")).toBeVisible();
  await expect(page.locator(".resultat-paiement")).toContainText("5 000 FCFA");
  await expect(page.getByText("Monnaie à rendre : 3 000 FCFA")).toBeVisible();
  // Ticket de caisse = bon de sortie : numéro et code affichés.
  const bon = page.locator(".bon-sortie strong");
  bonSortie = { numero: (await bon.nth(0).textContent())!, code: (await bon.nth(1).textContent())! };
  expect(bonSortie.code).toMatch(/^[A-Z2-9]{4}$/);
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
  await page.getByRole("button", { name: /^Table T3 Libre/ }).click();
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

test("administration protégée par mot de passe ; cotisations INPS/AMO à taux manuels", async ({ page }) => {
  await connexion(page, /Mariam/, "1234");
  await page.goto("/administration");
  await page.getByRole("tab", { name: "Restaurant et règles" }).click();
  // Le PIN ne suffit pas (RG-AUT-06).
  await expect(page.getByText("Mot de passe requis")).toBeVisible();
  await page.getByRole("button", { name: "Saisir mon mot de passe" }).click();
  await page.getByLabel("Mot de passe", { exact: true }).fill("mauvais1");
  await page.getByRole("button", { name: "Confirmer" }).click();
  await expect(page.getByText("Mot de passe incorrect")).toBeVisible();
  await page.getByLabel("Mot de passe", { exact: true }).fill("baobab123");
  await page.getByRole("button", { name: "Confirmer" }).click();
  // Cotisations désactivées par défaut, taux vides à saisir.
  const inps = page.getByRole("checkbox", { name: "Prélever la cotisation INPS" });
  await expect(inps).not.toBeChecked();
  await expect(page.getByRole("checkbox", { name: "Prélever la cotisation AMO" })).not.toBeChecked();
  await inps.check();
  await expect(page.getByLabel("INPS part salarié (%)")).toHaveValue("0,00");
  await page.getByRole("button", { name: "Enregistrer les règles" }).click();
  await expect(page.getByText("Saisissez le taux de cotisation avant de l'activer")).toBeVisible();
  await page.getByLabel("INPS part salarié (%)").fill("3,6");
  await page.getByRole("button", { name: "Enregistrer les règles" }).click();
  await expect(page.getByText("Paramètres enregistrés")).toBeVisible();
  // Tous les opérateurs Mobile Money sont proposés.
  await page.getByRole("tab", { name: "Moyens de paiement" }).click();
  for (const op of ["Orange Money", "Moov Money", "Wave", "Sama Money"]) {
    await expect(page.getByRole("checkbox", { name: `${op} actif` })).toBeChecked();
  }
});

test("contrôle de sortie : le ticket payé est un bon de sortie, une seule fois", async ({ page }) => {
  expect(bonSortie).not.toBeNull();
  await connexion(page, /Awa/, "4444");
  await page.goto("/sortie");
  await page.getByLabel("N° du bon de sortie").fill(bonSortie!.numero);
  await page.getByLabel("Code de contrôle").fill(bonSortie!.code);
  await page.getByRole("button", { name: "Vérifier" }).click();
  await expect(page.getByText("PAYÉ — peut sortir")).toBeVisible();
  await expect(page.getByText("3 × Bière blonde")).toBeVisible();
  await page.getByRole("button", { name: "Ticket suivant" }).click();
  await page.getByLabel("N° du bon de sortie").fill(bonSortie!.numero);
  await page.getByLabel("Code de contrôle").fill(bonSortie!.code);
  await page.getByRole("button", { name: "Vérifier" }).click();
  await expect(page.getByText("DÉJÀ PRÉSENTÉ")).toBeVisible();
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

test("commandes à distance : QR sur la table, en ligne, zone à risque, validation et suivi", async ({ page, browser }) => {
  // Le propriétaire active les canaux, crée les QR des tables et une zone à risque.
  await connexion(page, /Mariam/, "1234");
  await page.goto("/administration");
  await page.getByRole("tab", { name: "Commandes à distance" }).click();
  await page.getByLabel("QR code sur les tables (le client commande depuis son téléphone)").check();
  await page.getByLabel("Commandes en ligne (livraison, à emporter)").check();
  await page.getByRole("button", { name: "Enregistrer", exact: true }).click();
  await page.getByLabel("Mot de passe", { exact: true }).fill("baobab123");
  await page.getByRole("button", { name: "Confirmer" }).click();
  await expect(page.getByText("Canaux enregistrés")).toBeVisible();
  await page.getByRole("button", { name: /^Créer les codes/ }).click();
  await expect(page.locator(".qr-table")).toHaveCount(17);
  await page.getByRole("button", { name: "+ Zone à risque" }).click();
  const zone = page.getByRole("dialog", { name: "Zone à risque" });
  await zone.getByLabel("Nom").fill("Kalaban toute la journée");
  await zone.getByLabel("Quartier").fill("Kalaban Coura");
  await zone.getByLabel("De (heure)").fill("00:00");
  await zone.getByLabel("À (heure)").fill("24:00");
  await zone.getByRole("button", { name: "Enregistrer" }).click();
  await expect(page.getByRole("cell", { name: "Kalaban toute la journée" })).toBeVisible();
  const legende = await page.locator(".qr-table").filter({ hasText: /— 5Scannez/ }).first().locator("figcaption").textContent();
  const code = /code ([A-Z2-9]{6})/.exec(legende ?? "")![1];

  // Le client, sans compte ni appairage, commande depuis sa table.
  const client = await browser.newContext({ viewport: { width: 390, height: 780 }, isMobile: true, hasTouch: true });
  const tel = await client.newPage();
  await tel.goto(`/menu?table=${code}`);
  await expect(tel.getByText("Table 5")).toBeVisible();
  await tel.getByRole("tab", { name: /Grillades/ }).click();
  await tel.getByRole("button", { name: "Ajouter Brochettes (3)" }).click();
  await tel.getByRole("button", { name: "Ajouter Brochettes (3)" }).click();
  await tel.getByRole("button", { name: /^Commander \(2\)/ }).click();
  await tel.getByRole("button", { name: "Envoyer la commande" }).click();
  await expect(tel).toHaveURL(/\/suivi\/[A-Z2-9]{8}$/);
  await expect(tel.locator("li[aria-current=step]")).toHaveText("Commande reçue");
  const suiviQr = tel.url();

  // Commande en ligne : zone bloquée, puis autre quartier.
  const web = await client.newPage();
  await web.goto("/menu");
  await web.getByRole("tab", { name: /Grillades/ }).click();
  await web.getByRole("button", { name: "Ajouter Brochettes (3)" }).click();
  await web.getByRole("button", { name: /^Commander \(1\)/ }).click();
  await web.getByLabel("Votre nom").fill("Fanta");
  await web.getByLabel("Votre téléphone").fill("76 11 22 33");
  await web.getByLabel("Quartier").selectOption("Kalaban Coura");
  await web.getByLabel("Point de repère").fill("Près du marché");
  await web.getByRole("button", { name: "Envoyer la commande" }).click();
  await expect(web.getByRole("alert")).toContainText("Kalaban toute la journée");
  await web.getByRole("button", { name: /^Commander \(1\)/ }).click();
  await web.getByLabel("Votre téléphone").fill("76 11 22 33");
  await web.getByLabel("Quartier").selectOption("Hamdallaye");
  await web.getByLabel("Point de repère").fill("Derrière la mosquée");
  await web.getByRole("button", { name: "Envoyer la commande" }).click();
  await expect(web).toHaveURL(/\/suivi\//);

  // La caissière voit les deux commandes, accepte celle de la table et refuse l'autre.
  await connexion(page, /Kadi/, "3333");
  await expect(page.getByRole("link", { name: /Commandes reçues/ }).first()).toContainText("2");
  await page.goto("/entrantes");
  const qr = page.locator(".entrante").filter({ hasText: "Table 5" });
  await expect(qr).toContainText("2 × Brochettes (3)");
  await qr.getByRole("button", { name: "Accepter et envoyer" }).click();
  const enLigne = page.locator(".entrante").filter({ hasText: "Hamdallaye" });
  await expect(enLigne).toContainText("nouveau client");
  await enLigne.getByRole("button", { name: "Refuser" }).click();
  await page.getByRole("button", { name: "Rupture" }).click();
  await page.getByRole("button", { name: "Confirmer" }).click();
  await expect(page.getByText("Aucune commande en attente")).toBeVisible();
  await page.goto("/salle");
  await expect(page.getByRole("button", { name: /^Table 5 Occupée/ })).toBeVisible();

  // Le client suit sa commande en direct.
  await tel.goto(suiviQr);
  await expect(tel.locator("li[aria-current=step]")).toHaveText("En préparation");
  await web.reload();
  await expect(web.getByText("Commande refusée")).toBeVisible();
  await expect(web.getByText("Rupture")).toBeVisible();
  await client.close();
});

test("recette d'un plat : coût matière calculé, rapport « Coût matière »", async ({ page }) => {
  await connexion(page, /Adama/, "2222");
  await page.goto("/administration");
  await page.getByRole("button", { name: "Recette de Frites" }).click();
  const d = page.getByRole("dialog", { name: "Recette — Frites" });
  // Démo : 250 g de pommes de terre (1 FCFA/g) + 30 ml d'huile (2 FCFA/ml) = 310 FCFA pour 750 FCFA.
  await expect(d).toContainText("Coût matière : 310 FCFA");
  await expect(d).toContainText("41,33 %");
  await expect(d.getByText(/Taille : Grande \(\+ 150 FCFA\)/)).toBeVisible();
  await d.getByLabel("Plat : quantité de Pommes de terre").fill("300");
  await expect(d).toContainText("Coût matière : 360 FCFA");
  await d.getByRole("button", { name: "Enregistrer la recette" }).click();
  await expect(page.getByText("Recette enregistrée")).toBeVisible();
  await page.goto("/rapports");
  await page.getByRole("tab", { name: "Coût matière" }).click();
  const ligne = page.getByRole("row", { name: /Frites/ });
  await expect(ligne).toContainText("360 FCFA");
  await expect(ligne).toContainText("48,00 %");
});

test("consignes : casiers reçus et vides rendus à la livraison, comptage", async ({ page }) => {
  await connexion(page, /Adama/, "2222");
  await page.goto("/achats");
  await page.getByLabel("Fournisseur").selectOption({ label: "Dépôt de boissons du quartier" });
  await page.getByLabel("Paiement").selectOption("credit");
  await page.getByText(/^Emballages consignés/).click();
  await page.getByLabel("Reçus : Casier bière (12)").fill("2");
  await page.getByLabel("Rendus : Casier bière (12)").fill("1");
  await expect(page.getByText("Emballages consignés (consigne 2 500 FCFA)")).toBeVisible();
  await page.getByRole("button", { name: "Enregistrer la réception" }).click();
  await expect(page.getByText("Réception enregistrée")).toBeVisible();

  await page.goto("/stock");
  await page.getByRole("tab", { name: /Consignes/ }).click();
  // Démo : 3 casiers ; +2 reçus −1 rendu = 4 détenus.
  const casier = page.getByRole("row", { name: /Casier bière \(12\)/ });
  await expect(casier.getByRole("cell").nth(2)).toHaveText("4");
  await page.getByRole("button", { name: "Compter les vides : Casier bière (12)" }).click();
  await page.getByLabel("Nombre compté").fill("4");
  await page.getByRole("button", { name: "Valider le comptage" }).click();
  await expect(page.getByText("Comptage conforme")).toBeVisible();
});

test("Mobile Money : relevé de l'opérateur importé, paiement vérifié d'un coup", async ({ page }) => {
  await connexion(page, /Adama/, "2222");
  await page.goto("/mobile-money");
  await page.getByRole("tab", { name: "Relevé de l'opérateur" }).click();
  await page.getByLabel("Compte").selectOption({ label: "Orange Money" });
  // Le paiement de la table 4 (4 000 FCFA, référence saisie au service) figure au relevé, avec une ligne inconnue.
  await page.getByLabel("Fichier du relevé").setInputFiles({
    name: "orange-money.csv",
    mimeType: "text/csv",
    buffer: Buffer.from("Référence;Montant;Expéditeur\nPP260314.1830.A12345;4 000;70112233\nINCONNU1;500;76000000\n"),
  });
  const bilan = page.getByLabel("Bilan du rapprochement");
  await expect(bilan).toContainText("1 paiement(s) vérifié(s)");
  await expect(bilan).toContainText("INCONNU1");
  await page.getByRole("tab", { name: "Tous" }).click();
  await expect(page.getByRole("row", { name: /PP260314\.1830\.A12345/ })).toContainText("Vérifié");
});

test("happy hour : le prix réduit s'affiche et s'applique à la saisie", async ({ page }) => {
  await connexion(page, /Adama/, "2222");
  await page.goto("/administration");
  await page.getByRole("tab", { name: "Promotions" }).click();
  await page.getByRole("button", { name: "+ Promotion" }).click();
  const d = page.getByRole("dialog", { name: "Promotion" });
  await d.getByLabel("Nom").fill("Coca à 500");
  await d.getByLabel("Produit").selectOption({ label: "Coca-Cola (750 FCFA)" });
  await d.getByLabel("Prix pendant la promotion").fill("500");
  await d.getByLabel("De (heure)").fill("00:00");
  await d.getByLabel("À (heure)").fill("24:00");
  await d.getByRole("button", { name: "Enregistrer" }).click();
  await expect(page.getByRole("cell", { name: "Coca à 500" })).toBeVisible();

  // Vente à emporter : indépendante des tables occupées par les tests précédents.
  await page.goto("/salle");
  await page.getByRole("button", { name: "+ Emporter / livraison" }).click();
  await page.getByRole("dialog").getByRole("button", { name: "Créer" }).click();
  await page.getByRole("tab", { name: /Boissons/ }).click();
  const coca = page.getByRole("button", { name: "Coca-Cola 500 FCFA" });
  await expect(coca).toContainText("Coca à 500");
  await coca.click();
  // À emporter : on paie d'abord ; le montant à encaisser est celui du happy hour.
  await expect(page.getByRole("button", { name: /^Encaisser 500 FCFA/ })).toBeVisible();
});
