import { Bike, Phone } from "lucide-react";
import { useState } from "react";
import { useParams } from "react-router";
import { get, post } from "../api";
import { Champ, ChampMontant, Choix, Modal, Montant, TableauDonnees } from "../composants/Base";
import { useApp, useDonnees } from "../contexte";
import { comptesHorsCaisse } from "../paiement";
import { dateFr, fcfa } from "../format";
import { t } from "../i18n";
import type { Bulletin, Compte, Employe } from "../types";
import { FormulaireEmploye, libelleMontant } from "./Employes";
import { BulletinImprimable } from "./Paie";

type LigneCompte = { seq: number; date: string; type: string; montant: number; quantite: number | null; motif: string; solde: number };
type Fiche = { employe: Employe; releve: LigneCompte[]; bulletins: Bulletin[] };

export default function FicheEmploye() {
  const { id = "" } = useParams();
  const { peut } = useApp();
  const { donnees, recharger } = useDonnees(() => get<Fiche>(`/employes/${id}`), ["employes", "caisse"], [id]);
  const [mode, setMode] = useState<"" | "modifier" | "avance" | "evenement">("");
  const [bulletin, setBulletin] = useState<Bulletin | null>(null);
  if (!donnees) return <p className="aide">Chargement…</p>;
  const e = donnees.employe;
  return (
    <div>
      <div className="titre-ligne">
        <h1>
          {e.nom} {e.surnom && `« ${e.surnom} »`}
        </h1>
        {peut("employe.gerer") && <button onClick={() => setMode("modifier")}>Modifier</button>}
      </div>
      <div className="grille-2">
        <div className="carte">
          <div className="ligne-valeur">
            <span>Fonction</span>
            <strong>{e.fonction}</strong>
          </div>
          <div className="ligne-valeur">
            <span>{libelleMontant(e.type_remuneration)}</span>
            <strong>{e.type_remuneration === "aucun" ? "Non rémunéré" : fcfa(e.montant_base)}</strong>
          </div>
          <div className="ligne-valeur">
            <span>Engagement</span>
            <strong>{e.type_contrat === "aucun" ? "Sans contrat" : t(e.type_contrat)}</strong>
          </div>
          <div className="ligne-valeur">
            <span>INPS / AMO</span>
            <strong>
              INPS : {e.declare_inps ? "oui" : "non"} / AMO : {e.affilie_amo ? "oui" : "non"}
            </strong>
          </div>
          {e.date_embauche && (
            <div className="ligne-valeur">
              <span>Arrivée</span>
              <strong>{dateFr(e.date_embauche)}</strong>
            </div>
          )}
          {e.avantages_nature && <p className="aide">Avantages : {e.avantages_nature}</p>}
          {e.telephone && (
            <p>
              <Phone size={16} className="icone-texte" aria-hidden /> {e.telephone}
            </p>
          )}
          <div className="total">
            <span>{e.solde >= 0 ? "Le restaurant lui doit" : "Il/elle doit au restaurant"}</span>
            <Montant valeur={Math.abs(e.solde)} fort />
          </div>
          <div className="menu-actions">
            {peut("employe.avance") && <button onClick={() => setMode("avance")}>Avance</button>}
            {peut("paie.gerer") && <button onClick={() => setMode("evenement")}>Prime, retenue, tâches…</button>}
          </div>
        </div>
        <div className="carte">
          <h2>Bulletins</h2>
          {donnees.bulletins.length === 0 && <p className="aide">Aucun bulletin : clôturez une période dans « Paie ».</p>}
          {donnees.bulletins.map((b) => (
            <button key={b.id} className="ligne-commande" onClick={() => setBulletin(b)}>
              <strong>n°{b.numero}</strong>
              <span>
                {dateFr(b.debut)} → {dateFr(b.fin)}
              </span>
              <span>
                Net {fcfa(b.net_a_payer)} {b.reste_a_payer > 0 ? `(reste ${fcfa(b.reste_a_payer)})` : "(payé)"}
              </span>
            </button>
          ))}
        </div>
      </div>
      {peut("employe.gerer") && (e.fonction === "livreur" || e.acces_livreur) && <AccesLivreur employe={e} fait={recharger} />}
      <div className="carte">
        <h2>Compte de l'employé</h2>
        <TableauDonnees
          colonnes={["Date", "Mouvement", "Détail", "Montant", "Solde"]}
          lignes={[...donnees.releve]
            .reverse()
            .map((l) => [
              dateFr(l.date),
              t(l.type),
              `${l.quantite ? `${l.quantite} × ` : ""}${l.motif}`,
              <Montant valeur={l.montant} />,
              <Montant valeur={l.solde} />,
            ])}
        />
      </div>
      {mode === "modifier" && <FormulaireEmploye initial={e} fermer={() => setMode("")} fait={recharger} />}
      {mode === "avance" && <AvanceModal employe={e} fermer={() => setMode("")} fait={recharger} />}
      {mode === "evenement" && <EvenementModal employe={e} fermer={() => setMode("")} fait={recharger} />}
      {bulletin && (
        <Modal titre={`Bulletin n°${bulletin.numero}`} fermer={() => setBulletin(null)} large>
          <BulletinImprimable b={bulletin} />
        </Modal>
      )}
    </div>
  );
}

/**
 * RG-LIV-05 (fiche 0047) : le livreur se connecte à l'application Youma Livreur avec son téléphone et ce PIN, et y
 * voit les courses qu'on lui assigne. Le PIN n'est jamais réaffiché : on en donne un nouveau s'il l'oublie.
 */
function AccesLivreur({ employe, fait }: { employe: Employe; fait: () => void }) {
  const { agir } = useApp();
  const [pin, setPin] = useState("");
  const valide = /^\d{4,6}$/.test(pin);
  const envoyer = (p: string | null, message: string) =>
    agir((x) => post(`/employes/${employe.id}/acces-livreur`, { pin: p }, x), message).then((r) => {
      if (r !== undefined) {
        setPin("");
        fait();
      }
    });
  return (
    <div className="carte">
      <h2>
        <Bike size={20} className="icone-texte" aria-hidden /> Application Youma Livreur
      </h2>
      {employe.acces_livreur ? (
        <p>
          Accès donné : il se connecte avec <strong>{employe.telephone}</strong> et son PIN, puis voit les courses qui lui sont assignées.
        </p>
      ) : (
        <p className="aide">
          Donnez-lui un PIN : il se connectera avec son téléphone ({employe.telephone || "à indiquer d'abord sur sa fiche"}) et verra ses courses sans lien à
          coller.
        </p>
      )}
      <div className="grille-2">
        <Champ libelle={employe.acces_livreur ? "Nouveau PIN (4 à 6 chiffres)" : "PIN du livreur (4 à 6 chiffres)"} valeur={pin} changer={setPin} type="tel" />
        <button className="principal" disabled={!valide} onClick={() => envoyer(pin, "PIN du livreur enregistré")}>
          {employe.acces_livreur ? "Changer le PIN" : "Donner l'accès"}
        </button>
      </div>
      {employe.acces_livreur && <button onClick={() => envoyer(null, "Accès retiré")}>Retirer l'accès</button>}
    </div>
  );
}

function AvanceModal({ employe, fermer, fait }: { employe: Employe; fermer: () => void; fait: () => void }) {
  const { agir } = useApp();
  const { donnees: caisse } = useDonnees(() => get<{ session: { compte_id: string } | null; comptes: Compte[] }>("/caisse"), []);
  const [montant, setMontant] = useState(0);
  const [motif, setMotif] = useState("");
  // Deux possibilités (fiche 0027) : le tiroir de sa caisse (compte dans la clôture), ou un compte hors caisse.
  const horsCaisse = comptesHorsCaisse(caisse?.comptes ?? []);
  const [choix, setCompte] = useState<string | null>(null);
  const compte = choix ?? (caisse?.session ? "" : (horsCaisse[0]?.id ?? ""));
  return (
    <Modal titre={`Avance à ${employe.nom}`} fermer={fermer}>
      <p className="aide">L'avance sera déduite à la prochaine paie. Au-delà du plafond, le propriétaire doit autoriser.</p>
      <ChampMontant libelle="Montant" valeur={montant} changer={setMontant} autoFocus />
      <Choix
        libelle="Payée depuis"
        valeur={compte}
        changer={setCompte}
        options={[...(caisse?.session ? [{ valeur: "", libelle: "Tiroir de ma caisse" }] : []), ...horsCaisse.map((c) => ({ valeur: c.id, libelle: c.nom }))]}
      />
      <p className="aide">
        {compte === "" ? "Sortie du tiroir : l'avance compte dans la clôture de votre caisse." : "Hors caisse : la clôture de caisse n'en dépend pas."}
      </p>
      <Champ libelle="Motif" valeur={motif} changer={setMotif} placeholder="Maladie, transport, fête…" />
      <div className="actions">
        <button onClick={fermer}>Annuler</button>
        <button
          className="principal"
          disabled={montant <= 0 || (compte === "" && !caisse?.session)}
          onClick={() =>
            agir((pin) => post("/employes/avance", { employe_id: employe.id, montant, motif, compte_id: compte || null }, pin), "Avance enregistrée").then(
              (r) => r !== undefined && (fait(), fermer()),
            )
          }
        >
          Donner l'avance
        </button>
      </div>
    </Modal>
  );
}

function EvenementModal({ employe, fermer, fait }: { employe: Employe; fermer: () => void; fait: () => void }) {
  const { agir } = useApp();
  const [type, setType] = useState(employe.type_remuneration === "tache" ? "tache" : "prime");
  const [montant, setMontant] = useState(0);
  const [quantite, setQuantite] = useState(1);
  const [motif, setMotif] = useState("");
  return (
    <Modal titre={employe.nom} fermer={fermer}>
      <Choix
        libelle="Type"
        valeur={type}
        changer={setType}
        options={[
          { valeur: "prime", libelle: "Prime" },
          { valeur: "retenue", libelle: "Retenue (casse, manquant…)" },
          { valeur: "tache", libelle: "Tâches / courses effectuées" },
          { valeur: "regularisation", libelle: "Régularisation (+ ou −)" },
        ]}
      />
      {type === "tache" ? (
        <>
          <label className="champ">
            <span>Nombre</span>
            <input type="number" min={1} value={quantite} onChange={(e) => setQuantite(Number(e.target.value))} />
          </label>
          <ChampMontant libelle={`Prix unitaire (vide = ${fcfa(employe.montant_base)})`} valeur={montant} changer={setMontant} />
        </>
      ) : (
        <ChampMontant libelle="Montant" valeur={montant} changer={setMontant} />
      )}
      {type === "regularisation" && <p className="aide">Pour une régularisation négative, saisissez « -2000 ».</p>}
      <Champ libelle="Motif" valeur={motif} changer={setMotif} obligatoire={type !== "tache"} />
      <div className="actions">
        <button onClick={fermer}>Annuler</button>
        <button
          className="principal"
          onClick={() =>
            agir(
              (pin) => post("/employes/evenement", { employe_id: employe.id, type, montant, quantite: type === "tache" ? quantite : null, motif }, pin),
              "Enregistré",
            ).then((r) => r !== undefined && (fait(), fermer()))
          }
        >
          Enregistrer
        </button>
      </div>
    </Modal>
  );
}
