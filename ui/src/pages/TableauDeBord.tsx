import { Link } from "react-router-dom";
import { get } from "../api";
import { Montant, Vide } from "../composants/Base";
import { useDonnees } from "../contexte";
import { dateFr, fcfa, nombre } from "../format";
import { t } from "../i18n";
import type { Indicateur, Journee } from "../types";

type Tdb = {
  journee: Journee | null;
  indicateurs: Indicateur[];
  ventes_par_type: [string, number, number][];
  encaissements: [string, number][];
  mobile_money_a_verifier: [number, number];
  stock_critique: [string, number, string][];
  commandes_en_attente: number;
  annulations: [number, number];
  remises: [number, number];
  offerts: [number, number];
  employes_presents: number;
  employes_actifs: number;
  salaires_a_payer: number;
  avances_en_cours: number;
  sessions_ouvertes: number;
  impressions_en_erreur: number;
};

/** Un propriétaire comprend sa journée en 10 secondes : peu de chiffres, formules à portée de main. */
export default function TableauDeBord() {
  const { donnees: d } = useDonnees(() => get<Tdb>("/tableau-de-bord"), ["paiement", "commande", "caisse", "stock", "journee"]);
  if (!d) return <p className="aide">Chargement…</p>;
  if (!d.journee) return <Vide>Aucune journée enregistrée pour l'instant.</Vide>;
  const v = (cle: string) => d.indicateurs.find((i) => i.cle === cle);
  return (
    <div>
      <h1>
        Journée du {dateFr(d.journee.date_exploitation)} {d.journee.statut === "cloturee" && <small>(clôturée)</small>}
      </h1>
      <div className="indicateurs">
        {["ca", "nb_commandes", "depenses", "benefice", "credit"].map((c) => {
          const i = v(c);
          if (!i) return null;
          return (
            <details key={c} className={`indicateur ${c === "benefice" ? "benefice" : ""}`}>
              <summary>
                <span>{i.libelle}</span>
                <strong>{c === "nb_commandes" ? nombre(i.valeur) : fcfa(i.valeur)}</strong>
                <small>Comment est-ce calculé ?</small>
              </summary>
              <p className="formule">{i.formule}</p>
            </details>
          );
        })}
      </div>
      <div className="grille-3">
        <div className="carte">
          <h3>Encaissements</h3>
          {d.encaissements.map(([m, montant]) => (
            <div key={m} className="ligne-valeur">
              <span>{t(m)}</span>
              <Montant valeur={montant} />
            </div>
          ))}
          {d.mobile_money_a_verifier[0] > 0 && (
            <Link to="/mobile-money" className="alerte">
              ⚠️ {d.mobile_money_a_verifier[0]} paiement(s) Mobile Money à vérifier ({fcfa(d.mobile_money_a_verifier[1])})
            </Link>
          )}
        </div>
        <div className="carte">
          <h3>Ventes par type</h3>
          {d.ventes_par_type.map(([type, n, montant]) => (
            <div key={type} className="ligne-valeur">
              <span>
                {t(type)} ({n})
              </span>
              <Montant valeur={montant} />
            </div>
          ))}
          <div className="ligne-valeur">
            <span>Additions en cours</span>
            <strong>{d.commandes_en_attente}</strong>
          </div>
        </div>
        <div className="carte">
          <h3>Contrôle</h3>
          <div className="ligne-valeur">
            <span>Annulations après envoi ({d.annulations[0]})</span>
            <Montant valeur={d.annulations[1]} />
          </div>
          <div className="ligne-valeur">
            <span>Remises ({d.remises[0]})</span>
            <Montant valeur={d.remises[1]} />
          </div>
          <div className="ligne-valeur">
            <span>Offerts ({d.offerts[0]})</span>
            <Montant valeur={d.offerts[1]} />
          </div>
          {d.impressions_en_erreur > 0 && (
            <Link to="/cuisine" className="alerte">
              ⚠️ {d.impressions_en_erreur} ticket(s) non imprimé(s)
            </Link>
          )}
        </div>
        <div className="carte">
          <h3>Stock critique</h3>
          {d.stock_critique.length === 0 ? (
            <p className="aide">Rien à signaler.</p>
          ) : (
            d.stock_critique.map(([nom, q, u]) => (
              <div key={nom} className="ligne-valeur">
                <span>{nom}</span>
                <strong className={q <= 0 ? "negatif" : ""}>
                  {q} {u}
                </strong>
              </div>
            ))
          )}
        </div>
        <div className="carte">
          <h3>Personnel</h3>
          <div className="ligne-valeur">
            <span>Présents aujourd'hui</span>
            <strong>
              {d.employes_presents} / {d.employes_actifs}
            </strong>
          </div>
          <div className="ligne-valeur">
            <span>Salaires restant à payer</span>
            <Montant valeur={d.salaires_a_payer} />
          </div>
          <div className="ligne-valeur">
            <span>Avances en cours</span>
            <Montant valeur={d.avances_en_cours} />
          </div>
        </div>
      </div>
    </div>
  );
}
