import { expect, test } from "@playwright/test";

// Fiche 0020 : application installable (manifeste, icônes, service worker, ouverture sans réseau).
test.use({ serviceWorkers: "allow" });

test("manifeste de l'application : nom, plein écran, icônes", async ({ request }) => {
  for (const [chemin, debut] of [
    ["/manifest.webmanifest", "/"],
    ["/manifest-proprietaire.webmanifest", "/proprietaire"],
  ]) {
    const r = await request.get(chemin);
    expect(r.ok()).toBeTruthy();
    const m = await r.json();
    expect(m.display).toBe("standalone");
    expect(m.start_url).toBe(debut);
    expect(m.lang).toBe("fr");
    const tailles = m.icons.map((i: { sizes: string }) => i.sizes);
    expect(tailles).toContain("192x192");
    expect(tailles).toContain("512x512");
    expect(m.icons.some((i: { purpose?: string }) => i.purpose === "maskable")).toBeTruthy();
    for (const i of m.icons) {
      const icone = await request.get(i.src);
      expect(icone.headers()["content-type"]).toBe("image/png");
    }
  }
});

test("l'application s'ouvre même quand le réseau tombe (coquille en cache)", async ({ page, context }) => {
  await page.goto("/");
  await expect(page.getByRole("button", { name: /Mariam/ })).toBeVisible();
  // Le service worker prend la main sur la page.
  await page.evaluate(async () => {
    await navigator.serviceWorker.ready;
    if (!navigator.serviceWorker.controller) await new Promise((r) => navigator.serviceWorker.addEventListener("controllerchange", r, { once: true }));
  });
  const caches = await page.evaluate(async () => (await self.caches.keys()).filter((c) => c.startsWith("youma-")).length);
  expect(caches).toBe(1);

  await context.setOffline(true);
  await page.reload();
  // L'interface est là ; les données, elles, viennent toujours du poste central.
  await expect(page.getByText("Poste central injoignable")).toBeVisible();
  await context.setOffline(false);
  await expect(page.getByText("Poste central injoignable")).toBeHidden({ timeout: 20_000 });
});

test("rouvrir l'application sans Wi-Fi ne déconnecte pas l'utilisateur", async ({ page, context }) => {
  await page.goto("/");
  await page.getByRole("button", { name: /Mariam/ }).click();
  for (const c of "1234") await page.getByRole("button", { name: `Chiffre ${c}` }).click();
  await page.getByRole("button", { name: "Entrer" }).click();
  await expect(page.getByRole("button", { name: "Changer d'utilisateur" })).toBeVisible();
  await page.evaluate(async () => {
    await navigator.serviceWorker.ready;
    if (!navigator.serviceWorker.controller) await new Promise((r) => navigator.serviceWorker.addEventListener("controllerchange", r, { once: true }));
  });

  await context.setOffline(true);
  await page.reload();
  await expect(page.getByText("Poste central injoignable")).toBeVisible();
  await context.setOffline(false);
  // Le Wi-Fi revient : même session, sans retaper le PIN.
  await expect(page.getByRole("button", { name: "Changer d'utilisateur" })).toBeVisible({ timeout: 20_000 });
  await expect(page.getByText("Poste central injoignable")).toBeHidden({ timeout: 20_000 });
});
