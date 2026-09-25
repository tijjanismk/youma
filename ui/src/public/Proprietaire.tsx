import { useEffect, useState } from "react";
import { Champ, TableauDonnees } from "../composants/Base";
import { CarteInstallation } from "../composants/Installation";
import { dateFr, dateHeure, fcfa, nombre } from "../format";
import { t } from "../i18n";
import type { ResumeJournee } from "../types";
import { Page } from "./MenuClient";

type Tableau = {
  restaurants: { id: string; nom: string; dernier_contact: number | null; derniere_sauvegarde: number | null; resumes: ResumeJournee[] }[];
  totaux: { date: string; chiffre_affaires: number; commandes: number }[];
};

const CLE = "youma.proprietaire";

function lireJeton(): string | null {
  try {
    return sessionStorage.getItem(CLE);
  } catch {
    return null;
  }
}

async function appelProprio<T>(chemin: string, jeton: string | null, corps?: unknown): Promise<T> {
  const r = await fetch(`/api/proprietaire${chemin}`, {
    method: corps ? "POST" : "GET",
    headers: { ...(corps ? { "Content-Type": "application/json" } : {}), ...(jeton ? { Authorization: `Bearer ${jeton}` } : {}) },
    body: corps ? JSON.stringify(corps) : undefined,
  });
  const d = await r.json().catch(() => ({}));
  if (!r.ok) throw new Error(d.message ?? `Erreur ${r.status}`);
  return d as T;
}

/**
 * Espace propriétaire (fiche 0018, RG-CLO-05) : servi par le serveur Internet, il montre les résumés envoyés par
 * les postes de tous les restaurants du propriétaire. Consultation seule : rien ne se modifie à distance.
 */
export default function Proprietaire() {
  const [jeton, setJeton] = useState<string | null>(lireJeton);
  const [donnees, setDonnees] = useState<Tableau | null>(null);
  const [erreur, setErreur] = useState("");
  useEffect(() => {
    if (!jeton) return;
    appelProprio<Tableau>("/tableau", jeton)
      .then(setDonnees)
      .catch((e) => {
        setErreur(e.message);
        setJeton(null);
      });
  }, [jeton]);
  if (!jeton) return <ConnexionProprio connecte={setJeton} erreurInitiale={erreur} />;
  if (!donnees) return <Page titre="Mes restaurants">{<p className="aide">Chargement…</p>}</Page>;
  const aujourdhui = donnees.totaux[0];
  return (
    <Page titre="Mes restaurants" sousTitre={aujourdhui ? `Dernière journée : ${dateFr(aujourdhui.date)}` : undefined}>
      <CarteInstallation reseauLocal={false} />
      {donnees.restaurants.length > 1 && (
        <div className="carte" aria-label="Tous les restaurants">
          <h2>Tous les restaurants</h2>
          <TableauDonnees
            colonnes={["Journée", "Chiffre d'affaires", "Commandes"]}
            lignes={donnees.totaux.slice(0, 14).map((x) => [dateFr(x.date), fcfa(x.chiffre_affaires), nombre(x.commandes)])}
          />
        </div>
      )}
      {donnees.restaurants.map((r) => {
        const d = r.resumes[0];
        return (
          <div key={r.id} className="carte" aria-label={r.nom}>
            <h2>{r.nom}</h2>
            <p className="aide">
              {r.dernier_contact ? `Dernières nouvelles : ${dateHeure(r.dernier_contact)}` : "Jamais connecté"}
              {" · "}
              {r.derniere_sauvegarde ? `sauvegarde du ${dateHeure(r.derniere_sauvegarde)}` : "aucune sauvegarde dans le cloud"}
            </p>
            {d && (
              <>
                <p>
                  <strong>{dateFr(d.date)}</strong> {d.cloturee ? "(clôturée)" : "(en cours)"} : <strong>{fcfa(d.chiffre_affaires)}</strong>, {nombre(d.commandes)}{" "}
                  commande(s)
                </p>
                <p>{d.encaissements.map(([m, x]) => `${t(m)} ${fcfa(x)}`).join(" · ")}</p>
                {d.mobile_money_a_verifier[0] > 0 && (
                  <p className="attention-texte">
                    Mobile Money à vérifier : {d.mobile_money_a_verifier[0]} ({fcfa(d.mobile_money_a_verifier[1])})
                  </p>
                )}
                {d.annulations[0] > 0 && (
                  <p className="attention-texte">
                    Annulations : {d.annulations[0]} ({fcfa(d.annulations[1])})
                  </p>
                )}
                {d.ecarts_caisse !== 0 && <p className="attention-texte">Écart de caisse : {fcfa(d.ecarts_caisse)}</p>}
              </>
            )}
            <TableauDonnees
              colonnes={["Journée", "Chiffre d'affaires", "Commandes", "Dépenses"]}
              lignes={r.resumes.map((x) => [dateFr(x.date), fcfa(x.chiffre_affaires), nombre(x.commandes), fcfa(x.depenses)])}
            />
          </div>
        );
      })}
      <button
        onClick={() => {
          try {
            sessionStorage.removeItem(CLE);
          } catch {
            /* stockage indisponible */
          }
          setJeton(null);
          setDonnees(null);
        }}
      >
        Se déconnecter
      </button>
    </Page>
  );
}

function ConnexionProprio({ connecte, erreurInitiale }: { connecte: (j: string) => void; erreurInitiale: string }) {
  const [telephone, setTelephone] = useState("");
  const [mdp, setMdp] = useState("");
  const [erreur, setErreur] = useState(erreurInitiale);
  const valider = async () => {
    setErreur("");
    try {
      const r = await appelProprio<{ jeton: string }>("/connexion", null, { telephone, mot_de_passe: mdp });
      try {
        sessionStorage.setItem(CLE, r.jeton);
      } catch {
        /* stockage indisponible : la session reste en mémoire */
      }
      connecte(r.jeton);
    } catch (e) {
      setErreur(e instanceof Error ? e.message : String(e));
    }
  };
  return (
    <Page titre="Espace propriétaire" sousTitre="Consultez vos restaurants à distance">
      <div className="carte etroite">
        <Champ libelle="Votre téléphone" valeur={telephone} changer={setTelephone} type="tel" autoFocus />
        <Champ libelle="Mot de passe distant" valeur={mdp} changer={setMdp} type="password" />
        {erreur && <p className="erreur-texte">{erreur}</p>}
        <button className="principal grand" disabled={telephone.replace(/\D/g, "").length < 8 || !mdp} onClick={valider}>
          Se connecter
        </button>
      </div>
    </Page>
  );
}
