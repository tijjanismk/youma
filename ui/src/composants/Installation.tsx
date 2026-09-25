import { Download, Share } from "lucide-react";
import { useEffect, useState } from "react";
import { get } from "../api";
import { estIphone, usePwa } from "../pwa";

type Reseau = { adresses_https?: string[]; certificat?: string | null };

/**
 * Proposition d'installer Youma comme une application (fiche 0020).
 * Sur le Wi-Fi du restaurant en HTTP, explique comment passer à l'adresse sécurisée.
 */
export function CarteInstallation({ reseauLocal = true }: { reseauLocal?: boolean }) {
  const { installable, installee, installer } = usePwa();
  const [reseau, setReseau] = useState<Reseau | null>(null);
  const nonSecurise = !window.isSecureContext;
  useEffect(() => {
    if (reseauLocal && nonSecurise)
      get<Reseau>("/reseau")
        .then(setReseau)
        .catch(() => setReseau(null));
  }, [reseauLocal, nonSecurise]);

  if (installee) return null;
  if (installable)
    return (
      <div className="carte installation">
        <div>
          <strong>Youma sur l'écran d'accueil</strong>
          <p className="aide">S'ouvre comme une application, en plein écran, sans barre d'adresse.</p>
        </div>
        <button className="principal" onClick={installer}>
          <Download size={20} /> Installer l'application
        </button>
      </div>
    );
  if (nonSecurise && reseauLocal) {
    const https = reseau?.adresses_https?.[0];
    if (!https) return null;
    return (
      <div className="carte installation">
        <div>
          <strong>Installer Youma sur ce téléphone</strong>
          <p className="aide">
            1. Une seule fois : <a href={reseau?.certificat ?? "/api/reseau/certificat"}>téléchargez le certificat du restaurant</a> puis installez-le (Android
            : Paramètres → Sécurité → Installer un certificat → Certificat CA ; iPhone : Réglages → Profil téléchargé, puis Réglages → Général → Informations →
            Réglages des certificats).
          </p>
          <p className="aide">
            2. Ouvrez l'adresse sécurisée <a href={https}>{https}</a>, connectez l'appareil, puis touchez « Installer l'application ».
          </p>
        </div>
      </div>
    );
  }
  if (estIphone())
    return (
      <div className="carte installation">
        <div>
          <strong>Youma sur l'écran d'accueil</strong>
          <p className="aide">
            Touchez <Share size={16} aria-label="Partager" /> puis « Sur l'écran d'accueil ».
          </p>
        </div>
      </div>
    );
  return null;
}

/** Bandeau « nouvelle version » : l'utilisateur choisit le moment (pas au milieu d'une commande). */
export function BandeauMiseAJour() {
  const { nouvelleVersion, mettreAJour } = usePwa();
  if (!nouvelleVersion) return null;
  return (
    <div className="bandeau info" role="status">
      Une nouvelle version de Youma est prête.{" "}
      <button className="petit" onClick={mettreAJour}>
        Mettre à jour
      </button>
    </div>
  );
}
