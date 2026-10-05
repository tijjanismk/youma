import { ArrowDown, ArrowUp } from "lucide-react";
import { useState } from "react";
import { appliquerMenuPerso, ecrireMenuPerso, lireMenuPerso } from "../menuPerso";
import type { EntreeMenu } from "../pages/Accueil";
import { Modal } from "./Base";

/**
 * Personnaliser le menu (fiche 0051) : cocher les écrans à afficher, les monter ou descendre. Propre à l'utilisateur et
 * à cet appareil ; ne change aucun droit.
 */
export function PersonnaliserMenu({ permis, utilisateur, fermer }: { permis: EntreeMenu[]; utilisateur: string; fermer: () => void }) {
  const depart = lireMenuPerso(utilisateur);
  const [ordre, setOrdre] = useState(() => appliquerMenuPerso(permis, depart, true).map((m) => m.chemin));
  const [caches, setCaches] = useState(depart.caches);
  const entree = (c: string) => permis.find((m) => m.chemin === c)!;
  const deplacer = (i: number, d: -1 | 1) =>
    setOrdre((o) => {
      const n = [...o];
      [n[i], n[i + d]] = [n[i + d], n[i]];
      return n;
    });
  return (
    <Modal titre="Personnaliser le menu" fermer={fermer}>
      <p className="aide">Cochez les écrans à afficher et rangez-les. Ce choix ne vaut que pour vous, sur cet appareil ; vos droits ne changent pas.</p>
      <ul className="liste-menu-perso">
        {ordre.map((c, i) => {
          const m = entree(c);
          return (
            <li key={c}>
              <label className="case">
                <input
                  type="checkbox"
                  checked={!caches.includes(c)}
                  onChange={(e) => setCaches((x) => (e.target.checked ? x.filter((y) => y !== c) : [...x, c]))}
                />
                <m.Icone size={20} aria-hidden /> <span>{m.libelle}</span>
              </label>
              <span className="boutons-ligne">
                <button className="petit" disabled={i === 0} onClick={() => deplacer(i, -1)} aria-label={`Monter ${m.libelle}`}>
                  <ArrowUp size={16} aria-hidden />
                </button>
                <button className="petit" disabled={i === ordre.length - 1} onClick={() => deplacer(i, 1)} aria-label={`Descendre ${m.libelle}`}>
                  <ArrowDown size={16} aria-hidden />
                </button>
              </span>
            </li>
          );
        })}
      </ul>
      <div className="actions">
        <button
          onClick={() => {
            ecrireMenuPerso(utilisateur, null);
            fermer();
          }}
        >
          Menu d'origine
        </button>
        <button
          className="principal"
          onClick={() => {
            ecrireMenuPerso(utilisateur, { ordre, caches });
            fermer();
          }}
        >
          Enregistrer
        </button>
      </div>
    </Modal>
  );
}
