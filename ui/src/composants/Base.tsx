import { ReactNode, useEffect, useState } from "react";
import { fcfa, lireMontant, nombre } from "../format";

export function Modal({ titre, fermer, children, large }: { titre: string; fermer: () => void; children: ReactNode; large?: boolean }) {
  useEffect(() => {
    const k = (e: KeyboardEvent) => e.key === "Escape" && fermer();
    window.addEventListener("keydown", k);
    return () => window.removeEventListener("keydown", k);
  }, [fermer]);
  return (
    <div className="voile" onMouseDown={(e) => e.target === e.currentTarget && fermer()}>
      <div className={`modal ${large ? "large" : ""}`} role="dialog" aria-modal="true" aria-label={titre}>
        <div className="modal-entete">
          <h2>{titre}</h2>
          <button className="fermer" onClick={fermer} aria-label="Fermer">
            ✕
          </button>
        </div>
        <div className="modal-corps">{children}</div>
      </div>
    </div>
  );
}

/** Pavé numérique de PIN : grands boutons, utilisable sans clavier. */
export function PinPad({ valider, libelle = "Valider", longueurMax = 6 }: { valider: (pin: string) => void; libelle?: string; longueurMax?: number }) {
  const [pin, setPin] = useState("");
  const ajouter = (c: string) => setPin((p) => (p.length < longueurMax ? p + c : p));
  useEffect(() => {
    const k = (e: KeyboardEvent) => {
      if (/^\d$/.test(e.key)) ajouter(e.key);
      else if (e.key === "Backspace") setPin((p) => p.slice(0, -1));
      else if (e.key === "Enter" && pin.length >= 4) valider(pin);
    };
    window.addEventListener("keydown", k);
    return () => window.removeEventListener("keydown", k);
  });
  return (
    <div className="pinpad">
      <div className="pin-affichage" aria-label="PIN saisi">
        {pin.length ? "●".repeat(pin.length) : <span className="aide">Code PIN</span>}
      </div>
      <div className="pin-touches">
        {["1", "2", "3", "4", "5", "6", "7", "8", "9"].map((c) => (
          <button key={c} onClick={() => ajouter(c)} aria-label={`Chiffre ${c}`}>
            {c}
          </button>
        ))}
        <button onClick={() => setPin("")} aria-label="Effacer tout">
          C
        </button>
        <button onClick={() => ajouter("0")} aria-label="Chiffre 0">
          0
        </button>
        <button onClick={() => setPin((p) => p.slice(0, -1))} aria-label="Effacer">
          ⌫
        </button>
      </div>
      <button
        className="principal grand"
        disabled={pin.length < 4}
        onClick={() => {
          valider(pin);
          setPin("");
        }}
      >
        {libelle}
      </button>
    </div>
  );
}

/** Saisie d'un montant FCFA entier, avec raccourcis. */
export function ChampMontant({
  valeur,
  changer,
  libelle,
  raccourcis = [],
  autoFocus,
}: {
  valeur: number;
  changer: (n: number) => void;
  libelle: string;
  raccourcis?: number[];
  autoFocus?: boolean;
}) {
  return (
    <label className="champ">
      <span>{libelle}</span>
      <input
        inputMode="numeric"
        autoFocus={autoFocus}
        value={valeur ? nombre(valeur) : ""}
        placeholder="0"
        onChange={(e) => changer(lireMontant(e.target.value))}
        aria-label={libelle}
      />
      {raccourcis.length > 0 && (
        <div className="raccourcis">
          {raccourcis.map((r) => (
            <button type="button" key={r} onClick={() => changer(r)}>
              {nombre(r)}
            </button>
          ))}
        </div>
      )}
    </label>
  );
}

export function Champ({
  libelle,
  valeur,
  changer,
  type = "text",
  placeholder,
  obligatoire,
  autoFocus,
}: {
  libelle: string;
  valeur: string;
  changer: (v: string) => void;
  type?: string;
  placeholder?: string;
  obligatoire?: boolean;
  autoFocus?: boolean;
}) {
  return (
    <label className="champ">
      <span>
        {libelle}
        {obligatoire && " *"}
      </span>
      <input type={type} value={valeur} placeholder={placeholder} autoFocus={autoFocus} onChange={(e) => changer(e.target.value)} aria-label={libelle} />
    </label>
  );
}

export function Choix<T extends string>({
  libelle,
  valeur,
  changer,
  options,
}: {
  libelle: string;
  valeur: T;
  changer: (v: T) => void;
  options: { valeur: T; libelle: string }[];
}) {
  return (
    <label className="champ">
      <span>{libelle}</span>
      <select value={valeur} onChange={(e) => changer(e.target.value as T)} aria-label={libelle}>
        {options.map((o) => (
          <option key={o.valeur} value={o.valeur}>
            {o.libelle}
          </option>
        ))}
      </select>
    </label>
  );
}

const AUTRE = "\u0000autre";

/** Liste de choix (groupée ou non) avec « Autre… » pour une saisie libre : la valeur reste un simple texte. */
export function ChoixOuAutre({
  libelle,
  valeur,
  changer,
  groupes,
  obligatoire,
}: {
  libelle: string;
  valeur: string;
  changer: (v: string) => void;
  groupes: { nom: string; options: string[] }[];
  obligatoire?: boolean;
}) {
  const connue = (v: string) => groupes.some((g) => g.options.some((o) => o.toLowerCase() === v.trim().toLowerCase()));
  const [autre, setAutre] = useState(() => valeur.trim() !== "" && !connue(valeur));
  const choisie = groupes.flatMap((g) => g.options).find((o) => o.toLowerCase() === valeur.trim().toLowerCase()) ?? "";
  const options = (liste: string[]) =>
    liste.map((o) => (
      <option key={o} value={o}>
        {o}
      </option>
    ));
  return (
    <label className="champ">
      <span>
        {libelle}
        {obligatoire && " *"}
      </span>
      <select
        value={autre ? AUTRE : choisie}
        onChange={(e) => {
          const v = e.target.value;
          setAutre(v === AUTRE);
          changer(v === AUTRE ? "" : v);
        }}
        aria-label={libelle}
      >
        <option value="">— Choisir —</option>
        {groupes.map((g) =>
          g.nom ? (
            <optgroup key={g.nom} label={g.nom}>
              {options(g.options)}
            </optgroup>
          ) : (
            options(g.options)
          ),
        )}
        <option value={AUTRE}>Autre…</option>
      </select>
      {autre && <input value={valeur} onChange={(e) => changer(e.target.value)} placeholder="Saisir le nom" aria-label={`${libelle} (autre)`} autoFocus />}
    </label>
  );
}

export function Case({ libelle, valeur, changer }: { libelle: string; valeur: boolean; changer: (v: boolean) => void }) {
  return (
    <label className="case">
      <input type="checkbox" checked={valeur} onChange={(e) => changer(e.target.checked)} />
      <span>{libelle}</span>
    </label>
  );
}

export function Montant({ valeur, fort }: { valeur: number; fort?: boolean }) {
  return <span className={`montant ${valeur < 0 ? "negatif" : ""} ${fort ? "fort" : ""}`}>{fcfa(valeur)}</span>;
}

export function Onglets<T extends string>({ onglets, actif, changer }: { onglets: { cle: T; libelle: string }[]; actif: T; changer: (c: T) => void }) {
  return (
    <div className="onglets" role="tablist">
      {onglets.map((o) => (
        <button key={o.cle} role="tab" aria-selected={actif === o.cle} className={actif === o.cle ? "actif" : ""} onClick={() => changer(o.cle)}>
          {o.libelle}
        </button>
      ))}
    </div>
  );
}

export function Vide({ children }: { children: ReactNode }) {
  return <div className="vide">{children}</div>;
}

/** Tableau simple lisible sur téléphone (défilement horizontal). */
/**
 * Tableau ; sur téléphone, chaque ligne devient une carte (« colonne : valeur ») au lieu de colonnes écrasées.
 * La 1re colonne sert de titre de carte ; une colonne sans titre (actions) passe en bas de la carte.
 */
export function TableauDonnees({ colonnes, lignes, cartes = true }: { colonnes: string[]; lignes: ReactNode[][]; cartes?: boolean }) {
  return (
    <div className="tableau-conteneur">
      <table className={`tableau ${cartes ? "cartes" : ""}`} role="table">
        <thead role="rowgroup">
          <tr role="row">
            {colonnes.map((c, i) => (
              <th key={`${c}-${i}`} role="columnheader">
                {c}
              </th>
            ))}
          </tr>
        </thead>
        <tbody role="rowgroup">
          {lignes.map((l, i) => (
            <tr key={i} role="row">
              {l.map((c, j) => (
                <td key={j} role="cell" data-label={colonnes[j] ?? ""}>
                  {c}
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

/** Demande un texte (motif) : les motifs sont obligatoires pour les actions sensibles. */
export function DemandeMotif({
  titre,
  suggestions = [],
  valider,
  fermer,
  enfants,
}: {
  titre: string;
  suggestions?: string[];
  valider: (motif: string) => void;
  fermer: () => void;
  enfants?: ReactNode;
}) {
  const [motif, setMotif] = useState("");
  return (
    <Modal titre={titre} fermer={fermer}>
      {enfants}
      <div className="suggestions">
        {suggestions.map((s) => (
          <button key={s} className={motif === s ? "actif" : ""} onClick={() => setMotif(s)}>
            {s}
          </button>
        ))}
      </div>
      <Champ libelle="Motif" valeur={motif} changer={setMotif} obligatoire autoFocus />
      <div className="actions">
        <button onClick={fermer}>Annuler</button>
        <button className="principal" disabled={!motif.trim()} onClick={() => valider(motif.trim())}>
          Confirmer
        </button>
      </div>
    </Modal>
  );
}
