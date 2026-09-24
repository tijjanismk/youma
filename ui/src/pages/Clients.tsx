import { useState } from "react";
import { get, post } from "../api";
import { Case, Champ, ChampMontant, Modal, Montant, TableauDonnees } from "../composants/Base";
import { useApp, useDonnees } from "../contexte";
import { dateHeure, fcfa } from "../format";
import { t } from "../i18n";
import type { Client } from "../types";

type Releve = { horodatage: number; type: string; montant: number; commande_numero: number | null; motif: string; solde: number }[];

export default function Clients() {
  const { peut } = useApp();
  const [q, setQ] = useState("");
  const { donnees, recharger } = useDonnees(() => get<Client[]>(`/clients?q=${encodeURIComponent(q)}`), ["caisse", "paiement"], [q]);
  const [edition, setEdition] = useState<Client | null>(null);
  const [fiche, setFiche] = useState<Client | null>(null);
  return (
    <div>
      <div className="titre-ligne">
        <h1>Clients et crédit</h1>
        <button className="principal" onClick={() => setEdition({ id: "", nom: "", telephone: "", adresse: "", reperes: "", credit_autorise: false, limite_credit: 0, actif: true, dette: 0 })}>
          + Client
        </button>
      </div>
      <Champ libelle="Rechercher (nom ou téléphone)" valeur={q} changer={setQ} />
      <TableauDonnees
        colonnes={["Client", "Téléphone", "Crédit", "Dette", ""]}
        lignes={(donnees ?? []).map((c) => [
          <button className="lien" onClick={() => setFiche(c)}>
            {c.nom}
          </button>,
          c.telephone ?? "",
          c.credit_autorise ? `Oui, limite ${fcfa(c.limite_credit)}` : "Non",
          <Montant valeur={c.dette} />,
          peut("client.gerer") && (
            <button className="petit" onClick={() => setEdition(c)}>
              Modifier
            </button>
          ),
        ])}
      />
      {edition && <FormClient c={edition} fermer={() => setEdition(null)} fait={recharger} />}
      {fiche && <FicheClient c={fiche} fermer={() => setFiche(null)} fait={recharger} />}
    </div>
  );
}

function FormClient({ c, fermer, fait }: { c: Client; fermer: () => void; fait: () => void }) {
  const { agir } = useApp();
  const [x, setX] = useState(c);
  return (
    <Modal titre={c.id ? c.nom : "Nouveau client"} fermer={fermer}>
      <Champ libelle="Nom" valeur={x.nom} changer={(v) => setX({ ...x, nom: v })} obligatoire autoFocus />
      <Champ libelle="Téléphone" type="tel" valeur={x.telephone ?? ""} changer={(v) => setX({ ...x, telephone: v })} />
      <Champ libelle="Adresse / quartier" valeur={x.adresse} changer={(v) => setX({ ...x, adresse: v })} />
      <Champ libelle="Repères" valeur={x.reperes} changer={(v) => setX({ ...x, reperes: v })} />
      <Case libelle="Autoriser le crédit (ardoise)" valeur={x.credit_autorise} changer={(v) => setX({ ...x, credit_autorise: v })} />
      {x.credit_autorise && <ChampMontant libelle="Limite de crédit" valeur={x.limite_credit} changer={(v) => setX({ ...x, limite_credit: v })} />}
      <p className="aide">Le crédit est désactivé par défaut (RG-CLI-01). Le modifier demande la permission du gérant.</p>
      <div className="actions">
        <button onClick={fermer}>Annuler</button>
        <button className="principal" disabled={!x.nom.trim()} onClick={() => agir((pin) => post("/clients", x, pin), "Client enregistré").then((r) => r !== undefined && (fait(), fermer()))}>
          Enregistrer
        </button>
      </div>
    </Modal>
  );
}

function FicheClient({ c, fermer, fait }: { c: Client; fermer: () => void; fait: () => void }) {
  const { agir, peut } = useApp();
  const { donnees, recharger } = useDonnees(() => get<{ client: Client; releve: Releve }>(`/clients/${c.id}`), [], [c.id]);
  const [montant, setMontant] = useState(0);
  const dette = donnees?.client.dette ?? c.dette;
  return (
    <Modal titre={`Relevé — ${c.nom}`} fermer={fermer} large>
      <div className="imprimable">
        <h3>
          {c.nom} {c.telephone && `(${c.telephone})`}
        </h3>
        <TableauDonnees
          colonnes={["Date", "Opération", "Commande", "Montant", "Solde"]}
          lignes={(donnees?.releve ?? []).map((l) => [dateHeure(l.horodatage), t(l.type), l.commande_numero ?? "", <Montant valeur={l.montant} />, <Montant valeur={l.solde} />])}
        />
        <div className="total">
          <span>Dette actuelle</span>
          <Montant valeur={dette} fort />
        </div>
      </div>
      {dette > 0 && peut("caisse.encaisser") && (
        <div className="carte">
          <ChampMontant libelle="Règlement reçu" valeur={montant} changer={setMontant} raccourcis={[dette]} />
          <button
            className="principal"
            disabled={montant <= 0}
            onClick={() =>
              agir((pin) => post("/clients/reglement", { client_id: c.id, montant }, pin), "Règlement enregistré").then(() => {
                setMontant(0);
                recharger();
                fait();
              })
            }
          >
            Encaisser le règlement (espèces, ma caisse)
          </button>
        </div>
      )}
      <button onClick={() => window.print()}>Imprimer le relevé</button>
    </Modal>
  );
}
