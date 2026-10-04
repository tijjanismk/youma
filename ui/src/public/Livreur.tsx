import { useEffect, useRef, useState } from "react";
import { ErreurApi, post } from "../api";
import { heure, versMicro } from "../format";
import { APPLI } from "../appli";
import { Page } from "./MenuClient";
import { messageErreurPosition, positionPossible } from "./position";
import { lireSessions } from "./MesCourses";

/**
 * Page du livreur (lien secret distinct du code du client) : envoie sa position pendant la course
 * pour le suivi en direct (RG-LIV-04). La géolocalisation exige HTTPS (serveur relais) ou le poste lui-même.
 */
export default function Livreur({ code }: { code: string }) {
  const [actif, setActif] = useState(false);
  const [dernier, setDernier] = useState<number | null>(null);
  const [message, setMessage] = useState("");
  const dernierEnvoi = useRef(0);
  const possible = positionPossible();
  // Écran maintenu allumé pendant la course : verrouillé, le navigateur coupe la position et le client ne suit plus.
  const [ecranAllume, setEcranAllume] = useState(false);

  useEffect(() => {
    if (!actif || !possible) return;
    const envoyer = async (lat: number, lon: number) => {
      // Au plus une position toutes les 15 s : économise le forfait et la batterie.
      if (Date.now() - dernierEnvoi.current < 15_000) return;
      dernierEnvoi.current = Date.now();
      try {
        await post(`/public/position/${encodeURIComponent(code)}`, { lat: versMicro(lat), lon: versMicro(lon) });
        setDernier(Date.now());
        setMessage("");
      } catch (e) {
        const m = e instanceof Error ? e.message : String(e);
        setMessage(m);
        // Course finie, lien inconnu ou trop d'essais (RG-CAN-08) : on arrête d'envoyer.
        if (m.includes("course") || (e instanceof ErreurApi && ["NON_TROUVE", "TROP_D_ESSAIS", "INTERDIT"].includes(e.code))) setActif(false);
      }
    };
    // Application Youma Livreur : position en arrière-plan, écran verrouillé compris (fiche 0041).
    if (APPLI === "livreur") {
      let arret: (() => void) | null = null;
      let fini = false;
      void import("../appli/positionArrierePlan")
        .then(({ suivreEnArrierePlan }) => suivreEnArrierePlan((lat, lon) => void envoyer(lat, lon), setMessage))
        .then((a) => (fini ? a() : (arret = a)))
        .catch((e) => setMessage(e instanceof Error ? e.message : String(e)));
      return () => {
        fini = true;
        arret?.();
      };
    }
    const id = navigator.geolocation.watchPosition(
      (p) => void envoyer(p.coords.latitude, p.coords.longitude),
      (e) => setMessage(messageErreurPosition(e.code, "Le client ne voit plus votre position.")),
      { enableHighAccuracy: true, maximumAge: 10_000 },
    );
    return () => navigator.geolocation.clearWatch(id);
  }, [actif, possible, code]);

  useEffect(() => {
    // Navigateur seulement : l'application, elle, continue écran verrouillé.
    if (APPLI || !actif || !("wakeLock" in navigator)) return;
    let verrou: WakeLockSentinel | null = null;
    let fini = false;
    const demander = async () => {
      try {
        const v = await navigator.wakeLock.request("screen");
        if (fini) return void v.release();
        verrou = v;
        setEcranAllume(true);
        v.addEventListener("release", () => setEcranAllume(false));
      } catch {
        setEcranAllume(false); // batterie faible ou refus du navigateur
      }
    };
    void demander();
    // Le navigateur relâche le verrou quand la page passe en arrière-plan : on le reprend au retour.
    const retour = () => document.visibilityState === "visible" && void demander();
    document.addEventListener("visibilitychange", retour);
    return () => {
      fini = true;
      document.removeEventListener("visibilitychange", retour);
      void verrou?.release();
    };
  }, [actif]);

  return (
    <Page titre="Livraison en cours" sousTitre="Gardez cette page ouverte pendant la course">
      {!possible ? (
        <p className="attention-texte">
          Ce téléphone ne peut pas partager sa position sur cette adresse. Le suivi en direct passe par le serveur relais (adresse en https).
        </p>
      ) : actif ? (
        <div className="carte resultat-sortie ok" role="status">
          <h2>Position partagée</h2>
          <p>{dernier ? `Dernier envoi à ${heure(dernier)}` : "Recherche de la position…"}</p>
          {APPLI ? (
            <p className="aide">Vous pouvez verrouiller l'écran : la position est envoyée jusqu'à « Arrêter » ou la fin de la course.</p>
          ) : (
            <p className={ecranAllume ? "aide" : "attention-texte"}>
              {ecranAllume
                ? "L'écran reste allumé jusqu'à la fin de la course : le client suit votre position."
                : "Ne verrouillez pas l'écran et gardez cette page devant : sinon le client ne voit plus votre position."}
            </p>
          )}
          <button className="grand" onClick={() => setActif(false)}>
            Arrêter
          </button>
        </div>
      ) : (
        <>
          <button className="principal grand" onClick={() => setActif(true)}>
            Démarrer le suivi
          </button>
          {!APPLI && location.protocol === "https:" && (
            <p className="aide">
              Avec l'application Youma Livreur, la position continue écran verrouillé :{" "}
              <a href={`youma-livreur://course?relais=${encodeURIComponent(location.origin)}&code=${encodeURIComponent(code)}`}>ouvrir dans l'application</a>.
            </p>
          )}
        </>
      )}
      {message && <p className="erreur-texte">{message}</p>}
      {!APPLI && lireSessions().some((s) => s.relais === "") && (
        <p>
          <a href="/livreur">Mes courses</a>
        </p>
      )}
    </Page>
  );
}
