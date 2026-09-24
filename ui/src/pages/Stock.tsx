import { useState } from "react";
import { get, post } from "../api";
import { Champ, Choix, Modal, Montant, Onglets, TableauDonnees, Vide } from "../composants/Base";
import Consignes from "./Consignes";
import { useApp, useDonnees } from "../contexte";
import { dateHeure, fcfa } from "../format";
import { t } from "../i18n";
import type { NiveauStock } from "../types";

type Inventaire = {
  id: string;
  libelle: string;
  statut: string;
  lignes: { article_id: string; nom: string; unite: string; compte: number; theorique: number; ecart: number; valeur_ecart: number }[];
};

export default function Stock() {
  const { peut } = useApp();
  const { donnees, recharger } = useDonnees(() => get<NiveauStock[]>("/stock"), ["stock"]);
  const [onglet, setOnglet] = useState<"niveaux" | "inventaire" | "consignes">("niveaux");
  const [mouvement, setMouvement] = useState<NiveauStock | null>(null);
  const [article, setArticle] = useState<NiveauStock | "nouveau" | null>(null);
  const [historique, setHistorique] = useState<NiveauStock | null>(null);
  const valeur = (donnees ?? []).reduce((s, n) => s + n.valeur, 0);
  return (
    <div>
      <div className="titre-ligne">
        <h1>Stock</h1>
        {peut("stock.mouvement") && (
          <button className="principal" onClick={() => setArticle("nouveau")}>
            + Article
          </button>
        )}
      </div>
      <Onglets
        onglets={[
          { cle: "niveaux", libelle: "Niveaux" },
          ...(peut("stock.inventaire") ? [{ cle: "inventaire" as const, libelle: "Inventaire" }] : []),
          { cle: "consignes", libelle: "Consignes (bouteilles, casiers)" },
        ]}
        actif={onglet}
        changer={setOnglet}
      />
      {onglet === "niveaux" && (
        <>
          <p>
            Valeur du stock : <strong>{fcfa(valeur)}</strong> <small className="aide">(quantité × dernier prix d'achat)</small>
          </p>
          <TableauDonnees
            colonnes={["Article", "Quantité", "Seuil", "Coût unitaire", "Valeur", ""]}
            lignes={(donnees ?? []).map((n) => [
              <button className="lien" onClick={() => setHistorique(n)}>
                {n.nom}
              </button>,
              <strong className={n.alerte ? "negatif" : ""}>
                {n.quantite} {n.unite}
                {n.alerte ? " ⚠️" : ""}
              </strong>,
              n.seuil_alerte,
              fcfa(n.cout_unitaire),
              fcfa(n.valeur),
              <span className="boutons-ligne">
                {peut("stock.mouvement") && (
                  <button className="petit" onClick={() => setMouvement(n)}>
                    Perte / sortie
                  </button>
                )}
                {peut("stock.mouvement") && (
                  <button className="petit" onClick={() => setArticle(n)}>
                    Modifier
                  </button>
                )}
              </span>,
            ])}
          />
        </>
      )}
      {onglet === "inventaire" && <Inventaires articles={donnees ?? []} />}
      {onglet === "consignes" && <Consignes />}
      {mouvement && <MouvementStock n={mouvement} fermer={() => setMouvement(null)} fait={recharger} />}
      {article && <FormArticle n={article === "nouveau" ? null : article} fermer={() => setArticle(null)} fait={recharger} />}
      {historique && <Historique n={historique} fermer={() => setHistorique(null)} />}
    </div>
  );
}

function MouvementStock({ n, fermer, fait }: { n: NiveauStock; fermer: () => void; fait: () => void }) {
  const { agir } = useApp();
  const [type, setType] = useState("casse");
  const [quantite, setQuantite] = useState(1);
  const [cond, setCond] = useState("");
  const [motif, setMotif] = useState("");
  const types = ["casse", "perte", "perime", "vol", "repas_personnel", "offert", "consommation_interne", "retour_fournisseur", "regularisation"];
  return (
    <Modal titre={`Sortie de stock — ${n.nom}`} fermer={fermer}>
      <Choix libelle="Type" valeur={type} changer={setType} options={types.map((x) => ({ valeur: x, libelle: t(x) }))} />
      <label className="champ">
        <span>Quantité {type === "regularisation" && "(négative pour retirer)"}</span>
        <input type="number" value={quantite} onChange={(e) => setQuantite(Number(e.target.value))} />
      </label>
      {n.conditionnements.length > 0 && (
        <Choix
          libelle="Unité"
          valeur={cond}
          changer={setCond}
          options={[{ valeur: "", libelle: n.unite }, ...n.conditionnements.map((c) => ({ valeur: c.id, libelle: `${c.nom} (${c.contenance})` }))]}
        />
      )}
      <Champ libelle="Motif" valeur={motif} changer={setMotif} obligatoire />
      <div className="actions">
        <button onClick={fermer}>Annuler</button>
        <button
          className="principal"
          disabled={!motif.trim() || quantite === 0}
          onClick={() =>
            agir((pin) => post("/stock/mouvements", { article_id: n.article_id, type, quantite, motif, conditionnement_id: cond || null }, pin), "Mouvement enregistré").then(
              (r) => r !== undefined && (fait(), fermer()),
            )
          }
        >
          Enregistrer
        </button>
      </div>
    </Modal>
  );
}

function FormArticle({ n, fermer, fait }: { n: NiveauStock | null; fermer: () => void; fait: () => void }) {
  const { agir } = useApp();
  const [nom, setNom] = useState(n?.nom ?? "");
  const [unite, setUnite] = useState(n?.unite ?? "bouteille");
  const [famille, setFamille] = useState(n?.famille ?? "Boissons");
  const [seuil, setSeuil] = useState(n?.seuil_alerte ?? 0);
  const [conds, setConds] = useState(n?.conditionnements ?? []);
  return (
    <Modal titre={n ? `Modifier ${n.nom}` : "Nouvel article de stock"} fermer={fermer}>
      <Champ libelle="Nom" valeur={nom} changer={setNom} obligatoire autoFocus />
      <Champ libelle="Unité de base (bouteille, pièce, g, ml…)" valeur={unite} changer={setUnite} />
      <Champ libelle="Famille" valeur={famille} changer={setFamille} />
      <label className="champ">
        <span>Seuil d'alerte</span>
        <input type="number" value={seuil} onChange={(e) => setSeuil(Number(e.target.value))} />
      </label>
      <h3>Conditionnements d'achat</h3>
      {conds.map((c, i) => (
        <div key={i} className="grille-2">
          <Champ libelle="Nom" valeur={c.nom} changer={(v) => setConds(conds.map((x, j) => (j === i ? { ...x, nom: v } : x)))} />
          <label className="champ">
            <span>Contient ({unite})</span>
            <input type="number" value={c.contenance} onChange={(e) => setConds(conds.map((x, j) => (j === i ? { ...x, contenance: Number(e.target.value) } : x)))} />
          </label>
        </div>
      ))}
      <button className="lien" onClick={() => setConds([...conds, { id: "", nom: "Casier de 24", contenance: 24 }])}>
        + Conditionnement
      </button>
      <div className="actions">
        <button onClick={fermer}>Annuler</button>
        <button
          className="principal"
          disabled={!nom.trim()}
          onClick={() =>
            agir(
              (pin) => post("/stock/articles", { id: n?.article_id ?? "", nom, unite, famille, seuil_alerte: seuil, conditionnements: conds.filter((c) => c.nom && c.contenance > 0) }, pin),
              "Article enregistré",
            ).then((r) => r !== undefined && (fait(), fermer()))
          }
        >
          Enregistrer
        </button>
      </div>
    </Modal>
  );
}

function Historique({ n, fermer }: { n: NiveauStock; fermer: () => void }) {
  type H = { mouvements: { type_: string; quantite: number; motif: string; horodatage: number; utilisateur: string | null }[]; prix_achat: [number, number, string | null][] };
  const { donnees } = useDonnees(() => get<H>(`/stock/articles/${n.article_id}/mouvements`), []);
  return (
    <Modal titre={n.nom} fermer={fermer} large>
      <h3>Mouvements</h3>
      <TableauDonnees
        colonnes={["Date", "Type", "Quantité", "Motif", "Par"]}
        lignes={(donnees?.mouvements ?? []).map((m) => [dateHeure(m.horodatage), t(m.type_), m.quantite > 0 ? `+${m.quantite}` : m.quantite, m.motif, m.utilisateur ?? ""])}
      />
      <h3>Historique des prix d'achat</h3>
      <TableauDonnees colonnes={["Date", "Coût unitaire", "Fournisseur"]} lignes={(donnees?.prix_achat ?? []).map(([h, c, f]) => [dateHeure(h), fcfa(c), f ?? "Marché"])} />
    </Modal>
  );
}

function Inventaires({ articles }: { articles: NiveauStock[] }) {
  const { agir, peut } = useApp();
  const { donnees: liste, recharger } = useDonnees(() => get<{ id: string; libelle: string; statut: string; cree_le: number }[]>("/inventaires"), ["stock"]);
  const enCours = liste?.find((i) => i.statut === "en_cours");
  const { donnees: inv, recharger: rechargerInv } = useDonnees(() => (enCours ? get<Inventaire>(`/inventaires/${enCours.id}`) : Promise.resolve(null)), ["stock"], [enCours?.id]);
  const [famille, setFamille] = useState("");
  const familles = [...new Set(articles.map((a) => a.famille))];
  if (!liste) return null;
  if (!enCours)
    return (
      <div className="carte">
        <p>Inventaire complet ou partiel (ex. « boissons » chaque soir).</p>
        <button className="principal grand" onClick={() => agir((pin) => post("/inventaires", { libelle: `Inventaire ${famille || "complet"}` }, pin), "Inventaire commencé").then(recharger)}>
          Commencer un inventaire
        </button>
        {liste.length > 0 && (
          <TableauDonnees colonnes={["Date", "Libellé", "Statut"]} lignes={liste.map((i) => [dateHeure(i.cree_le), i.libelle, i.statut])} />
        )}
      </div>
    );
  const comptes = new Map(inv?.lignes.map((l) => [l.article_id, l]) ?? []);
  return (
    <div className="carte">
      <h2>{enCours.libelle}</h2>
      <Choix libelle="Famille" valeur={famille} changer={setFamille} options={[{ valeur: "", libelle: "Toutes" }, ...familles.map((f) => ({ valeur: f, libelle: f }))]} />
      <TableauDonnees
        colonnes={["Article", "Compté", "Théorique", "Écart"]}
        lignes={articles
          .filter((a) => !famille || a.famille === famille)
          .map((a) => {
            const l = comptes.get(a.article_id);
            return [
              a.nom,
              <input
                type="number"
                min={0}
                className="petit-champ"
                defaultValue={l?.compte ?? ""}
                aria-label={`Compté ${a.nom}`}
                onBlur={(e) => e.target.value !== "" && agir((pin) => post(`/inventaires/${enCours.id}/comptage`, { article_id: a.article_id, compte: Number(e.target.value) }, pin)).then(rechargerInv)}
              />,
              l ? l.theorique : "",
              l ? <strong className={l.ecart < 0 ? "negatif" : ""}>{l.ecart}</strong> : "",
            ];
          })}
      />
      {(inv?.lignes.length ?? 0) === 0 && <Vide>Saisissez les quantités comptées.</Vide>}
      <div className="actions">
        <button onClick={() => agir((pin) => post(`/inventaires/${enCours.id}/abandonner`, {}, pin)).then(recharger)}>Abandonner</button>
        {peut("stock.inventaire") && (
          <button className="principal" onClick={() => agir((pin) => post(`/inventaires/${enCours.id}/valider`, {}, pin), "Inventaire validé : écarts enregistrés").then(recharger)}>
            Valider (responsable)
          </button>
        )}
      </div>
      <p className="aide">
        Valeur des écarts : <Montant valeur={(inv?.lignes ?? []).reduce((s, l) => s + l.valeur_ecart, 0)} />
      </p>
    </div>
  );
}
