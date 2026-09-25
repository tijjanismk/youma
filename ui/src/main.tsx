import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import "@fontsource/poppins/400.css";
import "@fontsource/poppins/500.css";
import "@fontsource/poppins/600.css";
import "@fontsource/poppins/700.css";
import "./styles.css";
import { enregistrerServiceWorker } from "./pwa";
import { appliquerTheme, lireTheme } from "./theme";

appliquerTheme(lireTheme());
enregistrerServiceWorker();

createRoot(document.getElementById("racine")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
