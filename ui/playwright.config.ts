import { defineConfig } from "@playwright/test";

// Tests d'interface de bout en bout : vrai youma-server (base de démonstration) + interface compilée.
const PORT = 7979;

export default defineConfig({
  testDir: "./e2e",
  timeout: 60_000,
  workers: 1,
  retries: 0,
  use: {
    baseURL: `http://127.0.0.1:${PORT}`,
    viewport: { width: 1280, height: 800 },
    locale: "fr-FR",
    launchOptions: process.env.PLAYWRIGHT_CHROMIUM ? { executablePath: process.env.PLAYWRIGHT_CHROMIUM } : {},
  },
  webServer: {
    command: `node e2e/lancer-serveur.mjs ${PORT}`,
    url: `http://127.0.0.1:${PORT}/api/etat`,
    timeout: 120_000,
    reuseExistingServer: false,
  },
});
