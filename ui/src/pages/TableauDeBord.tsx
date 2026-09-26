import type { LucideIcon } from "lucide-react";
import {
  AlertTriangle,
  Banknote,
  HandCoins,
  Info,
  MessageCircle,
  PackageX,
  Receipt,
  ShieldCheck,
  ShoppingBag,
  TrendingDown,
  TrendingUp,
  Users,
  Wallet,
} from "lucide-react";
import type { ReactNode } from "react";
import { Link } from "react-router";
import { get } from "../api";
import { Montant, Vide } from "../composants/Base";
import { useDonnees } from "../contexte";
import { dateFr, fcfa, nombre } from "../format";
import { t } from "../i18n";
import type { Indicateur, Journee } from "../types";
import { lienWhatsApp } from "../whatsapp";

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

const ICONES: Record<string, [LucideIcon, string]> = {
  ca: [Banknote, "accent"],
  nb_commandes: [Receipt, "bleu"],
  depenses: [TrendingDown, "rouge"],
  benefice: [TrendingUp, "vert"],
  credit: [HandCoins, "neutre"],
};

function Bloc({ titre, Icone, children }: { titre: string; Icone: LucideIcon; children: ReactNode }) {
  return (
    <div className="carte bloc">
      <h3>
        <span className="bloc-icone" aria-hidden>
          <Icone size={18} strokeWidth={2} />
        </span>
        {titre}
      </h3>
      {children}
    </div>
  );
}

/** Un propriétaire comprend sa journée en 10 secondes : peu de chiffres, formules à portée de main. */
export default function TableauDeBord() {
  const { donnees: d } = useDonnees(() => get<Tdb>("/tableau-de-bord"), ["paiement", "commande", "caisse", "stock", "journee"]);
  const { donnees: resume } = useDonnees(() => get<{ texte: string | null; telephone: string }>("/journee/resume"), ["paiement", "caisse", "journee"]);
  if (!d) return <p className="aide">Chargement…</p>;
  if (!d.journee) return <Vide>Aucune journée enregistrée pour l'instant.</Vide>;
  const v = (cle: string) => d.indicateurs.find((i) => i.cle === cle);
  return (
    <div>
      <div className="titre-ligne">
        <h1>
          Journée du {dateFr(d.journee.date_exploitation)} {d.journee.statut === "cloturee" && <small>(clôturée)</small>}
        </h1>
        {resume?.texte && (
          <a className="bouton" href={lienWhatsApp(resume.texte, resume.telephone)} target="_blank" rel="noreferrer">
            <MessageCircle size={20} aria-hidden /> Résumé par WhatsApp
          </a>
        )}
      </div>
      <div className="indicateurs">
        {["ca", "nb_commandes", "depenses", "benefice", "credit"].map((c) => {
          const i = v(c);
          if (!i) return null;
          const [Icone, ton] = ICONES[c];
          return (
            <details key={c} className={`indicateur ton-${ton} ${c === "benefice" ? "benefice" : ""}`}>
              <summary>
                <span className="chiffre-icone" aria-hidden>
                  <Icone size={22} strokeWidth={1.9} />
                </span>
                <span>{i.libelle}</span>
                <strong>{c === "nb_commandes" ? nombre(i.valeur) : fcfa(i.valeur)}</strong>
                <small>
                  <Info size={14} aria-hidden /> <span className="texte-long">Comment est-ce calculé ?</span>
                  <span className="texte-court">Formule</span>
                </small>
              </summary>
              <p className="formule">{i.formule}</p>
            </details>
          );
        })}
      </div>
      <div className="grille-3">
        <Bloc titre="Encaissements" Icone={Wallet}>
          {d.encaissements.length === 0 && <p className="aide">Aucun encaissement pour l'instant.</p>}
          {d.encaissements.map(([m, montant]) => (
            <div key={m} className="ligne-valeur">
              <span>{t(m)}</span>
              <Montant valeur={montant} />
            </div>
          ))}
          {d.mobile_money_a_verifier[0] > 0 && (
            <Link to="/mobile-money" className="alerte">
              <AlertTriangle size={16} className="icone-texte" aria-hidden /> {d.mobile_money_a_verifier[0]} paiement(s) Mobile Money à vérifier (
              {fcfa(d.mobile_money_a_verifier[1])})
            </Link>
          )}
        </Bloc>
        <Bloc titre="Ventes par type" Icone={ShoppingBag}>
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
        </Bloc>
        <Bloc titre="Contrôle" Icone={ShieldCheck}>
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
              <AlertTriangle size={16} className="icone-texte" aria-hidden /> {d.impressions_en_erreur} ticket(s) non imprimé(s)
            </Link>
          )}
        </Bloc>
        <Bloc titre="Stock critique" Icone={PackageX}>
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
          {d.stock_critique.length > 0 && (
            <Link to="/stock" className="lien-bloc">
              Voir le stock →
            </Link>
          )}
        </Bloc>
        <Bloc titre="Personnel" Icone={Users}>
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
        </Bloc>
      </div>
    </div>
  );
}
