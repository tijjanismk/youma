import { AlertTriangle, ArrowLeft, Banknote, Bike, Building2, CreditCard, Gift, HandCoins, Smartphone, Star, X } from "lucide-react";
import { useEffect, useState } from "react";
import { useNavigate, useParams } from "react-router";
import { get, post } from "../api";
import { Champ, ChampMontant, Modal, Montant } from "../composants/Base";
import { TicketImprimable, TicketWhatsApp } from "../composants/Ticket";
import { useApp, useDonnees } from "../contexte";
import { fcfa, nombre } from "../format";
import { t } from "../i18n";
import { billetsProposes, Part, rendu, sommeParts, verifierPaiement } from "../paiement";
import type { Carte, Client, Commande, Compte, Contrat, EtatFidelite, SessionCaisse } from "../types";

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
  const [choixClient, setChoixClient] = useState<"credit" | "fidelite" | null>(null);
  const [points, setPoints] = useState(false);
  const { donnees: contrats } = useDonnees(() => get<Contrat[]>("/contrats").catch(() => [] as Contrat[]), []);
  const fideliteActive = etat?.parametres.fidelite?.active ?? false;
  const [resultat, setResultat] = useState<Resultat | null>(null);
  const [nbParts, setNbParts] = useState(0);
  const [envoi, setEnvoi] = useState(false);
  // Ticket de caisse (bon de sortie) pour Ctrl+P et WhatsApp, chargé après le paiement.
  const [ticket, setTicket] = useState("");

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
  // RG-CAI-15 : carte passée sur le TPE du restaurant (non relié), encaissée sur son compte bancaire.
  const banques = caisse.comptes.filter((c) => c.type === "banque" && c.actif);
  const reste = aPayer - sommeParts(parts);
  const refObligatoire = etat?.parametres.reference_mm_obligatoire ?? true;
  const erreur = verifierPaiement(parts, cmd.totaux.reste, recues, refObligatoire);
  const livraison = cmd.type === "livraison" && !!cmd.livreur_id;
  const societes = (contrats ?? []).filter((c) => c.actif);
  // RG-SOC-02 : part de la société sur ce repas (le serveur revalide).
  const partSociete = (c: Contrat) => {
    const brute = c.type_prise === "pourcentage" ? Math.floor((cmd.totaux.total * c.valeur) / 100) : c.valeur;
    return Math.min(c.plafond_repas > 0 ? Math.min(brute, c.plafond_repas) : brute, Math.max(0, reste));
  };

  const ajouterPart = (p: Part) => setParts((x) => [...x, { ...p, montant: p.montant > 0 ? p.montant : Math.max(0, reste) }]);
  const modifierPart = (i: number, p: Partial<Part>) => setParts((x) => x.map((a, j) => (j === i ? { ...a, ...p } : a)));

  const valider = async () => {
    if (erreur || envoi) return;
    setEnvoi(true);
    const r = await agir((pin) => post<Resultat>("/caisse/encaisser", { commande_id: id, parts, especes_recues: recues || null }, pin));
    setEnvoi(false);
    if (r) {
      setResultat(r);
      get<string>(`/commandes/${id}/ticket`)
        .then(setTicket)
        .catch(() => setTicket(""));
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
        {ticket && <TicketImprimable texte={ticket} />}
        <div className="actions">
          <button onClick={() => agir(() => post(`/commandes/${id}/imprimer`), "Ticket envoyé à l'imprimante")}>Imprimer le ticket (bon de sortie)</button>
          {ticket && <button onClick={() => window.print()}>Imprimer (navigateur)</button>}
          {ticket && <TicketWhatsApp texte={ticket} telephone={cmd.livraison_telephone ?? client?.telephone} />}
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
        <button onClick={() => nav(`/commande/${id}`)}>
          <ArrowLeft size={18} aria-hidden /> Addition
        </button>
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
              <Banknote size={20} aria-hidden /> Espèces
            </button>
            {livraison && (
              <button className="grand" onClick={() => ajouterPart({ moyen: "especes", montant: 0, par_livreur: true })} disabled={reste <= 0}>
                <Bike size={20} aria-hidden /> Espèces au livreur
              </button>
            )}
            {mm.map((c) => (
              <button key={c.id} className="grand mm" onClick={() => ajouterPart({ moyen: "mobile_money", montant: 0, compte_id: c.id })} disabled={reste <= 0}>
                <Smartphone size={20} aria-hidden /> {c.nom}
              </button>
            ))}
            {banques.map((c) => (
              <button key={c.id} className="grand" onClick={() => ajouterPart({ moyen: "carte", montant: 0, compte_id: c.id })} disabled={reste <= 0}>
                <CreditCard size={20} aria-hidden /> {banques.length > 1 ? `Carte — ${c.nom}` : "Carte (TPE)"}
              </button>
            ))}
            <button
              className="grand"
              onClick={() => (client ? ajouterPart({ moyen: "credit", montant: 0, client_id: client.id }) : setChoixClient("credit"))}
              disabled={reste <= 0}
            >
              <HandCoins size={20} aria-hidden /> Crédit {client ? `(${client.nom})` : ""}
            </button>
            <button className="grand" onClick={() => ajouterPart({ moyen: "carte_cadeau", montant: 0 })} disabled={reste <= 0}>
              <Gift size={20} aria-hidden /> Carte cadeau / bon d'avoir
            </button>
            {societes.map((c) => (
              <button
                key={c.id}
                className="grand"
                onClick={() => ajouterPart({ moyen: "credit", montant: partSociete(c), contrat_id: c.id, reference: "" })}
                disabled={reste <= 0}
              >
                <Building2 size={20} aria-hidden /> Société : {c.client_nom}
              </button>
            ))}
            {fideliteActive && (
              <button className="grand" onClick={() => (client ? setPoints(true) : setChoixClient("fidelite"))} disabled={cmd.totaux.reste <= 0}>
                <Star size={20} aria-hidden /> Points de fidélité
              </button>
            )}
          </div>
        </div>
        <div className="carte">
          <h3>Répartition</h3>
          {parts.length === 0 && <p className="aide">Touchez un moyen de paiement.</p>}
          {parts.map((p, i) => (
            <div key={i} className="part">
              <div className="part-entete">
                <strong>
                  {p.moyen === "mobile_money"
                    ? caisse.comptes.find((c) => c.id === p.compte_id)?.nom
                    : p.contrat_id
                      ? `Société : ${societes.find((c) => c.id === p.contrat_id)?.client_nom ?? ""}`
                      : t(p.moyen)}
                  {p.moyen === "carte" && banques.length > 1 && ` — ${caisse.comptes.find((c) => c.id === p.compte_id)?.nom}`}
                  {p.par_livreur && " (livreur)"}
                </strong>
                <button className="petit" onClick={() => setParts((x) => x.filter((_, j) => j !== i))} aria-label="Retirer ce paiement">
                  <X size={16} aria-hidden />
                </button>
              </div>
              <ChampMontant libelle="Montant" valeur={p.montant} changer={(v) => modifierPart(i, { montant: v })} />
              {p.moyen === "carte" && (
                <>
                  <Champ
                    libelle="Numéro d'autorisation (ticket du TPE)"
                    valeur={p.reference ?? ""}
                    changer={(v) => modifierPart(i, { reference: v })}
                    obligatoire
                  />
                  <p className="aide">Passez la carte sur le TPE, attendez « accepté », puis recopiez le numéro d'autorisation de son ticket.</p>
                </>
              )}
              {p.moyen === "carte_cadeau" && <PartCarte part={p} changer={(x) => modifierPart(i, x)} />}
              {p.contrat_id && (
                <>
                  <Champ libelle="Nom de l'employé de la société" valeur={p.reference ?? ""} changer={(v) => modifierPart(i, { reference: v })} obligatoire />
                  <p className="aide">La part de la société est inscrite sur son compte (à facturer). L'employé paie le reste avec un autre moyen.</p>
                </>
              )}
              {p.moyen === "mobile_money" && (
                <>
                  <Champ libelle="Référence de la transaction" valeur={p.reference ?? ""} changer={(v) => modifierPart(i, { reference: v })} obligatoire={refObligatoire} />
                  <Champ libelle="Numéro du payeur" valeur={p.numero_payeur ?? ""} changer={(v) => modifierPart(i, { numero_payeur: v })} type="tel" />
                  <p className="aide">
                    <AlertTriangle size={16} className="icone-texte" aria-hidden /> Vérifiez le SMS de l'opérateur sur le téléphone du restaurant, pas la
                    capture du client.
                  </p>
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
          credit={choixClient === "credit"}
          fermer={() => setChoixClient(null)}
          choisir={(c) => {
            setClient(c);
            setChoixClient(null);
            agir((pin) => post(`/commandes/${id}/client`, { client_id: c.id }, pin));
            if (choixClient === "credit") ajouterPart({ moyen: "credit", montant: 0, client_id: c.id });
            else setPoints(true);
          }}
        />
      )}
      {points && client && (
        <UtiliserPoints
          commande={id}
          client={client}
          reste={cmd.totaux.reste}
          fermer={() => setPoints(false)}
          fait={() => {
            setPoints(false);
            setParts([]);
            setNbParts(0);
            recharger();
          }}
        />
      )}
    </div>
  );
}

/** RG-CAD-02 : code de la carte, avec son solde (vérifié sur demande) ; le montant ne dépasse pas le solde. */
function PartCarte({ part, changer }: { part: Part; changer: (p: Partial<Part>) => void }) {
  const [carte, setCarte] = useState<Carte | null>(null);
  const [erreur, setErreur] = useState("");
  const verifier = () =>
    get<Carte>(`/cartes/code/${encodeURIComponent(part.reference ?? "")}`)
      .then((k) => {
        setCarte(k);
        setErreur("");
        changer({ montant: Math.min(part.montant || k.solde, k.solde) });
      })
      .catch((e: Error) => {
        setCarte(null);
        setErreur(e.message);
      });
  return (
    <>
      <Champ libelle="Code de la carte" valeur={part.reference ?? ""} changer={(v) => changer({ reference: v })} placeholder="AB12-CD34" obligatoire />
      <button className="petit" onClick={verifier} disabled={!(part.reference ?? "").trim()}>
        Vérifier le solde
      </button>
      {carte && (
        <p className="aide">
          {t(carte.type)} de {carte.client_nom ?? (carte.beneficiaire || "—")} : solde <strong>{fcfa(carte.solde)}</strong>
          {carte.expire_le ? `, valable jusqu'au ${carte.expire_le.split("-").reverse().join("/")}` : ""}
        </p>
      )}
      {erreur && <p className="erreur-texte">{erreur}</p>}
    </>
  );
}

/** RG-FID-03 : des points deviennent une remise sur l'addition du client. */
function UtiliserPoints({ commande, client, reste, fermer, fait }: { commande: string; client: Client; reste: number; fermer: () => void; fait: () => void }) {
  const { agir } = useApp();
  const { donnees } = useDonnees(() => get<{ fidelite: EtatFidelite }>(`/clients/${client.id}`).then((r) => r.fidelite), [], [client.id]);
  const [points, setPoints] = useState(0);
  if (!donnees) return null;
  const maxPoints = Math.min(donnees.points, Math.floor(reste / donnees.valeur_point));
  return (
    <Modal titre={`Points de ${client.nom}`} fermer={fermer}>
      <p>
        <strong>{donnees.points} points</strong> = {fcfa(donnees.valeur)}. Un point vaut {fcfa(donnees.valeur_point)} ; il faut au moins {donnees.minimum_points} points pour les utiliser.
      </p>
      {maxPoints < donnees.minimum_points ? (
        <p className="attention-texte">Pas assez de points pour cette addition (au plus {maxPoints} utilisables).</p>
      ) : (
        <>
          <label className="champ">
            <span>Points à utiliser (au plus {maxPoints})</span>
            <input type="number" min={donnees.minimum_points} max={maxPoints} value={points || ""} onChange={(e) => setPoints(Number(e.target.value))} />
          </label>
          <p>
            Réduction : <strong>{fcfa(points * donnees.valeur_point)}</strong>
          </p>
        </>
      )}
      <div className="actions">
        <button onClick={fermer}>Annuler</button>
        <button
          className="principal"
          disabled={points < donnees.minimum_points || points > maxPoints}
          onClick={() => agir((pin) => post(`/commandes/${commande}/fidelite`, { points }, pin), "Points utilisés").then((r) => r !== undefined && fait())}
        >
          Utiliser {points || ""} points
        </button>
      </div>
    </Modal>
  );
}

function ChoixClient({ credit, fermer, choisir }: { credit: boolean; fermer: () => void; choisir: (c: Client) => void }) {
  const [q, setQ] = useState("");
  const [liste, setListe] = useState<Client[]>([]);
  useEffect(() => {
    get<Client[]>(`/clients?q=${encodeURIComponent(q)}`).then(setListe).catch(() => {});
  }, [q]);
  return (
    <Modal titre={credit ? "Vente à crédit : quel client ?" : "Quel client ?"} fermer={fermer}>
      <Champ libelle="Nom ou téléphone" valeur={q} changer={setQ} autoFocus />
      <div className="liste-commandes">
        {liste.map((c) => (
          <button key={c.id} className="ligne-commande" disabled={credit && !c.credit_autorise} onClick={() => choisir(c)}>
            <strong>{c.nom}</strong>
            <span>{c.telephone}</span>
            <span>
              {credit
                ? c.credit_autorise
                  ? `Dette ${fcfa(c.dette)} / limite ${fcfa(c.limite_credit)}`
                  : "Crédit non autorisé"
                : `${c.points ?? 0} points`}
            </span>
          </button>
        ))}
      </div>
    </Modal>
  );
}
