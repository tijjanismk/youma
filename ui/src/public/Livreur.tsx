import { useEffect, useRef, useState } from "react";
import { post } from "../api";
import { heure, versMicro } from "../format";
import { Page } from "./MenuClient";
import { messageErreurPosition, positionPossible } from "./position";

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
    const id = navigator.geolocation.watchPosition(
      async (p) => {
        // Au plus une position toutes les 15 s : économise le forfait et la batterie.
        if (Date.now() - dernierEnvoi.current < 15_000) return;
        dernierEnvoi.current = Date.now();
        try {
          await post(`/public/position/${encodeURIComponent(code)}`, { lat: versMicro(p.coords.latitude), lon: versMicro(p.coords.longitude) });
          setDernier(Date.now());
          setMessage("");
        } catch (e) {
          const m = e instanceof Error ? e.message : String(e);
          setMessage(m);
          if (m.includes("course")) setActif(false);
        }
      },
      (e) => setMessage(messageErreurPosition(e.code, "Le client ne voit plus votre position.")),
      { enableHighAccuracy: true, maximumAge: 10_000 },
    );
    return () => navigator.geolocation.clearWatch(id);
  }, [actif, possible, code]);

  useEffect(() => {
    if (!actif || !("wakeLock" in navigator)) return;
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
          <p className={ecranAllume ? "aide" : "attention-texte"}>
            {ecranAllume
              ? "L'écran reste allumé jusqu'à la fin de la course : le client suit votre position."
              : "Ne verrouillez pas l'écran et gardez cette page devant : sinon le client ne voit plus votre position."}
          </p>
          <button className="grand" onClick={() => setActif(false)}>
            Arrêter
          </button>
        </div>
      ) : (
        <button className="principal grand" onClick={() => setActif(true)}>
          Démarrer le suivi
        </button>
      )}
      {message && <p className="erreur-texte">{message}</p>}
    </Page>
  );
}
