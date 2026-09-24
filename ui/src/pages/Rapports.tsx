import { useState } from "react";
import { get, telecharger } from "../api";
import { Champ, Onglets, TableauDonnees } from "../composants/Base";
import { useApp, useDonnees } from "../contexte";
import { aujourdhui, dateFr, fcfa, nombre, premierDuMois } from "../format";
import type { Rapport } from "../types";

const COLONNES_TEXTE = /^(Journée|Produit|Catégorie|Serveur|Type|Moyen|Compte|Heure|Article|Motif|Par|Autorisé|Perte|Date|Fournisseur|Mode|Libellé|Client|Téléphone|Ancienneté|Caissier|Employé|N°|Commande|Unité|Nombre|Sessions|Quantité|Qté|Seuil)/;

function cellule(colonne: string, v: string | number | null) {
  if (v === null) return "";
  if (typeof v === "number") return COLONNES_TEXTE.test(colonne) ? nombre(v) : fcfa(v);
  return v;
}

/** Rapport lisible et imprimable, avec les formules (RG-RAP-01). */
export function AffichageRapport({ r }: { r: Rapport }) {
  return (
    <div className="imprimable rapport">
      <h2>
        {r.titre} {r.debut && `— du ${dateFr(r.debut)} au ${dateFr(r.fin)}`}
      </h2>
      <div className="indicateurs">
        {r.indicateurs.map((i) => (
          <details key={i.cle} className="indicateur" open>
            <summary>
              <span>{i.libelle}</span>
              <strong>{i.cle === "nb_commandes" ? nombre(i.valeur) : fcfa(i.valeur)}</strong>
            </summary>
            <p className="formule">{i.formule}</p>
          </details>
        ))}
      </div>
      {r.tableaux.map((t) => (
        <section key={t.titre}>
          <h3>{t.titre}</h3>
          {t.lignes.length === 0 ? (
            <p className="aide">Rien sur la période.</p>
          ) : (
            <TableauDonnees colonnes={t.colonnes} lignes={t.lignes.map((l) => l.map((v, j) => cellule(t.colonnes[j], v)))} />
          )}
          {t.formule && <p className="formule">{t.formule}</p>}
        </section>
      ))}
    </div>
  );
}

export default function Rapports() {
  const { notifier } = useApp();
  const [onglet, setOnglet] = useState<"periode" | "stock" | "dettes">("periode");
  const [debut, setDebut] = useState(premierDuMois());
  const [fin, setFin] = useState(aujourdhui());
  const chemin = onglet === "periode" ? `/rapports/periode?debut=${debut}&fin=${fin}` : `/rapports/${onglet}`;
  const { donnees } = useDonnees(() => get<Rapport>(chemin), [], [chemin]);
  return (
    <div>
      <h1>Rapports</h1>
      <Onglets
        onglets={[
          { cle: "periode", libelle: "Activité" },
          { cle: "stock", libelle: "Stock et valeur" },
          { cle: "dettes", libelle: "Dettes" },
        ]}
        actif={onglet}
        changer={setOnglet}
      />
      <div className="carte filtres non-imprime">
        {onglet === "periode" && (
          <>
            <Champ libelle="Du (journée d'exploitation)" type="date" valeur={debut} changer={setDebut} />
            <Champ libelle="Au" type="date" valeur={fin} changer={setFin} />
            <button onClick={() => { setDebut(aujourdhui()); setFin(aujourdhui()); }}>Aujourd'hui</button>
          </>
        )}
        <button onClick={() => window.print()}>Imprimer / PDF</button>
        <button
          onClick={() =>
            telecharger(`${chemin}${chemin.includes("?") ? "&" : "?"}format=csv`, `youma-${onglet}.csv`).catch((e) => notifier(e.message, "erreur"))
          }
        >
          Export Excel (CSV)
        </button>
      </div>
      {donnees ? <AffichageRapport r={donnees} /> : <p className="aide">Chargement…</p>}
    </div>
  );
}
