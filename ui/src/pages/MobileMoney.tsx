import { useState } from "react";
import { get, post } from "../api";
import { Onglets, TableauDonnees } from "../composants/Base";
import { useApp, useDonnees } from "../contexte";
import { dateHeure, fcfa } from "../format";
import { t } from "../i18n";

type Part = { part_id: string; compte: string; montant: number; reference: string | null; numero_payeur: string | null; horodatage: number; commande_numero: number | null; statut: string; caissier: string | null };

/** Paiements Mobile Money « à vérifier » : contrôle anti-fraude (faux SMS, captures). */
export default function MobileMoney() {
  const { agir } = useApp();
  const [filtre, setFiltre] = useState<"a_verifier" | "tous">("a_verifier");
  const { donnees, recharger } = useDonnees(() => get<Part[]>(`/mobile-money${filtre === "a_verifier" ? "?statut=a_verifier" : ""}`), ["paiement"], [filtre]);
  const verifier = (id: string, statut: string) => {
    const note = statut === "rejete" ? prompt("Pourquoi ? (introuvable sur le relevé…)") ?? "" : "";
    agir((pin) => post(`/mobile-money/${id}/verifier`, { statut, note }, pin), statut === "verifie" ? "Vérifié" : "Rejeté").then(recharger);
  };
  const total = (donnees ?? []).reduce((s, p) => s + p.montant, 0);
  return (
    <div>
      <h1>Mobile Money</h1>
      <p className="aide">Comparez chaque référence avec les SMS reçus sur le téléphone du restaurant ou le relevé de l'opérateur.</p>
      <Onglets
        onglets={[
          { cle: "a_verifier", libelle: "À vérifier" },
          { cle: "tous", libelle: "Tous" },
        ]}
        actif={filtre}
        changer={setFiltre}
      />
      <p>
        {donnees?.length ?? 0} paiement(s) — {fcfa(total)}
      </p>
      <TableauDonnees
        colonnes={["Date", "Opérateur", "Montant", "Référence", "Payeur", "Commande", "Caissier", "Statut", ""]}
        lignes={(donnees ?? []).map((p) => [
          dateHeure(p.horodatage),
          p.compte,
          fcfa(p.montant),
          <code>{p.reference ?? "—"}</code>,
          p.numero_payeur ?? "",
          p.commande_numero ?? "",
          p.caissier ?? "",
          t(p.statut),
          p.statut === "a_verifier" && (
            <span className="boutons-ligne">
              <button className="petit principal" onClick={() => verifier(p.part_id, "verifie")}>
                Reçu ✓
              </button>
              <button className="petit attention" onClick={() => verifier(p.part_id, "rejete")}>
                Introuvable
              </button>
            </span>
          ),
        ])}
      />
    </div>
  );
}
