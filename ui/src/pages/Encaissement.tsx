import { useEffect, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";
import { get, post } from "../api";
import { Champ, ChampMontant, Modal, Montant } from "../composants/Base";
import { useApp, useDonnees } from "../contexte";
import { fcfa, nombre } from "../format";
import { t } from "../i18n";
import { billetsProposes, Part, rendu, sommeParts, verifierPaiement } from "../paiement";
import type { Client, Commande, Compte, SessionCaisse } from "../types";

type Resultat = { numero: number; montant: number; especes_recues: number; rendu: number; reste: number; commande_payee: boolean; bon_sortie: [number, string] | null };

/** Encaissement standard en moins de 5 secondes : « Espèces » puis « Valider ». */
export default function Encaissement() {
  const { id = "" } = useParams();
  const nav = useNavigate();
  const { agir, etat } = useApp();
  const { donnees: cmd, recharger } = useDonnees(() => get<Commande>(`/commandes/${id}`), ["paiement"], [id]);
  const { donnees: caisse } = useDonnees(() => get<{ session: SessionCaisse | null; comptes: Compte[] }>("/caisse"), ["caisse"]);
  const [aPayer, setAPayer] = useState(0);
  const [parts, setParts] = useState<Part[]>([]);
  const [recues, setRecues] = useState(0);
  const [client, setClient] = useState<Client | null>(null);
  const [choixClient, setChoixClient] = useState(false);
  const [resultat, setResultat] = useState<Resultat | null>(null);
  const [nbParts, setNbParts] = useState(0);
  const [envoi, setEnvoi] = useState(false);

  useEffect(() => {
    if (cmd) setAPayer(cmd.totaux.reste);
  }, [cmd]);
  useEffect(() => {
    if (cmd?.client_id && !client) get<{ client: Client }>(`/clients/${cmd.client_id}`).then((r) => setClient(r.client)).catch(() => {});
  }, [cmd, client]);

  if (!cmd || !caisse) return <p className="aide">Chargement…</p>;
  if (!caisse.session)
    return (
      <div className="carte">
        <p>Vous n'avez pas de session de caisse ouverte (RG-CAI-01).</p>
        <button className="principal grand" onClick={() => nav("/caisse")}>
          Ouvrir ma caisse
        </button>
      </div>
    );

  const mm = caisse.comptes.filter((c) => c.type === "mobile_money" && c.actif);
  const reste = aPayer - sommeParts(parts);
  const refObligatoire = etat?.parametres.reference_mm_obligatoire ?? true;
  const erreur = verifierPaiement(parts, cmd.totaux.reste, recues, refObligatoire);
  const livraison = cmd.type === "livraison" && !!cmd.livreur_id;

  const ajouterPart = (p: Part) => setParts((x) => [...x, { ...p, montant: p.montant > 0 ? p.montant : Math.max(0, reste) }]);
  const modifierPart = (i: number, p: Partial<Part>) => setParts((x) => x.map((a, j) => (j === i ? { ...a, ...p } : a)));

  const valider = async () => {
    if (erreur || envoi) return;
    setEnvoi(true);
    const r = await agir((pin) => post<Resultat>("/caisse/encaisser", { commande_id: id, parts, especes_recues: recues || null }, pin));
    setEnvoi(false);
    if (r) {
      setResultat(r);
      setParts([]);
      setRecues(0);
      recharger();
    }
  };

  if (resultat)
    return (
      <div className="carte resultat-paiement">
        <h1>Reçu n°{resultat.numero}</h1>
        <div className="ligne-valeur">
          <span>Montant payé</span>
          <strong>{fcfa(resultat.montant)}</strong>
        </div>
        {resultat.especes_recues > 0 && (
          <>
            <div className="ligne-valeur">
              <span>Argent reçu du client</span>
              <strong>{fcfa(resultat.especes_recues)}</strong>
            </div>
            <p className="rendu">
              Monnaie à rendre : <strong>{fcfa(resultat.rendu)}</strong>
            </p>
          </>
        )}
        {resultat.commande_payee ? <p>Addition soldée.</p> : <p>Reste à payer : {fcfa(resultat.reste)}</p>}
        {resultat.bon_sortie && (
          <p className="bon-sortie">
            Bon de sortie n°<strong>{resultat.bon_sortie[0]}</strong> — code <strong>{resultat.bon_sortie[1]}</strong>
          </p>
        )}
        <div className="actions">
          <button onClick={() => agir(() => post(`/commandes/${id}/imprimer`), "Ticket envoyé à l'imprimante")}>Imprimer le ticket (bon de sortie)</button>
          {!resultat.commande_payee && <button onClick={() => setResultat(null)}>Encaisser le reste</button>}
          <button className="principal grand" onClick={() => nav(cmd.table_id || resultat.commande_payee ? "/salle" : `/commande/${id}`)}>
            Terminé
          </button>
        </div>
      </div>
    );

  const especes = parts.filter((p) => p.moyen === "especes").reduce((s, p) => s + p.montant, 0);

  return (
    <div className="encaissement">
      <div className="titre-ligne">
        <h1>
          Encaisser — {cmd.table_nom ? `table ${cmd.table_nom}` : `${t(cmd.type)} n°${cmd.numero}`}
        </h1>
        <button onClick={() => nav(`/commande/${id}`)}>← Addition</button>
      </div>
      <div className="grille-2">
        <div className="carte">
          <div className="total">
            <span>Reste dû sur l'addition</span>
            <Montant valeur={cmd.totaux.reste} fort />
          </div>
          <div className="division">
            <span>Diviser :</span>
            {[2, 3, 4, 5].map((n) => (
              <button
                key={n}
                className={nbParts === n ? "actif" : ""}
                onClick={async () => {
                  const r = await get<number[]>(`/commandes/${id}/diviser?parts=${n}`);
                  setNbParts(n);
                  setAPayer(r[0]);
                  setParts([]);
                }}
              >
                {n} parts
              </button>
            ))}
            {nbParts > 0 && (
              <button
                onClick={() => {
                  setNbParts(0);
                  setAPayer(cmd.totaux.reste);
                }}
              >
                Tout
              </button>
            )}
          </div>
          <ChampMontant libelle="À encaisser maintenant" valeur={aPayer} changer={(v) => setAPayer(Math.min(v, cmd.totaux.reste))} />
          <h3>Moyen de paiement</h3>
          <div className="moyens">
            <button className="principal grand" onClick={() => ajouterPart({ moyen: "especes", montant: 0 })} disabled={reste <= 0}>
              💵 Espèces
            </button>
            {livraison && (
              <button className="grand" onClick={() => ajouterPart({ moyen: "especes", montant: 0, par_livreur: true })} disabled={reste <= 0}>
                🛵 Espèces au livreur
              </button>
            )}
            {mm.map((c) => (
              <button key={c.id} className="grand mm" onClick={() => ajouterPart({ moyen: "mobile_money", montant: 0, compte_id: c.id })} disabled={reste <= 0}>
                📱 {c.nom}
              </button>
            ))}
            <button
              className="grand"
              onClick={() => (client ? ajouterPart({ moyen: "credit", montant: 0, client_id: client.id }) : setChoixClient(true))}
              disabled={reste <= 0}
            >
              🧾 Crédit {client ? `(${client.nom})` : ""}
            </button>
          </div>
        </div>
        <div className="carte">
          <h3>Répartition</h3>
          {parts.length === 0 && <p className="aide">Touchez un moyen de paiement.</p>}
          {parts.map((p, i) => (
            <div key={i} className="part">
              <div className="part-entete">
                <strong>
                  {p.moyen === "mobile_money" ? caisse.comptes.find((c) => c.id === p.compte_id)?.nom : t(p.moyen)}
                  {p.par_livreur && " (livreur)"}
                </strong>
                <button className="petit" onClick={() => setParts((x) => x.filter((_, j) => j !== i))} aria-label="Retirer ce paiement">
                  ✕
                </button>
              </div>
              <ChampMontant libelle="Montant" valeur={p.montant} changer={(v) => modifierPart(i, { montant: v })} />
              {p.moyen === "mobile_money" && (
                <>
                  <Champ libelle="Référence de la transaction" valeur={p.reference ?? ""} changer={(v) => modifierPart(i, { reference: v })} obligatoire={refObligatoire} />
                  <Champ libelle="Numéro du payeur" valeur={p.numero_payeur ?? ""} changer={(v) => modifierPart(i, { numero_payeur: v })} type="tel" />
                  <p className="aide">⚠️ Vérifiez le SMS de l'opérateur sur le téléphone du restaurant, pas la capture du client.</p>
                </>
              )}
            </div>
          ))}
          {especes > 0 && (
            <ChampMontant libelle="Espèces reçues du client" valeur={recues} changer={setRecues} raccourcis={billetsProposes(especes)} />
          )}
          {especes > 0 && recues > especes && (
            <p className="rendu">
              Rendu : <strong>{fcfa(rendu(parts, recues))}</strong>
            </p>
          )}
          <div className="total">
            <span>Total saisi</span>
            <span>
              {nombre(sommeParts(parts))} / {nombre(aPayer)}
            </span>
          </div>
          {erreur && parts.length > 0 && <p className="erreur-texte">{erreur}</p>}
          <button className="principal tres-grand" disabled={!!erreur || envoi} onClick={valider}>
            Valider le paiement
          </button>
        </div>
      </div>
      {choixClient && (
        <ChoixClient
          fermer={() => setChoixClient(false)}
          choisir={(c) => {
            setClient(c);
            setChoixClient(false);
            agir((pin) => post(`/commandes/${id}/client`, { client_id: c.id }, pin));
            ajouterPart({ moyen: "credit", montant: 0, client_id: c.id });
          }}
        />
      )}
    </div>
  );
}

function ChoixClient({ fermer, choisir }: { fermer: () => void; choisir: (c: Client) => void }) {
  const [q, setQ] = useState("");
  const [liste, setListe] = useState<Client[]>([]);
  useEffect(() => {
    get<Client[]>(`/clients?q=${encodeURIComponent(q)}`).then(setListe).catch(() => {});
  }, [q]);
  return (
    <Modal titre="Vente à crédit : quel client ?" fermer={fermer}>
      <Champ libelle="Nom ou téléphone" valeur={q} changer={setQ} autoFocus />
      <div className="liste-commandes">
        {liste.map((c) => (
          <button key={c.id} className="ligne-commande" disabled={!c.credit_autorise} onClick={() => choisir(c)}>
            <strong>{c.nom}</strong>
            <span>{c.telephone}</span>
            <span>{c.credit_autorise ? `Dette ${fcfa(c.dette)} / limite ${fcfa(c.limite_credit)}` : "Crédit non autorisé"}</span>
          </button>
        ))}
      </div>
    </Modal>
  );
}
