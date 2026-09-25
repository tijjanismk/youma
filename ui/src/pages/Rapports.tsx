import { useState } from "react";
import { get, telecharger } from "../api";
import { Champ, Onglets, TableauDonnees } from "../composants/Base";
import { useApp, useDonnees } from "../contexte";
import { aujourdhui, dateFr, fcfa, nombre, premierDuMois } from "../format";
import { pourcentage } from "../recette";
import type { CoutMatiere, Rapport } from "../types";

const COLONNES_TEXTE = /^(Journée|Produit|Catégorie|Serveur|Type|Moyen|Compte|Heure|Article|Motif|Par|Autorisé|Perte|Date|Fournisseur|Mode|Libellé|Client|Téléphone|Ancienneté|Caissier|Employé|N°|Reçu n°|Commande|Unité|Nombre|Sessions|Quantité|Qté|Seuil)/;

function cellule(colonne: string, v: string | number | null) {
  if (v === null) return "";
  if (typeof v === "number") return COLONNES_TEXTE.test(colonne) ? nombre(v) : fcfa(v);
  return v;
}

/** nb_… : nombre ; …_pct : évolution en points de base (RG-STA-04) ; sinon FCFA. */
export function valeurIndicateur(cle: string, valeur: number): string {
  if (cle.startsWith("nb_")) return nombre(valeur);
  if (cle.endsWith("_pct")) return `${valeur > 0 ? "+" : valeur < 0 ? "−" : ""}${pourcentage(Math.abs(valeur))}`;
  return fcfa(valeur);
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
              <strong>{valeurIndicateur(i.cle, i.valeur)}</strong>
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
  const [onglet, setOnglet] = useState<"periode" | "statistiques" | "stock" | "dettes" | "cout-matiere">("periode");
  const [debut, setDebut] = useState(premierDuMois());
  const [fin, setFin] = useState(aujourdhui());
  const surPeriode = onglet === "periode" || onglet === "statistiques";
  const chemin = surPeriode ? `/rapports/${onglet}?debut=${debut}&fin=${fin}` : `/rapports/${onglet}`;
  const { donnees } = useDonnees(() => (onglet === "cout-matiere" ? Promise.resolve(null) : get<Rapport>(chemin)), [], [chemin]);
  return (
    <div>
      <h1>Rapports</h1>
      <Onglets
        onglets={[
          { cle: "periode", libelle: "Activité" },
          { cle: "statistiques", libelle: "Statistiques" },
          { cle: "stock", libelle: "Stock et valeur" },
          { cle: "dettes", libelle: "Dettes" },
          { cle: "cout-matiere", libelle: "Coût matière" },
        ]}
        actif={onglet}
        changer={setOnglet}
      />
      <div className="carte filtres non-imprime">
        {surPeriode && (
          <>
            <Champ libelle="Du (journée d'exploitation)" type="date" valeur={debut} changer={setDebut} />
            <Champ libelle="Au" type="date" valeur={fin} changer={setFin} />
            <button onClick={() => { setDebut(aujourdhui()); setFin(aujourdhui()); }}>Aujourd'hui</button>
          </>
        )}
        <button onClick={() => window.print()}>Imprimer / PDF</button>
        {onglet !== "cout-matiere" && (
          <button
            onClick={() =>
              telecharger(`${chemin}${chemin.includes("?") ? "&" : "?"}format=csv`, `youma-${onglet}.csv`).catch((e) => notifier(e.message, "erreur"))
            }
          >
            Export Excel (CSV)
          </button>
        )}
      </div>
      {onglet === "cout-matiere" ? <CoutsMatiere /> : donnees ? <AffichageRapport r={donnees} /> : <p className="aide">Chargement…</p>}
    </div>
  );
}

/** RG-REC-04 : coût matière des plats avec recette, avec la formule. */
function CoutsMatiere() {
  const { donnees } = useDonnees(() => get<CoutMatiere[]>("/rapports/cout-matiere"), ["catalogue", "stock"]);
  if (!donnees) return <p className="aide">Chargement…</p>;
  return (
    <div className="imprimable rapport">
      <h2>Coût matière des plats</h2>
      {donnees.length === 0 ? (
        <p className="aide">Aucun plat avec recette. Administration → Produits → « Recette ».</p>
      ) : (
        <TableauDonnees
          colonnes={["Produit", "Prix", "Coût matière", "Part du prix"]}
          lignes={donnees.map((c) => [c.nom, fcfa(c.prix), fcfa(c.cout), pourcentage(c.part_bp)])}
        />
      )}
      <p className="formule">Coût matière = Σ quantité × coût unitaire de l'article ; part = coût matière ÷ prix de vente.</p>
    </div>
  );
}
