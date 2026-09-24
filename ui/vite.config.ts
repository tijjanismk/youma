import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

// En développement, l'API est servie par youma-server (port 7878).
export default defineConfig({
  plugins: [react()],
  server: { proxy: { "/api": { target: "http://127.0.0.1:7878", ws: true } } },
  build: { target: "es2019", chunkSizeWarningLimit: 800 },
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["./src/test-setup.ts"],
    include: ["src/**/*.test.{ts,tsx}"],
  },
});
