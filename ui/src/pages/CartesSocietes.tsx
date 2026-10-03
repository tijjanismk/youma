import { useState } from "react";
import { get, post } from "../api";
import { Case, Champ, ChampMontant, Choix, Modal, Montant, TableauDonnees } from "../composants/Base";
import { useApp, useDonnees } from "../contexte";
import { aujourdhui, dateFr, dateHeure, fcfa, finDuMois, premierDuMois } from "../format";
import { t } from "../i18n";
import type { Carte, Client, Compte, Contrat } from "../types";

/** Cartes cadeaux vendues et bons d'avoir offerts (RG-CAD-01 à 05). */
export function CartesCadeaux() {
  const { peut } = useApp();
  const { donnees, recharger } = useDonnees(() => get<Carte[]>("/cartes"), ["caisse", "paiement"]);
  const [vente, setVente] = useState(false);
  const [avoir, setAvoir] = useState(false);
  const [affichee, setAffichee] = useState<Carte | null>(null);
  return (
    <div>
      <div className="titre-ligne">
        <h2>Cartes cadeaux et bons d'avoir</h2>
        <span className="boutons-ligne">
          {peut("caisse.encaisser") && (
            <button className="principal" onClick={() => setVente(true)}>
              Vendre une carte cadeau
            </button>
          )}
          {peut("commande.offrir") && <button onClick={() => setAvoir(true)}>Offrir un bon d'avoir</button>}
        </span>
      </div>
      <p className="aide">
        Une carte cadeau est payée d'avance : l'argent entre en caisse, mais la vente n'est comptée que lorsque la carte paie une addition. Un bon d'avoir est
        un geste du restaurant (client mécontent) : aucun argent n'entre.
      </p>
      <TableauDonnees
        colonnes={["Code", "Type", "Pour", "Valeur", "Solde", "Valable jusqu'au", ""]}
        lignes={(donnees ?? []).map((k) => [
          k.code,
          t(k.type),
          k.client_nom ?? k.beneficiaire,
          fcfa(k.montant_initial),
          <Montant valeur={k.solde} fort />,
          k.expire_le ? dateFr(k.expire_le) : "Sans fin",
          <button className="petit" onClick={() => setAffichee(k)}>
            Voir
          </button>,
        ])}
      />
      {vente && <VenteCarte fermer={() => setVente(false)} fait={(k) => (recharger(), setAffichee(k))} />}
      {avoir && <OffrirAvoir fermer={() => setAvoir(false)} fait={(k) => (recharger(), setAffichee(k))} />}
      {affichee && <CarteAffichee k={affichee} fermer={() => setAffichee(null)} />}
    </div>
  );
}

/** Carte à remettre au client : code en gros, imprimable. */
function CarteAffichee({ k, fermer }: { k: Carte; fermer: () => void }) {
  const { etat } = useApp();
  return (
    <Modal titre={k.type === "avoir" ? "Bon d'avoir" : "Carte cadeau"} fermer={fermer}>
      <div className="imprimable">
        <h3>{etat?.restaurant}</h3>
        <p>
          {k.type === "avoir" ? "Bon d'avoir" : "Carte cadeau"} de <strong>{fcfa(k.montant_initial)}</strong>
          {(k.client_nom || k.beneficiaire) && <> pour {k.client_nom || k.beneficiaire}</>}
        </p>
        <p className="code-carte" aria-label="Code de la carte">
          {k.code}
        </p>
        <p>
          Solde : <strong>{fcfa(k.solde)}</strong> — {k.expire_le ? `valable jusqu'au ${dateFr(k.expire_le)}` : "sans date de fin"}
        </p>
        <p className="aide">Présentez ce code à la caisse pour payer. Il se garde comme de l'argent.</p>
      </div>
      <div className="actions">
        <button onClick={() => window.print()}>Imprimer</button>
        <button className="principal" onClick={fermer}>
          Fermer
        </button>
      </div>
    </Modal>
  );
}

function VenteCarte({ fermer, fait }: { fermer: () => void; fait: (k: Carte) => void }) {
  const { agir } = useApp();
  const { donnees: comptes } = useDonnees(() => get<{ comptes: Compte[] }>("/caisse").then((c) => c.comptes), []);
  const mm = (comptes ?? []).filter((c) => c.type === "mobile_money" && c.actif);
  const [montant, setMontant] = useState(0);
  const [moyen, setMoyen] = useState("especes");
  const [compte, setCompte] = useState("");
  const [reference, setReference] = useState("");
  const [pour, setPour] = useState("");
  const [fin, setFin] = useState("");
  const compteId = compte || mm[0]?.id || "";
  const valide = montant > 0 && (moyen === "especes" || (!!compteId && !!reference.trim()));
  return (
    <Modal titre="Vendre une carte cadeau" fermer={fermer}>
      <ChampMontant libelle="Montant de la carte" valeur={montant} changer={setMontant} raccourcis={[5_000, 10_000, 25_000, 50_000]} autoFocus />
      <Champ libelle="Pour (facultatif)" valeur={pour} changer={setPour} placeholder="Nom de la personne qui recevra la carte" />
      <Champ libelle="Valable jusqu'au (facultatif)" type="date" valeur={fin} changer={setFin} />
      <Choix
        libelle="Payée en"
        valeur={moyen}
        changer={setMoyen}
        options={[{ valeur: "especes", libelle: "Espèces (dans ma caisse)" }, ...(mm.length ? [{ valeur: "mobile_money", libelle: "Mobile Money" }] : [])]}
      />
      {moyen === "mobile_money" && (
        <>
          <Choix libelle="Opérateur" valeur={compteId} changer={setCompte} options={mm.map((c) => ({ valeur: c.id, libelle: c.nom }))} />
          <Champ libelle="Référence de la transaction" valeur={reference} changer={setReference} obligatoire />
        </>
      )}
      <div className="actions">
        <button onClick={fermer}>Annuler</button>
        <button
          className="principal"
          disabled={!valide}
          onClick={() =>
            agir(
              (pin) =>
                post<Carte>(
                  "/cartes/vendre",
                  { montant, moyen, compte_id: moyen === "mobile_money" ? compteId : null, reference, beneficiaire: pour, expire_le: fin || null },
                  pin,
                ),
              "Carte vendue",
            ).then((k) => {
              if (k) {
                fermer();
                fait(k);
              }
            })
          }
        >
          Vendre la carte
        </button>
      </div>
    </Modal>
  );
}

function OffrirAvoir({ fermer, fait }: { fermer: () => void; fait: (k: Carte) => void }) {
  const { agir } = useApp();
  const [q, setQ] = useState("");
  const { donnees: clients } = useDonnees(() => get<Client[]>(`/clients?q=${encodeURIComponent(q)}`), [], [q]);
  const [client, setClient] = useState<Client | null>(null);
  const [montant, setMontant] = useState(0);
  const [motif, setMotif] = useState("");
  const [fin, setFin] = useState("");
  return (
    <Modal titre="Offrir un bon d'avoir" fermer={fermer}>
      <p className="aide">Geste du restaurant pour un client mécontent. Le motif est obligatoire et l'accord du gérant (son PIN) est demandé.</p>
      {client ? (
        <p>
          Pour : <strong>{client.nom}</strong>{" "}
          <button className="petit" onClick={() => setClient(null)}>
            Changer
          </button>
        </p>
      ) : (
        <>
          <Champ libelle="Client (nom ou téléphone)" valeur={q} changer={setQ} autoFocus />
          <div className="liste-commandes">
            {(clients ?? []).slice(0, 6).map((c) => (
              <button key={c.id} className="ligne-commande" onClick={() => setClient(c)}>
                <strong>{c.nom}</strong>
                <span>{c.telephone}</span>
              </button>
            ))}
          </div>
        </>
      )}
      <ChampMontant libelle="Montant du bon" valeur={montant} changer={setMontant} raccourcis={[1_000, 2_000, 5_000]} />
      <Champ libelle="Motif" valeur={motif} changer={setMotif} placeholder="Plat arrivé froid, attente trop longue…" obligatoire />
      <Champ libelle="Valable jusqu'au (facultatif)" type="date" valeur={fin} changer={setFin} />
      <div className="actions">
        <button onClick={fermer}>Annuler</button>
        <button
          className="principal"
          disabled={!client || montant <= 0 || !motif.trim()}
          onClick={() =>
            agir((pin) => post<Carte>("/cartes/avoir", { montant, client_id: client?.id, motif, expire_le: fin || null }, pin), "Bon d'avoir créé").then(
              (k) => {
                if (k) {
                  fermer();
                  fait(k);
                }
              },
            )
          }
        >
          Offrir le bon
        </button>
      </div>
    </Modal>
  );
}

/** Contrats avec des sociétés qui prennent en charge une part des repas de leurs employés (RG-SOC-01 à 04). */
export function Societes() {
  const { peut } = useApp();
  const { donnees, recharger } = useDonnees(() => get<Contrat[]>("/contrats"), ["caisse", "paiement"]);
  const [edition, setEdition] = useState<Contrat | null>(null);
  const [releve, setReleve] = useState<Contrat | null>(null);
  const nouveau: Contrat = { id: "", client_id: "", client_nom: "", nom: "", type_prise: "pourcentage", valeur: 50, plafond_repas: 0, actif: true, dette: 0 };
  return (
    <div>
      <div className="titre-ligne">
        <h2>Sociétés sous contrat</h2>
        {peut("client.depasser_limite") && (
          <button className="principal" onClick={() => setEdition(nouveau)}>
            + Contrat
          </button>
        )}
      </div>
      <p className="aide">
        La société paie une part du repas de ses employés (un pourcentage ou un montant). Cette part est inscrite sur son compte, à facturer ; l'employé paie le
        reste à la caisse.
      </p>
      <TableauDonnees
        colonnes={["Contrat", "Société", "Part prise en charge", "Plafond par repas", "À facturer", ""]}
        lignes={(donnees ?? []).map((c) => [
          c.actif ? c.nom : `${c.nom} (arrêté)`,
          c.client_nom,
          c.type_prise === "pourcentage" ? `${c.valeur} %` : fcfa(c.valeur),
          c.plafond_repas > 0 ? fcfa(c.plafond_repas) : "Sans plafond",
          <Montant valeur={c.dette} fort />,
          <span className="boutons-ligne">
            <button className="petit" onClick={() => setReleve(c)}>
              Relevé
            </button>
            {peut("client.depasser_limite") && (
              <button className="petit" onClick={() => setEdition(c)}>
                Modifier
              </button>
            )}
          </span>,
        ])}
      />
      {edition && <FormContrat c={edition} fermer={() => setEdition(null)} fait={recharger} />}
      {releve && <ReleveContrat c={releve} fermer={() => setReleve(null)} />}
    </div>
  );
}

function FormContrat({ c, fermer, fait }: { c: Contrat; fermer: () => void; fait: () => void }) {
  const { agir } = useApp();
  const [x, setX] = useState(c);
  const { donnees: clients } = useDonnees(() => get<Client[]>("/clients?q="), []);
  const societes = (clients ?? []).filter((k) => k.credit_autorise);
  return (
    <Modal titre={c.id ? c.nom : "Nouveau contrat"} fermer={fermer}>
      <Champ
        libelle="Nom du contrat"
        valeur={x.nom}
        changer={(v) => setX({ ...x, nom: v })}
        placeholder="Société X — repas du personnel"
        obligatoire
        autoFocus
      />
      <Choix
        libelle="Société (cliente à crédit)"
        valeur={x.client_id}
        changer={(v) => setX({ ...x, client_id: v })}
        options={[{ valeur: "", libelle: "Choisir…" }, ...societes.map((k) => ({ valeur: k.id, libelle: k.nom }))]}
      />
      <p className="aide">La société doit d'abord être créée dans « Clients » avec le crédit autorisé : c'est sur son compte que sa part est inscrite.</p>
      <Choix
        libelle="La société prend en charge"
        valeur={x.type_prise}
        changer={(v) => setX({ ...x, type_prise: v, valeur: v === "pourcentage" ? 50 : 1_000 })}
        options={[
          { valeur: "pourcentage", libelle: "Un pourcentage du repas" },
          { valeur: "montant", libelle: "Un montant fixe par repas" },
        ]}
      />
      {x.type_prise === "pourcentage" ? (
        <label className="champ">
          <span>Pourcentage pris en charge (1 à 100)</span>
          <input type="number" min={1} max={100} value={x.valeur} onChange={(e) => setX({ ...x, valeur: Number(e.target.value) })} />
        </label>
      ) : (
        <ChampMontant libelle="Montant pris en charge par repas" valeur={x.valeur} changer={(v) => setX({ ...x, valeur: v })} />
      )}
      <ChampMontant libelle="Plafond par repas (0 = sans plafond)" valeur={x.plafond_repas} changer={(v) => setX({ ...x, plafond_repas: v })} />
      <Case libelle="Contrat en cours" valeur={x.actif} changer={(v) => setX({ ...x, actif: v })} />
      <div className="actions">
        <button onClick={fermer}>Annuler</button>
        <button
          className="principal"
          disabled={!x.nom.trim() || !x.client_id || x.valeur <= 0}
          onClick={() => agir((pin) => post("/contrats", x, pin), "Contrat enregistré").then((r) => r !== undefined && (fait(), fermer()))}
        >
          Enregistrer
        </button>
      </div>
    </Modal>
  );
}

function ReleveContrat({ c, fermer }: { c: Contrat; fermer: () => void }) {
  const [debut, setDebut] = useState(premierDuMois());
  const [fin, setFin] = useState(finDuMois(aujourdhui()));
  const { donnees } = useDonnees(
    () =>
      get<{ lignes: { date: string; commande_numero: number; employe: string; total_repas: number; part_societe: number }[] }>(
        `/contrats/${c.id}/releve?debut=${debut}&fin=${fin}`,
      ),
    [],
    [c.id, debut, fin],
  );
  const lignes = donnees?.lignes ?? [];
  return (
    <Modal titre={`Relevé — ${c.nom}`} fermer={fermer} large>
      <div className="carte filtres periode">
        <Champ libelle="Du" type="date" valeur={debut} changer={setDebut} />
        <Champ libelle="Au" type="date" valeur={fin} changer={setFin} />
      </div>
      <div className="imprimable">
        <h3>
          {c.client_nom} — du {dateFr(debut)} au {dateFr(fin)}
        </h3>
        <TableauDonnees
          colonnes={["Date", "Commande", "Employé", "Repas", "Part de la société"]}
          lignes={lignes.map((l) => [dateFr(l.date), `n°${l.commande_numero}`, l.employe, fcfa(l.total_repas), <Montant valeur={l.part_societe} />])}
        />
        <div className="total">
          <span>À facturer à la société</span>
          <Montant valeur={lignes.reduce((s, l) => s + l.part_societe, 0)} fort />
        </div>
        <p className="aide">Édité le {dateHeure(Date.now())}</p>
      </div>
      <button onClick={() => window.print()}>Imprimer le relevé</button>
    </Modal>
  );
}
