import { Check, MapPin, Pencil, Star } from "lucide-react";
import { useEffect, useState } from "react";
import { ErreurApi, get, post } from "../api";
import { APPLI } from "../appli";
import { dateHeure, distanceTexte, fcfa, lienCarte, minutesDepuis } from "../format";
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
    let etape = "";
    const charger = async () => {
      try {
        const v = await get<SuiviT>(`/public/suivi/${encodeURIComponent(code)}`);
        if (arret) return;
        // Application Youma Client : prévenir le client quand l'étape change (fiche 0041).
        if (APPLI === "client" && etape && v.etape !== etape) {
          const libelle = ETAPES.find((e) => e.cle === v.etape)?.libelle ?? "Commande mise à jour";
          void import("../appli/notifier").then(({ notifier }) => notifier(`Commande${v.numero ? ` n°${v.numero}` : ""}`, libelle));
        }
        etape = v.etape;
        setS(v);
        setErreur("");
        if (FINALES.includes(v.etape)) return;
        minuterie = setTimeout(charger, v.etape === "en_route" ? 10_000 : 20_000);
      } catch (e) {
        if (arret) return;
        setErreur(e instanceof Error ? e.message : String(e));
        // Code inconnu ou trop d'essais (RG-CAN-08) : réessayer ne servirait qu'à se faire bloquer.
        if (e instanceof ErreurApi && ["NON_TROUVE", "TROP_D_ESSAIS", "INTERDIT"].includes(e.code)) return;
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
              {i < rang && <Check size={16} className="icone-texte" aria-hidden />}
              {e.libelle}
            </li>
          ))}
        </ol>
      )}
      {(s.avis_possible || s.avis) && <AvisClient code={code} note={s.avis ?? null} donne={(n) => setS({ ...s, avis: n, avis_possible: false })} />}
      {(s.modifications_restantes ?? 0) > 0 && (
        <div className="carte">
          <a className="bouton" href={`/menu?modifier=${encodeURIComponent(code)}${s.code_table ? `&table=${encodeURIComponent(s.code_table)}` : ""}`}>
            <Pencil size={18} aria-hidden /> Modifier ma commande
          </a>
          <p className="aide">
            Possible tant que le restaurant ne l'a pas prise, {s.modifications_restantes === 1 ? "encore une fois" : `encore ${s.modifications_restantes} fois`}.
          </p>
        </div>
      )}
      {s.etape === "en_route" && s.livreur && (
        <div className="carte">
          <p>
            <MapPin size={16} className="icone-texte" aria-hidden /> Position du livreur il y a {minutesDepuis(s.livreur[2])} min
            {distance !== null && <> — environ {distanceTexte(distance)} de chez vous</>}
          </p>
          <a
            className="bouton"
            href={lienCarte(s.livreur[0], s.livreur[1])}
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
        {!APPLI && location.protocol === "https:" && (
          <p className="aide">
            Prévenu à chaque étape avec l'application Youma Client :{" "}
            <a href={`youma-client://suivi?relais=${encodeURIComponent(location.origin)}&code=${encodeURIComponent(code)}`}>ouvrir dans l'application</a>.
          </p>
        )}
      </div>
    </Page>
  );
}

/** RG-AVI-01 : avis du client sur sa commande terminée, une seule fois (fiche 0043). */
function AvisClient({ code, note, donne }: { code: string; note: number | null; donne: (n: number) => void }) {
  const [choix, setChoix] = useState(0);
  const [commentaire, setCommentaire] = useState("");
  const [erreur, setErreur] = useState("");
  const [envoi, setEnvoi] = useState(false);
  if (note)
    return (
      <div className="carte" role="status">
        <p>
          Merci pour votre avis : <strong>{note} / 5</strong>
        </p>
      </div>
    );
  return (
    <form
      className="carte avis-client"
      onSubmit={async (e) => {
        e.preventDefault();
        setEnvoi(true);
        setErreur("");
        try {
          await post(`/public/avis/${encodeURIComponent(code)}`, { note: choix, commentaire });
          donne(choix);
        } catch (err) {
          setErreur(err instanceof Error ? err.message : String(err));
        }
        setEnvoi(false);
      }}
    >
      <h2>Votre avis</h2>
      <div className="etoiles" role="radiogroup" aria-label="Note">
        {[1, 2, 3, 4, 5].map((n) => (
          <button
            key={n}
            type="button"
            role="radio"
            aria-checked={choix === n}
            aria-label={`${n} sur 5`}
            className={n <= choix ? "etoile choisie" : "etoile"}
            onClick={() => setChoix(n)}
          >
            <Star size={32} aria-hidden fill={n <= choix ? "currentColor" : "none"} />
          </button>
        ))}
      </div>
      <label className="champ">
        <span>{choix > 0 && choix <= 2 ? "Dites-nous ce qui n'a pas été (le restaurant vous recontactera)" : "Un commentaire (facultatif)"}</span>
        <textarea className="zone-texte" rows={3} maxLength={500} value={commentaire} onChange={(e) => setCommentaire(e.target.value)} />
      </label>
      <button className="principal grand" disabled={choix === 0 || envoi}>
        Envoyer mon avis
      </button>
      {erreur && <p className="erreur-texte">{erreur}</p>}
    </form>
  );
}
