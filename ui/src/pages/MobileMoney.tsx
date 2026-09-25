import { useState } from "react";
import { get, post } from "../api";
import { Choix, Onglets, TableauDonnees } from "../composants/Base";
import { useApp, useDonnees } from "../contexte";
import { dateHeure, fcfa } from "../format";
import { t } from "../i18n";
import type { Compte } from "../types";

type Part = {
  part_id: string;
  compte: string;
  montant: number;
  reference: string | null;
  numero_payeur: string | null;
  horodatage: number;
  commande_numero: number | null;
  statut: string;
  caissier: string | null;
};

/** Paiements Mobile Money « à vérifier » : contrôle anti-fraude (faux SMS, captures). */
export default function MobileMoney() {
  const { agir } = useApp();
  const [filtre, setFiltre] = useState<"a_verifier" | "tous" | "releve">("a_verifier");
  const { donnees, recharger } = useDonnees(
    () => (filtre === "releve" ? Promise.resolve([] as Part[]) : get<Part[]>(`/mobile-money${filtre === "a_verifier" ? "?statut=a_verifier" : ""}`)),
    ["paiement"],
    [filtre],
  );
  const verifier = (id: string, statut: string) => {
    const note = statut === "rejete" ? (prompt("Pourquoi ? (introuvable sur le relevé…)") ?? "") : "";
    agir((pin) => post(`/mobile-money/${id}/verifier`, { statut, note }, pin), statut === "verifie" ? "Vérifié" : "Rejeté").then(recharger);
  };
  const total = (donnees ?? []).reduce((s, p) => s + p.montant, 0);
  return (
    <div>
      <h1>Mobile Money</h1>
      <p className="aide">Comparez chaque référence avec les SMS reçus sur le téléphone du restaurant ou le relevé de l'opérateur.</p>
      <Onglets
        onglets={[
          { cle: "a_verifier", libelle: "À vérifier" },
          { cle: "tous", libelle: "Tous" },
          { cle: "releve", libelle: "Relevé de l'opérateur" },
        ]}
        actif={filtre}
        changer={setFiltre}
      />
      {filtre === "releve" ? (
        <ReleveOperateur />
      ) : (
        <>
          <p>
            {donnees?.length ?? 0} paiement(s) — {fcfa(total)}
          </p>
          <TableauDonnees
            colonnes={["Date", "Opérateur", "Montant", "Référence", "Payeur", "Commande", "Caissier", "Statut", ""]}
            lignes={(donnees ?? []).map((p) => [
              dateHeure(p.horodatage),
              p.compte,
              fcfa(p.montant),
              <code>{p.reference ?? "—"}</code>,
              p.numero_payeur ?? "",
              p.commande_numero ?? "",
              p.caissier ?? "",
              t(p.statut),
              p.statut === "a_verifier" && (
                <span className="boutons-ligne">
                  <button className="petit principal" onClick={() => verifier(p.part_id, "verifie")}>
                    Reçu ✓
                  </button>
                  <button className="petit attention" onClick={() => verifier(p.part_id, "rejete")}>
                    Introuvable
                  </button>
                </span>
              ),
            ])}
          />
        </>
      )}
    </div>
  );
}

type Bilan = {
  lignes_lues: number;
  deja_importees: number;
  verifies: number;
  deja_verifies: number;
  ecarts: {
    reference: string;
    montant_releve: number;
    montant_caisse: number;
    commande_numero: number | null;
  }[];
  inconnues: {
    reference: string;
    montant: number;
    date: number | null;
    numero: string;
  }[];
  absents: Part[];
};

/**
 * Rapprochement par relevé (fiche 0016) : le fichier exporté de l'espace marchand (Orange Money, Moov, Wave…)
 * vérifie d'un coup les paiements dont la référence et le montant correspondent.
 */
function ReleveOperateur() {
  const { agir } = useApp();
  const { donnees: comptes } = useDonnees(() => get<Compte[]>("/comptes"), []);
  const { donnees: releves, recharger } = useDonnees(
    () =>
      get<
        {
          id: string;
          compte: string;
          nom_fichier: string;
          importe_le: number;
          lignes: number;
        }[]
      >("/mobile-money/releves"),
    [],
  );
  const mm = (comptes ?? []).filter((c) => c.type === "mobile_money" && c.actif);
  const [compte, setCompte] = useState("");
  const [bilan, setBilan] = useState<Bilan | null>(null);
  const compteId = compte || mm[0]?.id || "";
  const importer = async (f: File) => {
    const contenu = await f.text();
    const r = await agir((pin) => post<Bilan>("/mobile-money/releves", { compte_id: compteId, nom_fichier: f.name, contenu }, pin), "Relevé importé");
    if (r) {
      setBilan(r);
      recharger();
    }
  };
  return (
    <div>
      <div className="carte">
        <p className="aide">
          Exportez le relevé de l'espace marchand en CSV (colonnes référence et montant au minimum). Les paiements dont la référence et le montant correspondent
          passent en « vérifié ».
        </p>
        <Choix libelle="Compte" valeur={compteId} changer={setCompte} options={mm.map((c) => ({ valeur: c.id, libelle: c.nom }))} />
        <label className="champ">
          <span>Fichier du relevé (CSV)</span>
          <input type="file" accept=".csv,.txt,text/csv" aria-label="Fichier du relevé" onChange={(e) => e.target.files?.[0] && importer(e.target.files[0])} />
        </label>
        <button
          onClick={() =>
            agir((pin) => post<number>("/mobile-money/releves/relancer", { compte_id: compteId }, pin)).then(
              (n) => n !== undefined && alert(`${n} paiement(s) vérifié(s) grâce aux relevés déjà importés`),
            )
          }
        >
          Relancer le rapprochement
        </button>
      </div>
      {bilan && (
        <div className="carte" aria-label="Bilan du rapprochement">
          <h2>Bilan</h2>
          <p>
            {bilan.lignes_lues} ligne(s) lue(s) · <strong>{bilan.verifies} paiement(s) vérifié(s)</strong>
            {bilan.deja_verifies > 0 && ` · ${bilan.deja_verifies} déjà vérifié(s)`}
            {bilan.deja_importees > 0 && ` · ${bilan.deja_importees} ligne(s) déjà importée(s)`}
          </p>
          {bilan.ecarts.length > 0 && (
            <>
              <h3 className="attention-texte">Montants différents</h3>
              <TableauDonnees
                colonnes={["Référence", "Relevé", "Caisse", "Commande"]}
                lignes={bilan.ecarts.map((x) => [<code>{x.reference}</code>, fcfa(x.montant_releve), fcfa(x.montant_caisse), x.commande_numero ?? ""])}
              />
            </>
          )}
          {bilan.absents.length > 0 && (
            <>
              <h3 className="attention-texte">Saisis en caisse mais absents du relevé (SMS douteux ?)</h3>
              <TableauDonnees
                colonnes={["Date", "Montant", "Référence", "Commande", "Caissier"]}
                lignes={bilan.absents.map((p) => [
                  dateHeure(p.horodatage),
                  fcfa(p.montant),
                  <code>{p.reference}</code>,
                  p.commande_numero ?? "",
                  p.caissier ?? "",
                ])}
              />
            </>
          )}
          {bilan.inconnues.length > 0 && (
            <>
              <h3>Au relevé mais pas en caisse (paiement non saisi ou autre encaissement)</h3>
              <TableauDonnees
                colonnes={["Date", "Montant", "Référence", "Payeur"]}
                lignes={bilan.inconnues.map((l) => [l.date ? dateHeure(l.date) : "", fcfa(l.montant), <code>{l.reference}</code>, l.numero])}
              />
            </>
          )}
        </div>
      )}
      <h3>Relevés importés</h3>
      <TableauDonnees
        colonnes={["Importé le", "Compte", "Fichier", "Lignes"]}
        lignes={(releves ?? []).map((r) => [dateHeure(r.importe_le), r.compte, r.nom_fichier, r.lignes])}
      />
    </div>
  );
}
