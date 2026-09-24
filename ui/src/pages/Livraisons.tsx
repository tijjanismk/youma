import { useEffect, useState } from "react";
import QRCode from "qrcode";
import { Link } from "react-router-dom";
import { get, post } from "../api";
import { ChampMontant, Choix, Modal, Montant, TableauDonnees } from "../composants/Base";
import { useApp, useDonnees } from "../contexte";
import { fcfa, heure } from "../format";
import { t } from "../i18n";
import type { CommandeResume, Employe } from "../types";

type Livreur = { employe_id: string; nom: string; a_remettre: number; livraisons_en_cours: number };
const SUIVANT: Record<string, string> = { nouvelle: "confirmee", confirmee: "en_preparation", en_preparation: "prete", assignee: "en_route", en_route: "livree" };

export default function Livraisons() {
  const { agir } = useApp();
  const { donnees: livraisons, recharger } = useDonnees(() => get<CommandeResume[]>("/livraisons"), ["livraison", "commande", "paiement"]);
  const { donnees: livreurs, recharger: rechargerLivreurs } = useDonnees(() => get<Livreur[]>("/livreurs"), ["caisse", "paiement"]);
  const { donnees: employes } = useDonnees(() => get<Employe[]>("/employes").catch(() => [] as Employe[]), []);
  const [assigner, setAssigner] = useState<CommandeResume | null>(null);
  const [remise, setRemise] = useState<Livreur | null>(null);
  const [liens, setLiens] = useState<{ numero: number; code_suivi: string; code_livreur: string | null } | null>(null);
  const ouvrirLiens = (c: CommandeResume) =>
    agir((pin) => post<{ code_suivi: string; code_livreur: string | null }>(`/commandes/${c.id}/liens`, {}, pin)).then((l) => l && setLiens({ ...l, numero: c.numero }));
  const tout = () => {
    recharger();
    rechargerLivreurs();
  };
  return (
    <div>
      <h1>Livraisons</h1>
      <TableauDonnees
        colonnes={["N°", "Heure", "Client", "Statut", "Montant", "Reste", ""]}
        lignes={(livraisons ?? []).map((c) => [
          <Link to={`/commande/${c.id}`}>n°{c.numero}</Link>,
          heure(c.cree_le),
          c.client_nom ?? "",
          t(c.livraison_statut ?? ""),
          fcfa(c.total),
          fcfa(c.reste),
          <span className="boutons-ligne">
            <button className="petit" onClick={() => ouvrirLiens(c)}>
              Suivi
            </button>
            {["nouvelle", "confirmee", "en_preparation", "prete"].includes(c.livraison_statut ?? "") && (
              <button className="petit" onClick={() => setAssigner(c)}>
                Livreur
              </button>
            )}
            {SUIVANT[c.livraison_statut ?? ""] && (
              <button className="petit principal" onClick={() => agir((pin) => post(`/livraisons/${c.id}/statut`, { statut: SUIVANT[c.livraison_statut!] }, pin)).then(tout)}>
                → {t(SUIVANT[c.livraison_statut!])}
              </button>
            )}
            {c.livraison_statut !== "livree" && (
              <button
                className="petit"
                onClick={() => {
                  const motif = prompt("Motif de l'échec ?");
                  if (motif) agir((pin) => post(`/livraisons/${c.id}/statut`, { statut: "echec", motif }, pin)).then(tout);
                }}
              >
                Échec
              </button>
            )}
          </span>,
        ])}
      />
      <h2>Argent à remettre par les livreurs</h2>
      <TableauDonnees
        colonnes={["Livreur", "En cours", "À remettre", ""]}
        lignes={(livreurs ?? []).map((l) => [
          l.nom,
          l.livraisons_en_cours,
          <Montant valeur={l.a_remettre} />,
          l.a_remettre !== 0 && (
            <button className="petit principal" onClick={() => setRemise(l)}>
              Remise en caisse
            </button>
          ),
        ])}
      />
      {assigner && (
        <ChoixLivreur
          employes={(employes ?? []).filter((e) => e.statut === "actif")}
          fermer={() => setAssigner(null)}
          choisir={(id) => agir((pin) => post(`/livraisons/${assigner.id}/assigner`, { livreur_id: id }, pin), "Livreur assigné").then(() => { setAssigner(null); tout(); })}
        />
      )}
      {remise && <RemiseLivreur l={remise} fermer={() => setRemise(null)} fait={tout} />}
      {liens && <LiensSuivi {...liens} fermer={() => setLiens(null)} />}
    </div>
  );
}

function ChoixLivreur({ employes, fermer, choisir }: { employes: Employe[]; fermer: () => void; choisir: (id: string) => void }) {
  const livreurs = [...employes].sort((a, b) => Number(b.fonction === "livreur") - Number(a.fonction === "livreur"));
  const [id, setId] = useState(livreurs[0]?.id ?? "");
  return (
    <Modal titre="Assigner un livreur" fermer={fermer}>
      <Choix libelle="Livreur" valeur={id} changer={setId} options={livreurs.map((e) => ({ valeur: e.id, libelle: `${e.nom} (${e.fonction})` }))} />
      <button className="principal" disabled={!id} onClick={() => choisir(id)}>
        Assigner
      </button>
    </Modal>
  );
}

function RemiseLivreur({ l, fermer, fait }: { l: Livreur; fermer: () => void; fait: () => void }) {
  const { agir } = useApp();
  const [remis, setRemis] = useState(l.a_remettre);
  const [motif, setMotif] = useState("");
  const ecart = remis - l.a_remettre;
  return (
    <Modal titre={`Remise de ${l.nom}`} fermer={fermer}>
      <p>
        Attendu : <strong>{fcfa(l.a_remettre)}</strong>
      </p>
      <ChampMontant libelle="Montant remis" valeur={remis} changer={setRemis} autoFocus />
      {ecart !== 0 && (
        <>
          <p className="attention-texte">Écart : {fcfa(ecart)}</p>
          <label className="champ">
            <span>Motif de l'écart</span>
            <input value={motif} onChange={(e) => setMotif(e.target.value)} />
          </label>
        </>
      )}
      <button
        className="principal"
        disabled={ecart !== 0 && !motif.trim()}
        onClick={() => agir((pin) => post(`/livreurs/${l.employe_id}/remise`, { remis, motif }, pin), "Remise enregistrée").then((r) => r !== undefined && (fait(), fermer()))}
      >
        Valider la remise
      </button>
    </Modal>
  );
}

/** Liens de suivi : le client suit sa commande ; le livreur partage sa position pendant la course. */
function LiensSuivi({ numero, code_suivi, code_livreur, fermer }: { numero: number; code_suivi: string; code_livreur: string | null; fermer: () => void }) {
  const { donnees: reseau } = useDonnees(() => get<{ adresses: string[] }>("/reseau").catch(() => null), []);
  const { etat } = useApp();
  const base = (etat?.parametres.canaux?.relais_url || reseau?.adresses[0] || `${location.origin}/`).replace(/\/?$/, "/");
  const client = `${base}suivi/${code_suivi}`;
  const livreur = code_livreur ? `${base}livreur/${code_livreur}` : null;
  const [qr, setQr] = useState("");
  useEffect(() => {
    if (livreur) QRCode.toDataURL(livreur, { width: 220, margin: 1 }).then(setQr).catch(() => setQr(""));
  }, [livreur]);
  return (
    <Modal titre={`Suivi de la commande n°${numero}`} fermer={fermer}>
      <p>
        Lien à envoyer au client (SMS, WhatsApp) : <br />
        <strong className="selectionnable">{client}</strong>
      </p>
      <a className="bouton" href={`https://wa.me/?text=${encodeURIComponent(`Suivez votre commande n°${numero} : ${client}`)}`} target="_blank" rel="noreferrer">
        Envoyer par WhatsApp
      </a>
      {livreur && (
        <div className="qr">
          <p>Le livreur scanne ce QR pour partager sa position pendant la course :</p>
          {qr && <img src={qr} alt="QR du livreur" />}
          <p className="aide selectionnable">{livreur}</p>
        </div>
      )}
    </Modal>
  );
}
