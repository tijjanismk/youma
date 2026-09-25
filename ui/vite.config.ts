import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { defineConfig, type Plugin } from "vitest/config";
import react from "@vitejs/plugin-react";

/**
 * Service worker de l'application installable (fiche 0020) : liste exacte des fichiers de la version
 * compilée, et un numéro de version qui change avec eux (l'ancien cache est alors supprimé).
 */
function serviceWorker(): Plugin {
  return {
    name: "youma-service-worker",
    apply: "build",
    enforce: "post",
    generateBundle(_options, bundle) {
      const publics = ["manifest.webmanifest", "manifest-proprietaire.webmanifest", "icone.svg", "icone-192.png", "icone-512.png", "apple-touch-icon.png"];
      // Polices : seulement les jeux latins (français, bambara) ; les autres sont chargées au besoin.
      const utiles = Object.keys(bundle).filter((f) => /\.(js|css|html)$/.test(f) || /-latin(-ext)?-\d+-normal-[\w-]+\.woff2$/.test(f));
      const fichiers = [...utiles, ...publics].sort();
      const empreinte = createHash("sha256");
      for (const f of utiles) {
        const b = bundle[f];
        empreinte.update(f).update(b.type === "chunk" ? b.code : typeof b.source === "string" ? b.source : Buffer.from(b.source));
      }
      const source = readFileSync(new URL("./sw/sw.js", import.meta.url), "utf8")
        .replace("__VERSION__", empreinte.digest("hex").slice(0, 12))
        .replace("__FICHIERS__", JSON.stringify(fichiers));
      this.emitFile({ type: "asset", fileName: "sw.js", source });
    },
  };
}

// En développement, l'API est servie par youma-server (port 7878).
export default defineConfig({
  plugins: [react(), serviceWorker()],
  server: { proxy: { "/api": { target: "http://127.0.0.1:7878", ws: true } } },
  build: { target: "es2019", chunkSizeWarningLimit: 800 },
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["./src/test-setup.ts"],
    include: ["src/**/*.test.{ts,tsx}"],
  },
});
