import { useEffect, useState } from "react";
import { get, post } from "../api";
import { Onglets, Vide } from "../composants/Base";
import { useApp, useDonnees } from "../contexte";
import { heure, minutesDepuis } from "../format";
import { t } from "../i18n";
import type { Catalogue, Ligne } from "../types";

type EnvoiCuisine = { id: string; titre: string; numero: number; poste_id: string; statut: string; message: string; cree_le: number; serveur: string | null; lignes: Ligne[] };
type Job = { id: string; poste_nom: string | null; destination: string; type: string; statut: string; tentatives: number; erreur: string | null; cree_le: number };

/** Écran cuisine / bar : lisible à distance, un envoi = une carte. */
export default function Cuisine() {
  const { agir } = useApp();
  const { donnees: cat } = useDonnees(() => get<Catalogue>("/catalogue"), []);
  const [poste, setPoste] = useState<string>(() => localStorage.getItem("youma.poste") ?? "tous");
  const { donnees: envois, recharger } = useDonnees(
    () => get<EnvoiCuisine[]>(`/cuisine${poste !== "tous" ? `?poste=${poste}` : ""}`),
    ["envoi", "envoi_pret", "commande"],
    [poste],
  );
  const { donnees: jobs, recharger: rechargerJobs } = useDonnees(() => get<Job[]>("/impressions"), ["impression"]);
  const [, forcer] = useState(0);
  useEffect(() => {
    const i = setInterval(() => forcer((x) => x + 1), 30_000);
    return () => clearInterval(i);
  }, []);

  const changer = (id: string, statut: string, message = "") => agir((pin) => post(`/envois/${id}/statut`, { statut, message }, pin)).then(recharger);
  const enErreur = (jobs ?? []).filter((j) => j.statut === "erreur" && j.tentatives < 99);

  return (
    <div className="cuisine">
      <Onglets
        onglets={[{ cle: "tous", libelle: "Tous les postes" }, ...(cat?.postes ?? []).filter((p) => p.actif).map((p) => ({ cle: p.id, libelle: p.nom }))]}
        actif={poste}
        changer={(p) => {
          setPoste(p);
          localStorage.setItem("youma.poste", p);
        }}
      />
      {enErreur.length > 0 && (
        <div className="bandeau erreur">
          {enErreur.length} ticket(s) non imprimé(s) :
          {enErreur.map((j) => (
            <span key={j.id}>
              {" "}
              {j.poste_nom ?? "caisse"} ({j.erreur})
              <button
                className="petit"
                onClick={() => {
                  const dest = prompt("Réimprimer vers (ex. tcp:192.168.1.60:9100) — vide = même imprimante", "") ?? null;
                  if (dest !== null) agir((pin) => post(`/impressions/${j.id}/reimprimer`, { destination: dest || null }, pin), "Ticket renvoyé").then(rechargerJobs);
                }}
              >
                Réimprimer
              </button>
            </span>
          ))}
        </div>
      )}
      {envois && envois.length === 0 && <Vide>Aucune commande en attente.</Vide>}
      <div className="cartes-cuisine">
        {(envois ?? []).map((e) => {
          const attente = minutesDepuis(e.cree_le);
          return (
            <div key={e.id} className={`carte-cuisine ${e.statut} ${attente >= 20 ? "retard" : ""}`}>
              <div className="carte-cuisine-entete">
                <strong>{e.titre}</strong>
                <span>
                  Envoi n°{e.numero} — {heure(e.cree_le)} — <b>{attente} min</b>
                </span>
                {e.serveur && <small>Serveur : {e.serveur}</small>}
              </div>
              <ul>
                {e.lignes
                  .filter((l) => l.quantite > l.quantite_annulee)
                  .map((l) => (
                    <li key={l.id}>
                      <span className="qte">{l.quantite - l.quantite_annulee} ×</span> {l.libelle}
                      {l.options.length > 0 && <div className="note">+ {l.options.map((o) => o.nom).join(", ")}</div>}
                      {l.commentaire && <div className="note">« {l.commentaire} »</div>}
                    </li>
                  ))}
              </ul>
              {e.message && <p className="attention-texte">{e.message}</p>}
              <div className="actions-cuisine">
                {e.statut === "recu" && (
                  <button className="grand" onClick={() => changer(e.id, "en_preparation")}>
                    En préparation
                  </button>
                )}
                {e.statut !== "pret" && (
                  <button className="principal grand" onClick={() => changer(e.id, "pret")}>
                    Prêt ✓
                  </button>
                )}
                {e.statut === "pret" && (
                  <button className="principal grand" onClick={() => changer(e.id, "servi")}>
                    Servi
                  </button>
                )}
                <button
                  onClick={() => {
                    const m = prompt("Problème à signaler au serveur (ex. plus de poulet)");
                    if (m) changer(e.id, "probleme", m);
                  }}
                >
                  Problème
                </button>
              </div>
              <small className="aide">{t(e.statut)}</small>
            </div>
          );
        })}
      </div>
    </div>
  );
}
