// Position du livreur en arrière-plan (application Youma Livreur, fiche 0041) : service Android de premier plan,
// avec une notification « Course en cours » ; la position continue écran verrouillé, jusqu'à l'arrêt de la course.
import { registerPlugin } from "@capacitor/core";
import type { BackgroundGeolocationPlugin } from "@capacitor-community/background-geolocation";

const Geolocalisation = registerPlugin<BackgroundGeolocationPlugin>("BackgroundGeolocation");

/** Démarre le suivi ; renvoie la fonction d'arrêt. */
export async function suivreEnArrierePlan(position: (lat: number, lon: number) => void, erreur: (message: string) => void): Promise<() => void> {
  // Android 13 et plus : sans cette autorisation, la notification (et donc le suivi écran verrouillé) est refusée.
  try {
    const { LocalNotifications } = await import("@capacitor/local-notifications");
    await LocalNotifications.requestPermissions();
  } catch {
    /* refus : la position marche quand même tant que l'application reste devant */
  }
  const id = await Geolocalisation.addWatcher(
    {
      backgroundTitle: "Course en cours",
      backgroundMessage: "Votre position est envoyée au client jusqu'à la livraison.",
      requestPermissions: true,
      stale: false,
      distanceFilter: 15,
    },
    (p, e) => {
      if (e) {
        if (e.code === "NOT_AUTHORIZED") {
          erreur("Position refusée : autorisez la localisation « Toujours » pour Youma Livreur dans les réglages.");
          void Geolocalisation.openSettings();
        } else erreur(`Position indisponible : ${e.message}`);
        return;
      }
      if (p) position(p.latitude, p.longitude);
    },
  );
  return () => void Geolocalisation.removeWatcher({ id });
}
