import { useState } from "react";
import { get, post } from "../api";
import { Case, Champ, ChampMontant, Choix, Modal, TableauDonnees } from "../composants/Base";
import { useApp, useDonnees } from "../contexte";
import { fcfa, hhmm, JOURS, joursLibelle, lireHhmm } from "../format";
import { pourcentage } from "../recette";
import type { Catalogue, Promotion } from "../types";

const VIDE: Promotion = {
  id: "",
  nom: "Happy hour",
  produit_id: null,
  categorie_id: null,
  type: "prix",
  valeur: 0,
  debut_min: 18 * 60,
  fin_min: 20 * 60,
  jours: 127,
  date_debut: null,
  date_fin: null,
  actif: true,
};

/** Promotions et happy hours (fiche 0017) : prix réduit par plage horaire, jours et dates, sur un produit ou une catégorie. */
export default function Promotions() {
  const { donnees: cat } = useDonnees(() => get<Catalogue>("/catalogue"), ["catalogue"]);
  const { donnees, recharger } = useDonnees(() => get<Promotion[]>("/promotions"), ["catalogue"]);
  const [p, setP] = useState<Promotion | null>(null);
  if (!cat) return null;
  const cible = (x: Promotion) =>
    x.produit_id ? cat.produits.find((y) => y.id === x.produit_id)?.nom : `Catégorie ${cat.categories.find((y) => y.id === x.categorie_id)?.nom ?? ""}`;
  return (
    <div className="carte">
      <div className="titre-ligne">
        <h2>Promotions et happy hours</h2>
        <button className="principal" onClick={() => setP({ ...VIDE, produit_id: cat.produits[0]?.id ?? null })}>
          + Promotion
        </button>
      </div>
      <p className="aide">Le prix est appliqué au moment où l'article est saisi ; la promotion la plus avantageuse pour le client l'emporte.</p>
      <TableauDonnees
        colonnes={["Promotion", "Sur", "Prix ou remise", "Quand", ""]}
        lignes={(donnees ?? []).map((x) => [
          `${x.nom}${x.actif ? "" : " (inactive)"}`,
          cible(x) ?? "",
          x.type === "prix" ? fcfa(x.valeur) : `− ${pourcentage(x.valeur)}`,
          `${hhmm(x.debut_min)} → ${hhmm(x.fin_min)} · ${joursLibelle(x.jours)}${x.date_debut || x.date_fin ? ` · du ${x.date_debut ?? "…"} au ${x.date_fin ?? "…"}` : ""}`,
          <button className="petit" onClick={() => setP(x)}>
            Modifier
          </button>,
        ])}
      />
      {p && <FormPromotion p={p} cat={cat} fermer={() => setP(null)} fait={recharger} />}
    </div>
  );
}

function FormPromotion({ p: initiale, cat, fermer, fait }: { p: Promotion; cat: Catalogue; fermer: () => void; fait: () => void }) {
  const { agir } = useApp();
  const [p, setP] = useState(initiale);
  const [debut, setDebut] = useState(hhmm(initiale.debut_min));
  const [fin, setFin] = useState(hhmm(initiale.fin_min));
  const [sur, setSur] = useState<"produit" | "categorie">(initiale.categorie_id ? "categorie" : "produit");
  const d = lireHhmm(debut);
  const f = lireHhmm(fin);
  const valide = p.nom.trim() && (p.produit_id || p.categorie_id) && d !== null && f !== null && d !== f && p.jours > 0 && (p.type === "prix" || (p.valeur > 0 && p.valeur <= 10_000));
  return (
    <Modal titre="Promotion" fermer={fermer}>
      <Champ libelle="Nom" valeur={p.nom} changer={(v) => setP({ ...p, nom: v })} obligatoire autoFocus />
      <Choix
        libelle="Sur"
        valeur={sur}
        changer={(v) => {
          setSur(v);
          setP(v === "produit" ? { ...p, produit_id: cat.produits[0]?.id ?? null, categorie_id: null } : { ...p, produit_id: null, categorie_id: cat.categories[0]?.id ?? null });
        }}
        options={[
          { valeur: "produit", libelle: "Un produit" },
          { valeur: "categorie", libelle: "Une catégorie" },
        ]}
      />
      {sur === "produit" ? (
        <Choix libelle="Produit" valeur={p.produit_id ?? ""} changer={(v) => setP({ ...p, produit_id: v })} options={cat.produits.map((x) => ({ valeur: x.id, libelle: `${x.nom} (${fcfa(x.prix)})` }))} />
      ) : (
        <Choix libelle="Catégorie" valeur={p.categorie_id ?? ""} changer={(v) => setP({ ...p, categorie_id: v })} options={cat.categories.map((x) => ({ valeur: x.id, libelle: x.nom }))} />
      )}
      <Choix
        libelle="Type"
        valeur={p.type}
        changer={(v) => setP({ ...p, type: v, valeur: 0 })}
        options={[
          { valeur: "prix", libelle: "Prix fixe" },
          { valeur: "pourcentage", libelle: "Remise en %" },
        ]}
      />
      {p.type === "prix" ? (
        <ChampMontant libelle="Prix pendant la promotion" valeur={p.valeur} changer={(v) => setP({ ...p, valeur: v })} />
      ) : (
        <label className="champ">
          <span>Remise (%)</span>
          <input inputMode="numeric" aria-label="Remise (%)" value={p.valeur ? p.valeur / 100 : ""} onChange={(e) => setP({ ...p, valeur: Math.round((parseFloat(e.target.value.replace(",", ".")) || 0) * 100) })} />
        </label>
      )}
      <div className="grille-2">
        <Champ libelle="De (heure)" valeur={debut} changer={setDebut} placeholder="18:00" />
        <Champ libelle="À (heure)" valeur={fin} changer={setFin} placeholder="20:00" />
      </div>
      <div className="suggestions" role="group" aria-label="Jours">
        {JOURS.map((j, i) => (
          <button key={j} type="button" className={p.jours & (1 << i) ? "actif" : ""} aria-pressed={!!(p.jours & (1 << i))} onClick={() => setP({ ...p, jours: p.jours ^ (1 << i) })}>
            {j}
          </button>
        ))}
      </div>
      <div className="grille-2">
        <Champ libelle="Du (facultatif)" type="date" valeur={p.date_debut ?? ""} changer={(v) => setP({ ...p, date_debut: v || null })} />
        <Champ libelle="Au (facultatif)" type="date" valeur={p.date_fin ?? ""} changer={(v) => setP({ ...p, date_fin: v || null })} />
      </div>
      <Case libelle="Active" valeur={p.actif} changer={(v) => setP({ ...p, actif: v })} />
      <div className="actions">
        <button onClick={fermer}>Annuler</button>
        <button
          className="principal"
          disabled={!valide}
          onClick={() =>
            agir((pin) => post("/promotions", { ...p, debut_min: d, fin_min: f }, pin), "Promotion enregistrée").then((r) => {
              if (r !== undefined) {
                fait();
                fermer();
              }
            })
          }
        >
          Enregistrer
        </button>
      </div>
    </Modal>
  );
}
