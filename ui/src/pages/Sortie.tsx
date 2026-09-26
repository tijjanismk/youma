import { useState } from "react";
import { post } from "../api";
import { useApp } from "../contexte";
import { fcfa, heure } from "../format";

type Resultat = {
  statut: "paye" | "non_paye";
  numero: number;
  titre: string;
  total: number;
  reste: number;
  payee_le: number | null;
  lignes: [number, string][];
  deja_presente: { horodatage: number; par: string | null }[];
};

/** Contrôle du bon de sortie : numéro + code imprimés sur le ticket de caisse (RG-SOR-01 à 03). */
export default function Sortie() {
  const { agir } = useApp();
  const [numero, setNumero] = useState("");
  const [code, setCode] = useState("");
  const [r, setR] = useState<Resultat | null>(null);

  const controler = async () => {
    setR(null);
    const res = await agir((pin) => post<Resultat>("/sortie/controle", { numero: Number(numero), code: code.trim().toUpperCase() }, pin));
    if (res) setR(res);
  };

  const deja = r?.deja_presente.length ? r.deja_presente[r.deja_presente.length - 1] : null;
  const vert = r?.statut === "paye" && !deja;

  return (
    <div className="sortie">
      <h1>Contrôle de sortie</h1>
      <p className="aide">Saisissez le n° du bon de sortie et le code de contrôle imprimés en bas du ticket de caisse.</p>
      <div className="carte etroite">
        <label className="champ">
          <span>N° du bon de sortie</span>
          <input inputMode="numeric" value={numero} onChange={(e) => setNumero(e.target.value.replace(/\D/g, ""))} aria-label="N° du bon de sortie" autoFocus />
        </label>
        <label className="champ">
          <span>Code de contrôle</span>
          <input value={code} maxLength={4} onChange={(e) => setCode(e.target.value.toUpperCase())} aria-label="Code de contrôle" className="code-saisie" />
        </label>
        <button className="principal tres-grand" disabled={!numero || code.trim().length !== 4} onClick={controler}>
          Vérifier
        </button>
      </div>
      {r && (
        <div className={`carte resultat-sortie ${vert ? "ok" : "ko"}`} role="status">
          <h2>{r.statut === "paye" ? (deja ? "DÉJÀ PRÉSENTÉ" : "PAYÉ — peut sortir") : "NON PAYÉ"}</h2>
          {deja && (
            <p className="attention-texte">
              Ce ticket a déjà été présenté à {heure(deja.horodatage)}
              {deja.par ? ` (contrôlé par ${deja.par})` : ""}. Vérifiez les articles emportés.
            </p>
          )}
          <p>
            <strong>{r.titre}</strong> — bon n°{r.numero} — {fcfa(r.total)}
            {r.payee_le ? ` — payé à ${heure(r.payee_le)}` : ""}
          </p>
          {r.statut !== "paye" && <p className="attention-texte">Reste à payer : {fcfa(r.reste)}</p>}
          <ul>
            {r.lignes.map(([q, l], i) => (
              <li key={i}>
                {q} × {l}
              </li>
            ))}
          </ul>
          <button
            onClick={() => {
              setR(null);
              setNumero("");
              setCode("");
            }}
          >
            Ticket suivant
          </button>
        </div>
      )}
    </div>
  );
}
