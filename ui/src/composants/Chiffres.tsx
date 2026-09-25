import type { LucideIcon } from "lucide-react";
import type { ReactNode } from "react";

export type Ton = "accent" | "vert" | "rouge" | "bleu" | "neutre";

/** Rangée de chiffres clés en tête d'écran : l'essentiel se lit avant les tableaux. */
export function Chiffres({ children }: { children: ReactNode }) {
  return <div className="chiffres">{children}</div>;
}

export function Chiffre({
  libelle,
  valeur,
  Icone,
  ton = "accent",
  detail,
}: {
  libelle: string;
  valeur: ReactNode;
  Icone: LucideIcon;
  ton?: Ton;
  detail?: ReactNode;
}) {
  return (
    <div className={`chiffre ton-${ton}`}>
      <span className="chiffre-icone" aria-hidden>
        <Icone size={22} strokeWidth={1.9} />
      </span>
      <div className="chiffre-texte">
        <span className="chiffre-libelle">{libelle}</span>
        <strong>{valeur}</strong>
        {detail && <small>{detail}</small>}
      </div>
    </div>
  );
}

/** Pastille d'état (stock, paie…) : couleur + mot, jamais la couleur seule. */
export function Etat({ ton, children }: { ton: Ton; children: ReactNode }) {
  return <span className={`etat-pastille ton-${ton}`}>{children}</span>;
}
