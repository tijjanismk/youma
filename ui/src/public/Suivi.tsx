import { MapPin } from "lucide-react";
import { useEffect, useState } from "react";
import { get } from "../api";
import { dateHeure, depuisMicro, fcfa, minutesDepuis } from "../format";
import type { Suivi as SuiviT } from "../types";
import { Page } from "./MenuClient";
import { distanceMetres } from "./panierClient";

const ETAPES: { cle: string; libelle: string }[] = [
  { cle: "recue", libelle: "Commande reçue" },
  { cle: "acceptee", libelle: "Acceptée par le restaurant" },
  { cle: "en_preparation", libelle: "En préparation" },
  { cle: "prete", libelle: "Prête" },
  { cle: "en_route", libelle: "Le livreur est en route" },
  { cle: "livree", libelle: "Livrée" },
];

const FINALES = ["livree", "refusee", "annulee", "echec"];

/** Suivi en direct de la commande par le client (le code de suivi fait office de clé). */
export default function Suivi({ code }: { code: string }) {
  const [s, setS] = useState<SuiviT | null>(null);
  const [erreur, setErreur] = useState("");
  useEffect(() => {
    let arret = false;
    let minuterie: ReturnType<typeof setTimeout>;
    const charger = async () => {
      try {
        const v = await get<SuiviT>(`/public/suivi/${encodeURIComponent(code)}`);
        if (arret) return;
        setS(v);
        setErreur("");
        if (FINALES.includes(v.etape)) return;
        minuterie = setTimeout(charger, v.etape === "en_route" ? 10_000 : 20_000);
      } catch (e) {
        if (arret) return;
        setErreur(e instanceof Error ? e.message : String(e));
        minuterie = setTimeout(charger, 30_000);
      }
    };
    charger();
    return () => {
      arret = true;
      clearTimeout(minuterie);
    };
  }, [code]);

  if (!s) return <Page titre="Suivi de commande">{erreur ? <p className="erreur-texte">{erreur}</p> : <p className="aide">Chargement…</p>}</Page>;
  const etapes = ETAPES.filter((e) => s.type === "livraison" || e.cle !== "en_route").map((e) => (e.cle === "livree" && s.type !== "livraison" ? { ...e, libelle: "Servie" } : e));
  const rang = etapes.findIndex((e) => e.cle === s.etape);
  const distance = s.livreur && s.destination ? distanceMetres([s.livreur[0], s.livreur[1]], s.destination) : null;
  return (
    <Page titre={s.restaurant} sousTitre={s.numero ? `Commande n°${s.numero}` : "Commande transmise au restaurant"}>
      {s.etape === "refusee" || s.etape === "annulee" || s.etape === "echec" ? (
        <div className="carte resultat-sortie ko" role="status">
          <h2>{s.etape === "echec" ? "Livraison non aboutie" : "Commande refusée"}</h2>
          {s.motif && <p>{s.motif}</p>}
          <p>Appelez le restaurant pour plus d'informations.</p>
        </div>
      ) : (
        <ol className="etapes-suivi" aria-label="Étapes de la commande">
          {etapes.map((e, i) => (
            <li key={e.cle} className={i < rang ? "faite" : i === rang ? "courante" : ""} aria-current={i === rang ? "step" : undefined}>
              {i < rang ? "✓ " : ""}
              {e.libelle}
            </li>
          ))}
        </ol>
      )}
      {s.etape === "en_route" && s.livreur && (
        <div className="carte">
          <p>
            <MapPin size={16} className="icone-texte" aria-hidden /> Position du livreur il y a {minutesDepuis(s.livreur[2])} min
            {distance !== null && <> — environ {distance < 1000 ? `${distance} m` : `${(distance / 1000).toFixed(1).replace(".", ",")} km`} de chez vous</>}
          </p>
          <a
            className="bouton"
            href={`https://www.openstreetmap.org/?mlat=${depuisMicro(s.livreur[0])}&mlon=${depuisMicro(s.livreur[1])}#map=16/${depuisMicro(s.livreur[0])}/${depuisMicro(s.livreur[1])}`}
            target="_blank"
            rel="noreferrer"
          >
            Voir sur la carte
          </a>
        </div>
      )}
      <div className="carte">
        {s.lignes.length === 0 && s.numero === 0 && <p className="aide">Le détail s'affichera dès que le restaurant aura reçu la commande.</p>}
        <ul className="lignes-entrante">
          {s.lignes.map(([q, l], i) => (
            <li key={i}>
              {q} × {l}
            </li>
          ))}
        </ul>
        <p>
          Total <strong>{fcfa(s.total)}</strong>
          {s.reste === 0 && s.total > 0 ? " — payé" : s.paiement_mode === "a_la_livraison" ? " — à payer à la livraison" : ""}
        </p>
        <p className="aide">Mis à jour le {dateHeure(s.mis_a_jour)} · cette page s'actualise toute seule.</p>
      </div>
    </Page>
  );
}
