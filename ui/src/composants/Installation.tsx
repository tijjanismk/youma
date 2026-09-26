import { Download, EllipsisVertical, Share } from "lucide-react";
import { estIphone, usePwa } from "../pwa";

/**
 * Youma sur l'écran d'accueil du téléphone. En HTTPS (relais, espace propriétaire) : vraie installation (fiche 0020).
 * Sur le Wi-Fi du restaurant, en HTTP et sans certificat à installer (fiche 0023) : un raccourci du navigateur.
 */
export function CarteInstallation() {
  const { installable, installee, installer } = usePwa();
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
  if (!window.isSecureContext)
    return (
      <div className="carte installation">
        <div>
          <strong>Youma sur l'écran d'accueil</strong>
          <p className="aide">
            Touchez <EllipsisVertical size={16} aria-label="Menu du navigateur" /> puis « Ajouter à l'écran d'accueil ».
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
