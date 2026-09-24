// Démarre youma-server sur une base de démonstration neuve pour les tests d'interface.
// Usage : node e2e/lancer-serveur.mjs <port>
import { spawn, spawnSync } from "node:child_process";
import { existsSync, mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const ici = dirname(fileURLToPath(import.meta.url));
const racine = resolve(ici, "../..");
const port = process.argv[2] ?? "7979";
const binaire = join(racine, "target", "debug", process.platform === "win32" ? "youma-server.exe" : "youma-server");
const ui = resolve(ici, "../dist");

if (!existsSync(binaire)) {
  const r = spawnSync("cargo", ["build", "-p", "youma-server"], { cwd: racine, stdio: "inherit" });
  if (r.status !== 0) process.exit(r.status ?? 1);
}
if (!existsSync(join(ui, "index.html"))) {
  const r = spawnSync("npx", ["vite", "build"], { cwd: resolve(ici, ".."), stdio: "inherit", shell: true });
  if (r.status !== 0) process.exit(r.status ?? 1);
}

const donnees = mkdtempSync(join(tmpdir(), "youma-e2e-"));
const serveur = spawn(binaire, ["--demo", "--donnees", donnees, "--port", port, "--ui", ui], { stdio: "inherit" });
const arreter = () => serveur.kill();
process.on("SIGTERM", arreter);
process.on("SIGINT", arreter);
serveur.on("exit", (c) => process.exit(c ?? 0));
