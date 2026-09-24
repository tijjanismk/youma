import { useEffect, useState } from "react";
import { get, post } from "../api";
import { Modal } from "../composants/Base";
import { useApp, useDonnees } from "../contexte";
import { fcfa } from "../format";
import { coutRecette, partBp, pourcentage, recetteValide } from "../recette";
import type { LigneRecette, NiveauStock, Produit, Recette } from "../types";

/**
 * Recette d'un plat (fiche 0014) : ingrédients sortis du stock à chaque vente, plus ceux des options choisies.
 * Facultative : un plat sans recette se vend normalement.
 */
export default function EditeurRecette({ p, fermer, fait }: { p: Produit; fermer: () => void; fait: () => void }) {
  const { agir } = useApp();
  const { donnees: articles } = useDonnees(() => get<NiveauStock[]>("/stock"), []);
  const { donnees: initiale } = useDonnees(() => get<Recette>(`/produits/${p.id}/recette`), []);
  const [r, setR] = useState<Recette | null>(null);
  useEffect(() => {
    if (initiale) setR(initiale);
  }, [initiale]);
  if (!r || !articles) return null;
  const couts = Object.fromEntries(articles.map((a) => [a.article_id, a.cout_unitaire]));
  const unite = (id: string) => articles.find((a) => a.article_id === id)?.unite ?? "";
  const options = p.groupes_options.flatMap((g) => g.options.map((o) => ({ ...o, groupe: g.nom })));
  const lignesOption = (id: string) => r.options.find((o) => o.option_id === id)?.lignes ?? [];
  const majOption = (id: string, lignes: LigneRecette[]) =>
    setR({ ...r, options: [...r.options.filter((o) => o.option_id !== id), ...(lignes.length ? [{ option_id: id, lignes }] : [])] });
  const cout = coutRecette(r.lignes, couts);
  const valide = recetteValide(r.lignes) && r.options.every((o) => recetteValide(o.lignes));
  return (
    <Modal titre={`Recette — ${p.nom}`} fermer={fermer} large>
      <p className="aide">
        Quantités en nombres entiers, dans l'unité de l'article (g, ml, pièce). Chaque vente sort ces quantités du stock ; l'inventaire montre ensuite
        l'écart entre la consommation théorique et le réel.
      </p>
      <h3>Plat</h3>
      <Lignes section="Plat" lignes={r.lignes} articles={articles} unite={unite} changer={(l) => setR({ ...r, lignes: l })} />
      <p>
        Coût matière : <strong>{fcfa(cout)}</strong> pour un prix de {fcfa(p.prix)}, soit <strong>{pourcentage(partBp(cout, p.prix))}</strong>
      </p>
      <p className="formule">Coût matière = Σ quantité × coût unitaire de l'article</p>
      {options.length > 0 && <h3>Ingrédients en plus selon l'option choisie</h3>}
      {options.map((o) => (
        <details key={o.id} open={lignesOption(o.id).length > 0}>
          <summary>
            {o.groupe} : {o.nom}
            {lignesOption(o.id).length > 0 && ` (+ ${fcfa(coutRecette(lignesOption(o.id), couts))})`}
          </summary>
          <Lignes section={o.nom} lignes={lignesOption(o.id)} articles={articles} unite={unite} changer={(l) => majOption(o.id, l)} />
        </details>
      ))}
      <div className="actions">
        <button onClick={fermer}>Annuler</button>
        <button
          className="principal"
          disabled={!valide}
          onClick={() =>
            agir((pin) => post(`/produits/${p.id}/recette`, r, pin), "Recette enregistrée").then((x) => {
              if (x !== undefined) {
                fait();
                fermer();
              }
            })
          }
        >
          Enregistrer la recette
        </button>
      </div>
    </Modal>
  );
}

function Lignes({
  section,
  lignes,
  articles,
  unite,
  changer,
}: {
  section: string;
  lignes: LigneRecette[];
  articles: NiveauStock[];
  unite: (id: string) => string;
  changer: (l: LigneRecette[]) => void;
}) {
  const nom = (l: LigneRecette, i: number) => articles.find((a) => a.article_id === l.article_id)?.nom ?? `ingrédient ${i + 1}`;
  return (
    <div className="lignes-recette">
      {lignes.map((l, i) => (
        <div key={i} className="ligne-recette">
          <select
            aria-label={`${section} : ingrédient ${i + 1}`}
            value={l.article_id}
            onChange={(e) => changer(lignes.map((x, j) => (j === i ? { ...x, article_id: e.target.value } : x)))}
          >
            <option value="">— ingrédient —</option>
            {articles.map((a) => (
              <option key={a.article_id} value={a.article_id}>
                {a.nom} ({a.unite})
              </option>
            ))}
          </select>
          <input
            aria-label={`${section} : quantité de ${nom(l, i)}`}
            inputMode="numeric"
            value={l.quantite || ""}
            onChange={(e) => changer(lignes.map((x, j) => (j === i ? { ...x, quantite: parseInt(e.target.value.replace(/\D/g, ""), 10) || 0 } : x)))}
          />
          <span>{unite(l.article_id)}</span>
          <button className="petit" aria-label={`${section} : retirer ${nom(l, i)}`} onClick={() => changer(lignes.filter((_, j) => j !== i))}>
            ✕
          </button>
        </div>
      ))}
      <button className="lien" aria-label={`${section} : ajouter un ingrédient`} onClick={() => changer([...lignes, { article_id: "", quantite: 0 }])}>
        + ingrédient
      </button>
    </div>
  );
}
