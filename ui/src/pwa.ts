/**
 * Application installable (fiche 0020) : service worker, proposition d'installation, nouvelle version.
 * Tout est facultatif : sans HTTPS ni service worker, l'application marche comme un site ordinaire.
 */
import { useEffect, useState } from "react";

type InvitationInstallation = Event & { prompt: () => Promise<void>; userChoice: Promise<{ outcome: string }> };

let invitation: InvitationInstallation | null = null;
let activerNouvelleVersion: (() => void) | null = null;
const abonnes = new Set<() => void>();
const prevenir = () => abonnes.forEach((f) => f());

if (typeof window !== "undefined") {
  // L'événement arrive tôt : on le garde jusqu'à ce que l'utilisateur touche « Installer ».
  window.addEventListener("beforeinstallprompt", (e) => {
    e.preventDefault();
    invitation = e as InvitationInstallation;
    prevenir();
  });
  window.addEventListener("appinstalled", () => {
    invitation = null;
    prevenir();
  });
}

/** Enregistre le service worker (version compilée seulement : pas de cache pendant le développement). */
export function enregistrerServiceWorker() {
  if (!import.meta.env.PROD || !("serviceWorker" in navigator) || !window.isSecureContext) return;
  let rechargement = false;
  navigator.serviceWorker.addEventListener("controllerchange", () => {
    if (rechargement) location.reload();
  });
  navigator.serviceWorker
    .register("/sw.js")
    .then((reg) => {
      const signaler = (sw: ServiceWorker) => {
        activerNouvelleVersion = () => {
          rechargement = true;
          sw.postMessage("activer");
        };
        prevenir();
      };
      if (reg.waiting && navigator.serviceWorker.controller) signaler(reg.waiting);
      reg.addEventListener("updatefound", () => {
        const sw = reg.installing;
        sw?.addEventListener("statechange", () => {
          if (sw.state === "installed" && navigator.serviceWorker.controller) signaler(sw);
        });
      });
      // L'application reste ouverte toute la journée : on vérifie de temps en temps.
      setInterval(() => reg.update().catch(() => {}), 30 * 60 * 1000);
    })
    .catch(() => {});
}

export const estInstallee = () =>
  window.matchMedia?.("(display-mode: standalone)").matches || (navigator as Navigator & { standalone?: boolean }).standalone === true;

export const estIphone = () => /iphone|ipad|ipod/i.test(navigator.userAgent);

/** État de l'installation et de la mise à jour, pour l'affichage. */
export function usePwa() {
  const [, setTour] = useState(0);
  useEffect(() => {
    const f = () => setTour((n) => n + 1);
    abonnes.add(f);
    return () => {
      abonnes.delete(f);
    };
  }, []);
  return {
    installable: invitation !== null,
    installee: estInstallee(),
    installer: async () => {
      if (!invitation) return;
      await invitation.prompt();
      await invitation.userChoice.catch(() => null);
      invitation = null;
      prevenir();
    },
    nouvelleVersion: activerNouvelleVersion !== null,
    mettreAJour: () => activerNouvelleVersion?.(),
  };
}
