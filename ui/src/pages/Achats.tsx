import { useState } from "react";
import { get, post } from "../api";
import { Champ, ChampMontant, Choix, Modal, Montant, Onglets, TableauDonnees } from "../composants/Base";
import { useApp, useDonnees } from "../contexte";
import { dateHeure, fcfa } from "../format";
import { consigneNette, consignesSaisies } from "../consigne";
import type { Compte, ConsigneAchat, EtatEmballage, NiveauStock } from "../types";

type Fournisseur = { id: string; nom: string; telephone: string; notes: string; actif: boolean; dette: number };
type Achat = { id: string; numero: number; fournisseur: string | null; mode: string; total: number; horodatage: number; lignes: [string, number, number, number][] };
type LigneSaisie = { article_id: string; conditionnement_id: string; quantite: number; prix_total: number };

export default function Achats() {
  const [onglet, setOnglet] = useState<"reception" | "historique" | "fournisseurs">("reception");
  return (
    <div>
      <h1>Achats</h1>
      <Onglets
        onglets={[
          { cle: "reception", libelle: "Réception" },
          { cle: "historique", libelle: "Historique" },
          { cle: "fournisseurs", libelle: "Fournisseurs et dettes" },
        ]}
        actif={onglet}
        changer={setOnglet}
      />
      {onglet === "reception" && <Reception />}
      {onglet === "historique" && <Historique />}
      {onglet === "fournisseurs" && <Fournisseurs />}
    </div>
  );
}

function Reception() {
  const { agir } = useApp();
  const { donnees: articles } = useDonnees(() => get<NiveauStock[]>("/stock"), []);
  const { donnees: fournisseurs } = useDonnees(() => get<Fournisseur[]>("/fournisseurs"), []);
  const { donnees: comptes } = useDonnees(() => get<Compte[]>("/comptes"), []);
  const [fournisseur, setFournisseur] = useState("");
  const [mode, setMode] = useState("comptant");
  const [compte, setCompte] = useState("");
  const [lignes, setLignes] = useState<LigneSaisie[]>([]);
  const [note, setNote] = useState("");
  // Emballages consignés reçus et vides rendus avec la livraison (fiche 0015).
  const { donnees: emballages } = useDonnees(() => get<{ emballages: EtatEmballage[] }>("/emballages").then((d) => d.emballages).catch(() => [] as EtatEmballage[]), []);
  const [consignes, setConsignes] = useState<ConsigneAchat[]>([]);
  const valeurs = Object.fromEntries((emballages ?? []).map((e) => [e.emballage.id, e.emballage.valeur]));
  const saisies = consignesSaisies(consignes);
  const nette = consigneNette(saisies, valeurs);
  const majConsigne = (id: string, v: Partial<ConsigneAchat>) =>
    setConsignes((c) => [...c.filter((x) => x.emballage_id !== id), { ...(c.find((x) => x.emballage_id === id) ?? { emballage_id: id, recus: 0, rendus: 0 }), ...v }]);
  const consigneDe = (id: string) => consignes.find((x) => x.emballage_id === id) ?? { emballage_id: id, recus: 0, rendus: 0 };
  const total = lignes.reduce((s, l) => s + l.prix_total, 0) + nette;
  const maj = (i: number, l: Partial<LigneSaisie>) => setLignes((x) => x.map((a, j) => (j === i ? { ...a, ...l } : a)));
  return (
    <div className="carte">
      <p className="aide">Achat au marché sans bon préalable : laissez « Marché » comme fournisseur.</p>
      <div className="grille-2">
        <Choix libelle="Fournisseur" valeur={fournisseur} changer={setFournisseur} options={[{ valeur: "", libelle: "Marché (sans fournisseur)" }, ...(fournisseurs ?? []).map((f) => ({ valeur: f.id, libelle: f.nom }))]} />
        <Choix
          libelle="Paiement"
          valeur={mode}
          changer={setMode}
          options={[
            { valeur: "comptant", libelle: "Comptant" },
            ...(fournisseur ? [{ valeur: "credit", libelle: "À crédit (dette fournisseur)" }] : []),
          ]}
        />
        {mode === "comptant" && (
          <Choix
            libelle="Payé depuis"
            valeur={compte}
            changer={setCompte}
            options={[{ valeur: "", libelle: "Ma caisse (session ouverte)" }, ...(comptes ?? []).filter((c) => c.actif && c.type !== "livreur").map((c) => ({ valeur: c.id, libelle: c.nom }))]}
          />
        )}
      </div>
      {lignes.map((l, i) => {
        const a = articles?.find((x) => x.article_id === l.article_id);
        const cond = a?.conditionnements.find((c) => c.id === l.conditionnement_id);
        const base = l.quantite * (cond?.contenance ?? 1);
        return (
          <div key={i} className="ligne-achat">
            <Choix libelle="Article" valeur={l.article_id} changer={(v) => maj(i, { article_id: v, conditionnement_id: "" })} options={(articles ?? []).map((x) => ({ valeur: x.article_id, libelle: x.nom }))} />
            <Choix
              libelle="Unité"
              valeur={l.conditionnement_id}
              changer={(v) => maj(i, { conditionnement_id: v })}
              options={[{ valeur: "", libelle: a?.unite ?? "unité" }, ...(a?.conditionnements ?? []).map((c) => ({ valeur: c.id, libelle: `${c.nom} (${c.contenance})` }))]}
            />
            <label className="champ">
              <span>Nombre</span>
              <input type="number" min={1} value={l.quantite} onChange={(e) => maj(i, { quantite: Number(e.target.value) })} />
            </label>
            <ChampMontant libelle="Prix total payé" valeur={l.prix_total} changer={(v) => maj(i, { prix_total: v })} />
            <span className="aide">
              = +{base} {a?.unite} {base > 0 && l.prix_total > 0 && `(${fcfa(Math.round(l.prix_total / base))} l'unité)`}
            </span>
            <button className="petit" onClick={() => setLignes(lignes.filter((_, j) => j !== i))} aria-label="Retirer la ligne">
              ✕
            </button>
          </div>
        );
      })}
      <button onClick={() => setLignes([...lignes, { article_id: articles?.[0]?.article_id ?? "", conditionnement_id: articles?.[0]?.conditionnements[0]?.id ?? "", quantite: 1, prix_total: 0 }])}>
        + Ligne
      </button>
      {(emballages ?? []).length > 0 && (
        <details className="consignes-achat">
          <summary>Emballages consignés {saisies.length > 0 && `(consigne ${fcfa(nette)})`}</summary>
          {(emballages ?? []).map((e) => (
            <div key={e.emballage.id} className="ligne-consigne">
              <span>
                {e.emballage.nom} <small className="aide">({fcfa(e.emballage.valeur)})</small>
              </span>
              <label className="champ">
                <span>Reçus</span>
                <input
                  type="number"
                  min={0}
                  aria-label={`Reçus : ${e.emballage.nom}`}
                  value={consigneDe(e.emballage.id).recus}
                  onChange={(x) => majConsigne(e.emballage.id, { recus: Math.max(0, Number(x.target.value)) })}
                />
              </label>
              <label className="champ">
                <span>Vides rendus</span>
                <input
                  type="number"
                  min={0}
                  aria-label={`Rendus : ${e.emballage.nom}`}
                  value={consigneDe(e.emballage.id).rendus}
                  onChange={(x) => majConsigne(e.emballage.id, { rendus: Math.max(0, Number(x.target.value)) })}
                />
              </label>
            </div>
          ))}
          <p className="formule">Consigne = Σ (reçus − rendus) × consigne de l'emballage ; ajoutée au total.</p>
        </details>
      )}
      <Champ libelle="Note" valeur={note} changer={setNote} />
      <div className="total">
        <span>Total</span>
        <Montant valeur={total} fort />
      </div>
      <button
        className="principal grand"
        disabled={(!lignes.length && !saisies.length) || lignes.some((l) => !l.article_id || l.quantite <= 0) || total < 0}
        onClick={() =>
          agir(
            (pin) =>
              post(
                "/achats",
                {
                  fournisseur_id: fournisseur || null,
                  mode,
                  compte_id: compte || null,
                  note,
                  lignes: lignes.map((l) => ({ ...l, conditionnement_id: l.conditionnement_id || null })),
                  consignes: saisies,
                },
                pin,
              ),
            "Réception enregistrée : stock mis à jour",
          ).then((r) => {
            if (r !== undefined) {
              setLignes([]);
              setConsignes([]);
            }
          })
        }
      >
        Enregistrer la réception
      </button>
    </div>
  );
}

function Historique() {
  const { donnees } = useDonnees(() => get<Achat[]>("/achats"), ["stock"]);
  return (
    <TableauDonnees
      colonnes={["N°", "Date", "Fournisseur", "Mode", "Articles", "Total"]}
      lignes={(donnees ?? []).map((a) => [a.numero, dateHeure(a.horodatage), a.fournisseur ?? "Marché", a.mode, a.lignes.map(([n, q]) => `${q} ${n}`).join(", "), fcfa(a.total)])}
    />
  );
}

function Fournisseurs() {
  const { agir } = useApp();
  const { donnees, recharger } = useDonnees(() => get<Fournisseur[]>("/fournisseurs"), []);
  const [edition, setEdition] = useState<Fournisseur | null>(null);
  const [reglement, setReglement] = useState<Fournisseur | null>(null);
  const [montant, setMontant] = useState(0);
  return (
    <div>
      <button className="principal" onClick={() => setEdition({ id: "", nom: "", telephone: "", notes: "", actif: true, dette: 0 })}>
        + Fournisseur
      </button>
      <TableauDonnees
        colonnes={["Fournisseur", "Téléphone", "Dette", ""]}
        lignes={(donnees ?? []).map((f) => [
          f.nom,
          f.telephone,
          <Montant valeur={f.dette} />,
          <span className="boutons-ligne">
            <button className="petit" onClick={() => setEdition(f)}>
              Modifier
            </button>
            {f.dette > 0 && (
              <button
                className="petit principal"
                onClick={() => {
                  setMontant(f.dette);
                  setReglement(f);
                }}
              >
                Régler
              </button>
            )}
          </span>,
        ])}
      />
      {edition && (
        <Modal titre="Fournisseur" fermer={() => setEdition(null)}>
          <Champ libelle="Nom" valeur={edition.nom} changer={(v) => setEdition({ ...edition, nom: v })} obligatoire />
          <Champ libelle="Téléphone" valeur={edition.telephone} changer={(v) => setEdition({ ...edition, telephone: v })} />
          <Champ libelle="Notes" valeur={edition.notes} changer={(v) => setEdition({ ...edition, notes: v })} />
          <button className="principal" onClick={() => agir((pin) => post("/fournisseurs", edition, pin), "Enregistré").then(() => { setEdition(null); recharger(); })}>
            Enregistrer
          </button>
        </Modal>
      )}
      {reglement && (
        <Modal titre={`Régler ${reglement.nom}`} fermer={() => setReglement(null)}>
          <p>Dette : {fcfa(reglement.dette)}. Règlement partiel possible.</p>
          <ChampMontant libelle="Montant" valeur={montant} changer={setMontant} />
          <button
            className="principal"
            onClick={() => agir((pin) => post("/fournisseurs/reglement", { fournisseur_id: reglement.id, montant }, pin), "Règlement enregistré").then(() => { setReglement(null); recharger(); })}
          >
            Payer depuis ma caisse
          </button>
        </Modal>
      )}
    </div>
  );
}
