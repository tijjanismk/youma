import { MessageSquareWarning } from "lucide-react";
import { useState } from "react";
import { get, post } from "../api";
import { Champ, Modal, Onglets, TableauDonnees } from "../composants/Base";
import { useApp, useDonnees } from "../contexte";
import { dateHeure } from "../format";
import { t } from "../i18n";

type Avis = {
  id: string;
  commande_numero: number;
  canal: string;
  note: number;
  commentaire: string;
  recu_le: number;
  client: string;
  telephone: string | null;
  traite_le: number | null;
  suite: string | null;
};

/** Avis des clients (RG-AVI-01 à 03, fiche 0043) : les avis faibles attendent une suite du gérant. */
export function AvisClients() {
  const { agir, peut } = useApp();
  const [filtre, setFiltre] = useState<"a_traiter" | "tous">("a_traiter");
  const { donnees, recharger } = useDonnees(() => get<Avis[]>(`/avis${filtre === "a_traiter" ? "?a_traiter=1" : ""}`), ["avis"], [filtre]);
  const [enCours, setEnCours] = useState<Avis | null>(null);
  const [suite, setSuite] = useState("");
  return (
    <div className="carte">
      <h2>
        <MessageSquareWarning size={20} className="icone-texte" aria-hidden /> Avis des clients
      </h2>
      <Onglets
        onglets={[
          { cle: "a_traiter" as const, libelle: "Mécontents à rappeler" },
          { cle: "tous" as const, libelle: "Tous les avis" },
        ]}
        actif={filtre}
        changer={setFiltre}
      />
      <p className="aide">
        Note de 2 sur 5 ou moins : rappelez le client, puis notez la suite. Pour un geste : onglet « Cartes cadeaux », « Offrir un bon d'avoir ». Le rapport «
        Avis clients » (Rapports) donne la note moyenne par canal, livreur et serveur.
      </p>
      {donnees && donnees.length === 0 ? (
        <p className="aide">{filtre === "a_traiter" ? "Aucun avis faible en attente." : "Aucun avis pour l'instant."}</p>
      ) : (
        <TableauDonnees
          colonnes={["Reçu le", "N°", "Canal", "Note", "Commentaire", "Client", "Suite"]}
          lignes={(donnees ?? []).map((a) => [
            dateHeure(a.recu_le),
            String(a.commande_numero),
            t(a.canal),
            `${a.note} / 5`,
            a.commentaire || "—",
            [a.client, a.telephone].filter(Boolean).join(" — ") || "—",
            a.suite ??
              (a.note <= 2 && peut("commande.offrir") ? (
                <button
                  className="petit"
                  onClick={() => {
                    setSuite("");
                    setEnCours(a);
                  }}
                >
                  Noter la suite
                </button>
              ) : (
                "—"
              )),
          ])}
        />
      )}
      {enCours && (
        <Modal titre={`Avis sur la commande n°${enCours.commande_numero}`} fermer={() => setEnCours(null)}>
          <p>
            {enCours.note} / 5 — {enCours.commentaire || "sans commentaire"}
          </p>
          <Champ libelle="Suite donnée (appel, bon d'avoir, explication…)" valeur={suite} changer={setSuite} autoFocus />
          <button
            className="principal grand"
            disabled={!suite.trim()}
            onClick={() =>
              agir((pin) => post(`/avis/${enCours.id}/traiter`, { suite }, pin), "Suite enregistrée").then(() => {
                setEnCours(null);
                recharger();
              })
            }
          >
            Enregistrer
          </button>
        </Modal>
      )}
    </div>
  );
}
