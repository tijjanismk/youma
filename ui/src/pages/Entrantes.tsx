import { useState } from "react";
import { get, post } from "../api";
import { DemandeMotif, Vide } from "../composants/Base";
import { useApp, useDonnees } from "../contexte";
import { fcfa, minutesDepuis } from "../format";
import { t } from "../i18n";
import type { Entrante } from "../types";

/**
 * Commandes reçues sans passer par un serveur (QR sur la table, en ligne) : rien ne part en cuisine
 * avant qu'un membre du personnel ne les accepte (RG-CAN-02).
 */
export default function Entrantes() {
  const { agir } = useApp();
  const { donnees, recharger } = useDonnees(() => get<Entrante[]>("/entrantes"), ["commande_entrante", "commande"]);
  const [refus, setRefus] = useState<Entrante | null>(null);
  if (!donnees) return <p className="aide">Chargement…</p>;
  const accepter = (e: Entrante) => agir((pin) => post(`/entrantes/${e.commande.id}/valider`, { accepter: true }, pin), "Commande acceptée").then(recharger);
  return (
    <div>
      <h1>Commandes reçues</h1>
      {donnees.length === 0 && <Vide>Aucune commande en attente. Les commandes QR et en ligne apparaîtront ici.</Vide>}
      <div className="liste-entrantes">
        {donnees.map((e) => (
          <div key={e.commande.id} className={`carte entrante ${e.validation_responsable ? "risque" : ""}`} aria-label={`Commande reçue n°${e.commande.numero}`}>
            <div className="titre-ligne">
              <h2>
                n°{e.commande.numero} — {t(e.canal)}
                {e.table_demandee && ` · Table ${e.table_demandee}`}
              </h2>
              <span className="aide">il y a {minutesDepuis(e.commande.cree_le)} min</span>
            </div>
            {e.canal === "en_ligne" && (
              <p>
                <strong>{e.client_nom || "Client"}</strong> · <a href={`tel:${e.client_telephone}`}>{e.client_telephone}</a> ·{" "}
                {e.commandes_precedentes > 0 ? `${e.commandes_precedentes} commande(s) déjà servie(s)` : <span className="attention-texte">nouveau client</span>}
                <br />
                {t(e.commande.type)}
                {e.commande.livraison_quartier && ` — ${e.commande.livraison_quartier}, ${e.commande.livraison_repere ?? ""}`}
              </p>
            )}
            {e.verification_numero === "rappel" && e.canal === "en_ligne" && e.commandes_precedentes === 0 && (
              <p className="attention-texte">Rappelez le client pour confirmer le numéro avant d'accepter.</p>
            )}
            {e.validation_responsable && <p className="attention-texte">⚠️ {e.motif ?? "Zone à risque"} : accord d'un responsable.</p>}
            <ul className="lignes-entrante">
              {e.commande.lignes.map((l) => (
                <li key={l.id}>
                  {l.quantite} × {l.libelle}
                  {l.options.length > 0 && <small> ({l.options.map((o) => o.nom).join(", ")})</small>}
                  {l.commentaire && <small> — {l.commentaire}</small>}
                </li>
              ))}
            </ul>
            {e.commande.note && <p className="aide">Note : {e.commande.note}</p>}
            <p>
              Total <strong>{fcfa(e.commande.totaux.total)}</strong>
              {e.paiement_mode === "avance" && (
                <>
                  {" "}
                  · payé d'avance par {e.paiement_operateur} — réf. <strong>{e.paiement_reference}</strong> (vérifiez le SMS)
                </>
              )}
              {e.paiement_mode === "a_la_livraison" && " · paiement à la livraison"}
            </p>
            <div className="actions">
              <button className="attention grand" onClick={() => setRefus(e)}>
                Refuser
              </button>
              <button className="principal grand" onClick={() => accepter(e)}>
                Accepter et envoyer
              </button>
            </div>
          </div>
        ))}
      </div>
      {refus && (
        <DemandeMotif
          titre={`Refuser la commande n°${refus.commande.numero}`}
          suggestions={["Rupture", "Trop de monde", "Client injoignable", "Zone non desservie", "Commande douteuse"]}
          fermer={() => setRefus(null)}
          valider={(motif) =>
            agir((pin) => post(`/entrantes/${refus.commande.id}/valider`, { accepter: false, motif }, pin), "Commande refusée").then(() => {
              setRefus(null);
              recharger();
            })
          }
        />
      )}
    </div>
  );
}
