import { useState } from "react";
import { get } from "../api";
import { Choix, TableauDonnees } from "../composants/Base";
import { useDonnees } from "../contexte";
import { dateHeure } from "../format";

type L = { horodatage: number; utilisateur: string | null; autorise_par: string | null; appareil: string | null; action: string; entite: string; avant: string | null; apres: string | null; motif: string | null };

const FILTRES = [
  { valeur: "", libelle: "Tout" },
  { valeur: "commande.annuler", libelle: "Annulations" },
  { valeur: "commande.remise", libelle: "Remises" },
  { valeur: "commande.offrir", libelle: "Offerts" },
  { valeur: "caisse", libelle: "Caisse" },
  { valeur: "produit.prix", libelle: "Prix" },
  { valeur: "stock", libelle: "Stock et inventaire" },
  { valeur: "employe", libelle: "Employés, avances, salaires" },
  { valeur: "horloge", libelle: "Changements de date" },
  { valeur: "sauvegarde", libelle: "Sauvegardes" },
];

/** Journal d'audit : lecture seule, jamais modifiable (RG-SYS-05). */
export default function Journal() {
  const [action, setAction] = useState("");
  const { donnees } = useDonnees(() => get<L[]>(`/audit${action ? `?action=${action}` : ""}`), [], [action]);
  return (
    <div>
      <h1>Journal d'audit</h1>
      <Choix libelle="Filtrer" valeur={action} changer={setAction} options={FILTRES} />
      <TableauDonnees
        colonnes={["Date", "Action", "Par", "Autorisé par", "Poste", "Motif", "Détail"]}
        lignes={(donnees ?? []).map((l) => [
          dateHeure(l.horodatage),
          l.action,
          l.utilisateur ?? "système",
          l.autorise_par ?? "",
          l.appareil ?? "poste central",
          l.motif ?? "",
          <small>
            {l.avant && `avant : ${l.avant} `}
            {l.apres && `après : ${l.apres}`}
          </small>,
        ])}
      />
    </div>
  );
}
