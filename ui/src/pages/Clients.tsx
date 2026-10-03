import { useState } from "react";
import { get, post } from "../api";
import { Case, Champ, ChampMontant, Modal, Montant, Onglets, TableauDonnees } from "../composants/Base";
import { useApp, useDonnees } from "../contexte";
import { dateFr, dateHeure, fcfa } from "../format";
import { t } from "../i18n";
import type { Carte, Client, EtatFidelite, MouvementPoints } from "../types";
import { AvisClients } from "./Avis";
import { CartesCadeaux, Societes } from "./CartesSocietes";

type Releve = { horodatage: number; type: string; montant: number; commande_numero: number | null; motif: string; solde: number }[];

type Vue = "clients" | "cartes" | "societes" | "avis";

export default function Clients() {
  const { peut } = useApp();
  const [vue, setVue] = useState<Vue>("clients");
  return (
    <div>
      <Onglets
        onglets={[
          { cle: "clients" as Vue, libelle: "Clients et crédit" },
          { cle: "cartes" as Vue, libelle: "Cartes cadeaux" },
          { cle: "societes" as Vue, libelle: "Sociétés" },
          ...(peut("rapport.voir") ? [{ cle: "avis" as Vue, libelle: "Avis" }] : []),
        ]}
        actif={vue}
        changer={setVue}
      />
      {vue === "clients" && <ListeClients />}
      {vue === "cartes" && <CartesCadeaux />}
      {vue === "societes" && <Societes />}
      {vue === "avis" && <AvisClients />}
    </div>
  );
}

function ListeClients() {
  const { peut, etat } = useApp();
  const fidelite = etat?.parametres.fidelite?.active ?? false;
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
        colonnes={["Client", "Téléphone", "Crédit", "Dette", ...(fidelite ? ["Points"] : []), ""]}
        lignes={(donnees ?? []).map((c) => [
          <>
            <button className="lien" onClick={() => setFiche(c)}>
              {c.nom}
            </button>
            {c.vip && <span className="etiquette vip">Privilégié</span>}
          </>,
          c.telephone ?? "",
          c.credit_autorise ? `Oui, limite ${fcfa(c.limite_credit)}` : "Non",
          <Montant valeur={c.dette} />,
          ...(fidelite ? [c.points ?? 0] : []),
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
  const { donnees, recharger } = useDonnees(
    () => get<{ client: Client; releve: Releve; fidelite: EtatFidelite; points: MouvementPoints[]; cartes: Carte[] }>(`/clients/${c.id}`),
    [],
    [c.id],
  );
  const [montant, setMontant] = useState(0);
  const [privilege, setPrivilege] = useState(false);
  const dette = donnees?.client.dette ?? c.dette;
  const client = donnees?.client ?? c;
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
      {donnees?.fidelite.active && (
        <div className="carte">
          <h3>Fidélité</h3>
          <p>
            <strong>{donnees.fidelite.points} points</strong> = {fcfa(donnees.fidelite.valeur)} de réduction
            {donnees.fidelite.points < donnees.fidelite.minimum_points && ` (utilisables dès ${donnees.fidelite.minimum_points} points)`}
          </p>
          <TableauDonnees
            colonnes={["Date", "Mouvement", "Addition", "Points"]}
            lignes={donnees.points.slice(0, 8).map((m) => [dateHeure(m.horodatage), t(m.type_), m.commande_numero ? `n°${m.commande_numero}` : "", m.points])}
          />
        </div>
      )}
      {(donnees?.cartes.length ?? 0) > 0 && (
        <div className="carte">
          <h3>Cartes et bons d'avoir</h3>
          <TableauDonnees
            colonnes={["Code", "Type", "Solde", "Valable jusqu'au"]}
            lignes={donnees!.cartes.map((k) => [k.code, t(k.type), fcfa(k.solde), k.expire_le ? dateFr(k.expire_le) : "Sans fin"])}
          />
        </div>
      )}
      <div className="carte">
        <h3>Client privilégié</h3>
        <p className="aide">
          Ses commandes à distance passent en tête de la file et en cuisine. Les tables restent servies par ordre d'arrivée.
        </p>
        <p>
          {client.vip ? `Privilégié ${client.vip_jusqu_au ? `jusqu'au ${dateFr(client.vip_jusqu_au)}` : "sans date de fin"}` : "Client ordinaire"}
        </p>
        {peut("client.credit") && (
          <button onClick={() => setPrivilege(true)}>{client.vip ? "Modifier ou retirer le privilège" : "Rendre privilégié"}</button>
        )}
      </div>
      <button onClick={() => window.print()}>Imprimer le relevé</button>
      {privilege && <FormPrivilege c={client} fermer={() => setPrivilege(false)} fait={() => (recharger(), fait())} />}
    </Modal>
  );
}

function FormPrivilege({ c, fermer, fait }: { c: Client; fermer: () => void; fait: () => void }) {
  const { agir } = useApp();
  const [vip, setVip] = useState(c.vip ?? false);
  const [fin, setFin] = useState(c.vip_jusqu_au ?? "");
  const [motif, setMotif] = useState("");
  return (
    <Modal titre={`Privilège — ${c.nom}`} fermer={fermer}>
      <Case libelle="Client privilégié" valeur={vip} changer={setVip} />
      {vip && <Champ libelle="Jusqu'au (vide = sans date de fin)" type="date" valeur={fin} changer={setFin} />}
      <Champ libelle="Motif (facultatif)" valeur={motif} changer={setMotif} placeholder="Client fidèle, geste après une mauvaise expérience…" />
      <div className="actions">
        <button onClick={fermer}>Annuler</button>
        <button
          className="principal"
          onClick={() => agir((pin) => post(`/clients/${c.id}/privilege`, { vip, jusqu_au: vip ? fin || null : null, motif }, pin), "Privilège enregistré").then((r) => r !== undefined && (fait(), fermer()))}
        >
          Enregistrer
        </button>
      </div>
    </Modal>
  );
}
