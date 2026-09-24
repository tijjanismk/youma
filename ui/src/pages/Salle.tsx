import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { get, post } from "../api";
import { Case, Champ, Choix, Modal, Onglets, Vide } from "../composants/Base";
import { useApp, useDonnees } from "../contexte";
import { fcfa, minutesDepuis } from "../format";
import { t } from "../i18n";
import type { CommandeResume, Parametres, TablePlan, Zone } from "../types";

export default function Salle() {
  const { agir, etat } = useApp();
  const nav = useNavigate();
  const { donnees } = useDonnees(() => get<{ zones: Zone[]; tables: TablePlan[] }>("/salle"), ["commande", "table", "paiement", "envoi_pret", "envoi"]);
  const { donnees: autres } = useDonnees(() => get<CommandeResume[]>("/commandes?statut=ouverte"), ["commande", "paiement", "table"]);
  const [zone, setZone] = useState<string>("toutes");
  const [nouvelle, setNouvelle] = useState(false);

  if (!etat?.journee) return <Vide>Ouvrez la journée depuis l'accueil pour prendre des commandes.</Vide>;
  if (!donnees) return <p className="aide">Chargement…</p>;

  const ouvrirTable = (t: TablePlan) => {
    if (t.commande_id) return nav(`/commande/${t.commande_id}`);
    agir(async (pin) => {
      const id = await post<string>("/commandes", { type: "sur_place", table_id: t.id }, pin);
      nav(`/commande/${id}`);
    });
  };

  const tables = donnees.tables.filter((t) => zone === "toutes" || t.zone_id === zone);
  const horsTable = (autres ?? []).filter((c) => !c.table_nom);

  return (
    <div>
      <div className="titre-ligne">
        <h1>Salle</h1>
        <button className="principal" onClick={() => setNouvelle(true)}>
          + Emporter / livraison
        </button>
      </div>
      <Onglets
        onglets={[{ cle: "toutes", libelle: "Toutes" }, ...donnees.zones.filter((z) => z.actif).map((z) => ({ cle: z.id, libelle: z.nom }))]}
        actif={zone}
        changer={setZone}
      />
      <div className="plan-salle">
        {tables.map((tb) => (
          <button key={tb.id} className={`table-salle ${tb.statut}`} onClick={() => ouvrirTable(tb)} aria-label={`Table ${tb.nom} ${t(tb.statut)}`}>
            <strong>{tb.nom}</strong>
            <small>{t(tb.statut)}</small>
            {tb.commande_id && (
              <small>
                {tb.serveur ?? ""} · {tb.ouverte_le ? `${minutesDepuis(tb.ouverte_le)} min` : ""}
              </small>
            )}
            {tb.pretes > 0 && <span className="pastille pret">Prêt</span>}
            {tb.a_envoyer > 0 && <span className="pastille attente">À envoyer</span>}
          </button>
        ))}
      </div>
      {horsTable.length > 0 && (
        <>
          <h2>Comptoir, emporter, livraison</h2>
          <div className="liste-commandes">
            {horsTable.map((c) => (
              <button key={c.id} className="ligne-commande" onClick={() => nav(`/commande/${c.id}`)}>
                <strong>
                  n°{c.numero} — {t(c.type)}
                </strong>
                <span>{c.client_nom ?? c.serveur_nom ?? ""}</span>
                <span>{fcfa(c.reste)}</span>
              </button>
            ))}
          </div>
        </>
      )}
      {nouvelle && <NouvelleCommande fermer={() => setNouvelle(false)} />}
    </div>
  );
}

function NouvelleCommande({ fermer }: { fermer: () => void }) {
  const { agir, etat } = useApp();
  const nav = useNavigate();
  const [type, setType] = useState<"emporter" | "livraison">("emporter");
  const params: Parametres | undefined = etat?.parametres;
  const [quartier, setQuartier] = useState(params?.quartiers[0]?.nom ?? "");
  const [repere, setRepere] = useState("");
  const [telephone, setTelephone] = useState("");
  const telephoneActif = !!params?.canaux?.telephone;
  const [parTelephone, setParTelephone] = useState(telephoneActif);
  const creer = () =>
    agir(async (pin) => {
      const canal = telephoneActif && parTelephone ? "telephone" : undefined;
      const corps =
        type === "livraison" ? { type, canal, livraison: { quartier, repere, telephone } } : { type, canal };
      const id = await post<string>("/commandes", corps, pin);
      nav(`/commande/${id}`);
    });
  return (
    <Modal titre="Nouvelle commande" fermer={fermer}>
      <Onglets
        onglets={[
          { cle: "emporter", libelle: "À emporter" },
          { cle: "livraison", libelle: "Livraison" },
        ]}
        actif={type}
        changer={setType}
      />
      {type === "livraison" && (
        <>
          {params && params.quartiers.length > 0 ? (
            <Choix
              libelle="Quartier"
              valeur={quartier}
              changer={setQuartier}
              options={params.quartiers.map((q) => ({ valeur: q.nom, libelle: `${q.nom} (${fcfa(q.frais)})` }))}
            />
          ) : (
            <Champ libelle="Quartier" valeur={quartier} changer={setQuartier} obligatoire />
          )}
          <Champ libelle="Point de repère" valeur={repere} changer={setRepere} placeholder="Près de la pharmacie…" />
          <Champ libelle="Téléphone du client" valeur={telephone} changer={setTelephone} type="tel" obligatoire />
        </>
      )}
      {telephoneActif && <Case libelle="Commande reçue par téléphone" valeur={parTelephone} changer={setParTelephone} />}
      <div className="actions">
        <button onClick={fermer}>Annuler</button>
        <button className="principal" disabled={type === "livraison" && (!quartier || !telephone)} onClick={creer}>
          Créer
        </button>
      </div>
    </Modal>
  );
}
