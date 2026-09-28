import { UtensilsCrossed } from "lucide-react";
import { useState } from "react";
import { get, post } from "../api";
import { useApp, useDonnees } from "../contexte";
import { Case, Modal } from "./Base";

type Menu = { journee_ouverte: boolean; plats: [string, string, string][]; coches: string[]; precedents: string[] };

/**
 * Menu du jour (RG-CAT-07, fiche 0036) : chaque matin, on coche les plats du jour. Les produits qui ne sont pas des
 * « plats du jour » (boissons, eau…) restent toujours proposés.
 */
export function CarteMenuDuJour() {
  const { peut } = useApp();
  const { donnees: menu, recharger } = useDonnees(() => get<Menu>("/menu-du-jour"), ["catalogue", "journee"]);
  const [ouvert, setOuvert] = useState(false);
  if (!menu || !menu.journee_ouverte || menu.plats.length === 0) return null;
  const modifiable = peut("caisse.encaisser") || peut("catalogue.gerer");
  return (
    <div className={`carte menu-du-jour ${menu.coches.length === 0 ? "a-faire" : ""}`}>
      <span>
        <UtensilsCrossed size={20} className="icone-texte" aria-hidden /> <strong>Menu du jour</strong> :{" "}
        {menu.coches.length === 0
          ? "aucun plat coché, les plats du jour ne sont pas proposés."
          : `${menu.coches.length} plat${menu.coches.length > 1 ? "s" : ""} sur ${menu.plats.length}.`}
      </span>
      {modifiable && (
        <button className={menu.coches.length === 0 ? "principal" : ""} onClick={() => setOuvert(true)}>
          Composer le menu du jour
        </button>
      )}
      {ouvert && (
        <ComposerMenu
          menu={menu}
          fermer={() => {
            setOuvert(false);
            recharger();
          }}
        />
      )}
    </div>
  );
}

function ComposerMenu({ menu, fermer }: { menu: Menu; fermer: () => void }) {
  const { agir } = useApp();
  const [coches, setCoches] = useState<string[]>(menu.coches);
  const groupes = [...new Set(menu.plats.map((p) => p[2]))];
  return (
    <Modal titre="Menu du jour" fermer={fermer}>
      <p className="aide">Cochez les plats proposés aujourd'hui. Les boissons et les produits toujours au menu n'apparaissent pas ici.</p>
      {menu.precedents.length > 0 && (
        <button onClick={() => setCoches(menu.precedents.filter((id) => menu.plats.some((p) => p[0] === id)))}>Reprendre le menu d'hier</button>
      )}
      {groupes.map((g) => (
        <div key={g}>
          {g && <h3>{g}</h3>}
          {menu.plats
            .filter((p) => p[2] === g)
            .map(([id, nom]) => (
              <Case key={id} libelle={nom} valeur={coches.includes(id)} changer={(v) => setCoches(v ? [...coches, id] : coches.filter((x) => x !== id))} />
            ))}
        </div>
      ))}
      <div className="actions">
        <button onClick={fermer}>Annuler</button>
        <button
          className="principal"
          onClick={() => agir((pin) => post("/menu-du-jour", { produits: coches }, pin), "Menu du jour enregistré").then((r) => r !== undefined && fermer())}
        >
          Enregistrer ({coches.length})
        </button>
      </div>
    </Modal>
  );
}
