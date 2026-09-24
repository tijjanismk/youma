import { useEffect, useState } from "react";
import { get, post } from "../api";
import { Champ, ChampMontant, Modal, Montant, TableauDonnees } from "../composants/Base";
import { useApp, useDonnees } from "../contexte";
import { dateFr, fcfa, finDuMois, premierDuMois } from "../format";
import { t } from "../i18n";
import type { Bulletin, Employe } from "../types";

/** Bulletin simple, imprimable (A4 ou ticket). */
export function BulletinImprimable({ b }: { b: Bulletin }) {
  const { etat } = useApp();
  return (
    <div className="bulletin imprimable">
      <h2>{etat?.restaurant}</h2>
      <h3>
        Bulletin de paie {b.numero ? `n°${b.numero}` : "(aperçu)"} — {b.employe_nom}
      </h3>
      <p>
        {b.fonction} · {b.type_contrat === "aucun" ? "sans contrat écrit" : t(b.type_contrat)} · période du {dateFr(b.debut)} au {dateFr(b.fin)}
      </p>
      {(b.type_remuneration === "journalier" || b.jours_absence_nj > 0) && (
        <p>
          Jours travaillés : {b.jours_presents} — absences non justifiées : {b.jours_absence_nj}
        </p>
      )}
      <table className="tableau">
        <tbody>
          {b.report_precedent !== 0 && (
            <tr>
              <td>Report de la période précédente</td>
              <td className="droite">
                <Montant valeur={b.report_precedent} />
              </td>
            </tr>
          )}
          {b.lignes.map((l, i) => (
            <tr key={i}>
              <td>
                {l.libelle}
                {l.quantite ? ` (${l.quantite})` : ""}
              </td>
              <td className="droite">
                <Montant valeur={l.montant} />
              </td>
            </tr>
          ))}
          <tr className="total">
            <td>Net à payer</td>
            <td className="droite">
              <Montant valeur={b.net_a_payer} fort />
            </td>
          </tr>
          {b.paye_depuis > 0 && (
            <tr>
              <td>Payé depuis la clôture</td>
              <td className="droite">{fcfa(b.paye_depuis)}</td>
            </tr>
          )}
        </tbody>
      </table>
      {b.net_a_payer < 0 && <p className="attention-texte">Net négatif : {fcfa(-b.net_a_payer)} seront déduits de la prochaine paie (rien n'est perdu).</p>}
      {b.charges_employeur > 0 && <p className="aide">Charges patronales (information) : {fcfa(b.charges_employeur)}</p>}
      <div className="signatures">
        <span>Signature de l'employé</span>
        <span>Signature du responsable</span>
      </div>
      <button className="non-imprime" onClick={() => window.print()}>
        Imprimer
      </button>
    </div>
  );
}

export default function Paie() {
  const { agir } = useApp();
  const [debut, setDebut] = useState(premierDuMois());
  const [fin, setFin] = useState(finDuMois());
  const { donnees: employes, recharger } = useDonnees(() => get<Employe[]>("/employes"), ["employes"]);
  const [apercus, setApercus] = useState<Record<string, Bulletin | string>>({});
  const [affiche, setAffiche] = useState<Bulletin | null>(null);
  const [paiement, setPaiement] = useState<Bulletin | null>(null);

  const actifs = (employes ?? []).filter((e) => e.statut !== "parti" || e.solde !== 0);

  useEffect(() => {
    let annule = false;
    (async () => {
      const r: Record<string, Bulletin | string> = {};
      for (const e of actifs) {
        try {
          r[e.id] = await get<Bulletin>(`/paie/apercu?employe=${e.id}&debut=${debut}&fin=${fin}`);
        } catch (x) {
          r[e.id] = x instanceof Error ? x.message : String(x);
        }
      }
      if (!annule) setApercus(r);
    })();
    return () => {
      annule = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [employes, debut, fin]);

  const cloturer = (e: Employe) =>
    agir((pin) => post<Bulletin>("/paie/cloturer", { employe_id: e.id, debut, fin }, pin), `Paie de ${e.nom} clôturée`).then((b) => {
      if (b) {
        setAffiche(b);
        recharger();
      }
    });

  return (
    <div>
      <h1>Paie</h1>
      <div className="carte filtres">
        <Champ libelle="Du" type="date" valeur={debut} changer={setDebut} />
        <Champ libelle="Au" type="date" valeur={fin} changer={setFin} />
        <p className="aide">
          Mensuels : salaire fixe (absences déduites si activé). Journaliers : jours pointés présents × taux. À la tâche : tâches saisies. Les avances, retenues et
          consommations sont déduites ; un net négatif est reporté.
        </p>
      </div>
      <TableauDonnees
        colonnes={["Employé", "Mode", "Dû", "Déductions", "Net", ""]}
        lignes={actifs.map((e) => {
          const a = apercus[e.id];
          if (!a || typeof a === "string") return [e.nom, t(e.type_remuneration), "", "", a ?? "…", ""];
          return [
            e.nom,
            t(e.type_remuneration),
            <Montant valeur={a.total_gains + a.report_precedent} />,
            <Montant valeur={-(a.total_retenues + a.cotisations_salarie + a.deja_paye)} />,
            <Montant valeur={a.net_a_payer} fort />,
            <span className="boutons-ligne">
              <button className="petit" onClick={() => setAffiche(a)}>
                Détail
              </button>
              <button className="petit principal" onClick={() => cloturer(e)}>
                Clôturer
              </button>
            </span>,
          ];
        })}
      />
      <BulletinsAPayer ouvrir={setPaiement} />
      {affiche && (
        <Modal titre="Bulletin" fermer={() => setAffiche(null)} large>
          <BulletinImprimable b={affiche} />
          {affiche.id && affiche.reste_a_payer > 0 && (
            <button className="principal grand" onClick={() => { setPaiement(affiche); setAffiche(null); }}>
              Payer {fcfa(affiche.reste_a_payer)}
            </button>
          )}
        </Modal>
      )}
      {paiement && <PaiementSalaire b={paiement} fermer={() => setPaiement(null)} fait={recharger} />}
    </div>
  );
}

function BulletinsAPayer({ ouvrir }: { ouvrir: (b: Bulletin) => void }) {
  const { donnees } = useDonnees(() => get<Bulletin[]>("/paie/bulletins"), ["caisse", "employes"]);
  const aPayer = (donnees ?? []).filter((b) => b.reste_a_payer > 0);
  if (!aPayer.length) return null;
  return (
    <div className="carte">
      <h2>Bulletins restant à payer</h2>
      {aPayer.map((b) => (
        <button key={b.id} className="ligne-commande" onClick={() => ouvrir(b)}>
          <strong>{b.employe_nom}</strong>
          <span>
            {dateFr(b.debut)} → {dateFr(b.fin)}
          </span>
          <span>Reste {fcfa(b.reste_a_payer)}</span>
        </button>
      ))}
    </div>
  );
}

function PaiementSalaire({ b, fermer, fait }: { b: Bulletin; fermer: () => void; fait: () => void }) {
  const { agir } = useApp();
  const [montant, setMontant] = useState(b.reste_a_payer);
  return (
    <Modal titre={`Payer ${b.employe_nom}`} fermer={fermer}>
      <p>Reste à payer sur ce bulletin : {fcfa(b.reste_a_payer)}. Un paiement partiel est possible.</p>
      <ChampMontant libelle="Montant payé" valeur={montant} changer={setMontant} raccourcis={[b.reste_a_payer]} />
      <p className="aide">Payé depuis votre caisse (session ouverte).</p>
      <div className="actions">
        <button onClick={fermer}>Annuler</button>
        <button
          className="principal"
          disabled={montant <= 0 || montant > b.reste_a_payer}
          onClick={() =>
            agir((pin) => post("/paie/payer", { employe_id: b.employe_id, bulletin_id: b.id, montant }, pin), "Paiement enregistré").then(
              (r) => r !== undefined && (fait(), fermer()),
            )
          }
        >
          Payer
        </button>
      </div>
    </Modal>
  );
}
