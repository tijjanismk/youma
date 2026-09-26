import { useState } from "react";
import { Link } from "react-router";
import { get, post } from "../api";
import { Case, Champ, ChampMontant, Choix, Modal, Montant, Onglets, TableauDonnees } from "../composants/Base";
import { useApp, useDonnees } from "../contexte";
import { aujourdhui, fcfa } from "../format";
import { t } from "../i18n";
import type { Employe } from "../types";

type Refs = { contrats: string[]; remunerations: string[]; fonctions: string[]; presences: string[] };

export const EMPLOYE_VIDE: Employe = {
  id: "",
  nom: "",
  surnom: "",
  telephone: "",
  fonction: "serveur",
  date_embauche: null,
  type_remuneration: "mensuel",
  montant_base: 0,
  type_contrat: "aucun",
  date_fin_contrat: null,
  piece_identite: "",
  contact_urgence: "",
  quartier: "",
  declare_inps: false,
  numero_inps: "",
  affilie_amo: false,
  numero_amo: "",
  avantages_nature: "",
  plafond_avance: null,
  horaires: "",
  statut: "actif",
  date_depart: null,
  notes: "",
  solde: 0,
};

export function libelleMontant(type: string) {
  switch (type) {
    case "mensuel":
      return "Salaire mensuel";
    case "hebdomadaire":
      return "Salaire par semaine";
    case "journalier":
      return "Paye par jour travaillé";
    case "tache":
      return "Prix par tâche / course";
    default:
      return "Montant";
  }
}

/** Formulaire employé : l'essentiel d'abord, tout le reste est facultatif (RG-EMP-01). */
export function FormulaireEmploye({ initial, fermer, fait }: { initial: Employe; fermer: () => void; fait: () => void }) {
  const { agir } = useApp();
  const { donnees: refs } = useDonnees(() => get<Refs>("/employes/references"), []);
  const [e, setE] = useState<Employe>(initial);
  const [plus, setPlus] = useState(!!initial.id);
  const maj = <K extends keyof Employe>(k: K, v: Employe[K]) => setE((x) => ({ ...x, [k]: v }));
  const montantRequis = e.type_remuneration !== "aucun" && e.type_remuneration !== "tache";
  const ok = e.nom.trim() && (!montantRequis || e.montant_base > 0);
  return (
    <Modal titre={e.id ? `Modifier ${initial.nom}` : "Nouvel employé"} fermer={fermer} large>
      <h3>L'essentiel</h3>
      <div className="grille-2">
        <Champ libelle="Nom" valeur={e.nom} changer={(v) => maj("nom", v)} obligatoire autoFocus />
        <Champ libelle="Surnom (comme on l'appelle)" valeur={e.surnom} changer={(v) => maj("surnom", v)} />
        <Choix libelle="Fonction" valeur={e.fonction} changer={(v) => maj("fonction", v)} options={(refs?.fonctions ?? [e.fonction]).map((f) => ({ valeur: f, libelle: f }))} />
        <Choix
          libelle="Rémunération"
          valeur={e.type_remuneration}
          changer={(v) => maj("type_remuneration", v)}
          options={(refs?.remunerations ?? [e.type_remuneration]).map((r) => ({ valeur: r, libelle: r === "aucun" ? "Non rémunéré (aide familiale…)" : t(r) }))}
        />
        {e.type_remuneration !== "aucun" && (
          <ChampMontant libelle={libelleMontant(e.type_remuneration)} valeur={e.montant_base} changer={(v) => maj("montant_base", v)} />
        )}
      </div>
      <button className="lien" onClick={() => setPlus(!plus)}>
        {plus ? "▾" : "▸"} Informations facultatives (contrat, téléphone, INPS, AMO…)
      </button>
      {plus && (
        <>
          <h3>Informations facultatives</h3>
          <p className="aide">Aucune n'est obligatoire : un employé sans contrat écrit ni pièce d'identité peut être suivi normalement.</p>
          <div className="grille-2">
            <Choix
              libelle="Type d'engagement"
              valeur={e.type_contrat}
              changer={(v) => maj("type_contrat", v)}
              options={(refs?.contrats ?? [e.type_contrat]).map((c) => ({ valeur: c, libelle: c === "aucun" ? "Aucun contrat" : t(c) }))}
            />
            {["cdd", "essai", "stage", "apprentissage"].includes(e.type_contrat) && (
              <Champ libelle="Fin prévue" type="date" valeur={e.date_fin_contrat ?? ""} changer={(v) => maj("date_fin_contrat", v || null)} />
            )}
            <Champ libelle="Téléphone" type="tel" valeur={e.telephone} changer={(v) => maj("telephone", v)} />
            <Champ libelle="Quartier" valeur={e.quartier} changer={(v) => maj("quartier", v)} />
            <Champ libelle="Date d'arrivée" type="date" valeur={e.date_embauche ?? ""} changer={(v) => maj("date_embauche", v || null)} />
            <Champ libelle="Pièce d'identité (NINA, carte…)" valeur={e.piece_identite} changer={(v) => maj("piece_identite", v)} />
            <Champ libelle="Personne à prévenir" valeur={e.contact_urgence} changer={(v) => maj("contact_urgence", v)} />
            <Champ libelle="Horaires" valeur={e.horaires} changer={(v) => maj("horaires", v)} placeholder="18 h – 2 h, repos lundi" />
            <Champ libelle="Avantages en nature" valeur={e.avantages_nature} changer={(v) => maj("avantages_nature", v)} placeholder="Nourri le midi, logé…" />
            <ChampMontant libelle="Plafond d'avance propre (vide = règle générale)" valeur={e.plafond_avance ?? 0} changer={(v) => maj("plafond_avance", v || null)} />
          </div>
          <h3>Déclarations sociales (facultatif)</h3>
          <p className="aide">Les cotisations ne sont prélevées que si elles sont activées dans l'administration ET cochées ici.</p>
          <div className="grille-2">
            <Case libelle="Déclaré à l'INPS" valeur={e.declare_inps} changer={(v) => maj("declare_inps", v)} />
            {e.declare_inps && <Champ libelle="N° INPS" valeur={e.numero_inps} changer={(v) => maj("numero_inps", v)} />}
            <Case libelle="Affilié à l'AMO" valeur={e.affilie_amo} changer={(v) => maj("affilie_amo", v)} />
            {e.affilie_amo && <Champ libelle="N° AMO" valeur={e.numero_amo} changer={(v) => maj("numero_amo", v)} />}
          </div>
          {e.id && (
            <>
              <h3>Statut</h3>
              <Choix
                libelle="Statut"
                valeur={e.statut}
                changer={(v) => maj("statut", v)}
                options={["actif", "suspendu", "parti"].map((s) => ({ valeur: s, libelle: t(s) }))}
              />
            </>
          )}
          <label className="champ">
            <span>Notes</span>
            <textarea value={e.notes} onChange={(x) => maj("notes", x.target.value)} />
          </label>
        </>
      )}
      <div className="actions">
        <button onClick={fermer}>Annuler</button>
        <button className="principal" disabled={!ok} onClick={() => agir((pin) => post("/employes", e, pin), "Employé enregistré").then((r) => r !== undefined && (fait(), fermer()))}>
          Enregistrer
        </button>
      </div>
    </Modal>
  );
}

export default function Employes() {
  const { peut } = useApp();
  const [partis, setPartis] = useState(false);
  const { donnees, recharger } = useDonnees(() => get<Employe[]>(`/employes${partis ? "?partis=1" : ""}`), ["employes"], [partis]);
  const [onglet, setOnglet] = useState<"liste" | "pointage">("liste");
  const [nouveau, setNouveau] = useState(false);
  return (
    <div>
      <div className="titre-ligne">
        <h1>Employés</h1>
        {peut("employe.gerer") && (
          <button className="principal" onClick={() => setNouveau(true)}>
            + Employé
          </button>
        )}
      </div>
      <Onglets
        onglets={[
          { cle: "liste", libelle: "Liste" },
          ...(peut("employe.presence") ? [{ cle: "pointage" as const, libelle: "Présences du jour" }] : []),
        ]}
        actif={onglet}
        changer={setOnglet}
      />
      {onglet === "liste" && (
        <>
          <Case libelle="Afficher les anciens employés" valeur={partis} changer={setPartis} />
          <TableauDonnees
            colonnes={["Nom", "Fonction", "Rémunération", "Engagement", "Solde du compte", ""]}
            lignes={(donnees ?? []).map((e) => [
              <Link to={`/employes/${e.id}`}>
                {e.nom} {e.surnom && e.surnom !== e.nom ? `« ${e.surnom} »` : ""}
              </Link>,
              e.fonction,
              e.type_remuneration === "aucun" ? "Non rémunéré" : `${t(e.type_remuneration)} ${e.montant_base ? fcfa(e.montant_base) : ""}`,
              e.type_contrat === "aucun" ? "Sans contrat" : t(e.type_contrat),
              <Montant valeur={e.solde} />,
              e.statut !== "actif" ? t(e.statut) : "",
            ])}
          />
          <p className="aide">Solde positif : le restaurant doit de l'argent à l'employé. Négatif : l'employé doit (avances).</p>
        </>
      )}
      {onglet === "pointage" && <Pointage employes={(donnees ?? []).filter((e) => e.statut === "actif")} />}
      {nouveau && <FormulaireEmploye initial={EMPLOYE_VIDE} fermer={() => setNouveau(false)} fait={recharger} />}
    </div>
  );
}

type Presence = { employe_id: string; date: string; statut: string };

function Pointage({ employes }: { employes: Employe[] }) {
  const { agir, etat } = useApp();
  const date = etat?.journee?.date_exploitation ?? aujourdhui();
  const { donnees, recharger } = useDonnees(() => get<Presence[]>(`/presences?debut=${date}&fin=${date}`), ["employes"], [date]);
  const statut = (id: string) => donnees?.find((p) => p.employe_id === id)?.statut;
  const pointer = (employe_id: string, s: string) => agir((pin) => post("/presences", [{ employe_id, date, statut: s }], pin)).then(recharger);
  const tousPresents = () =>
    agir((pin) => post("/presences", employes.filter((e) => !statut(e.id)).map((e) => ({ employe_id: e.id, date, statut: "present" })), pin), "Présences enregistrées").then(recharger);
  return (
    <div className="carte">
      <div className="titre-ligne">
        <h2>Présences du {date.split("-").reverse().join("/")}</h2>
        <button onClick={tousPresents}>Tous les autres présents</button>
      </div>
      {employes.map((e) => (
        <div key={e.id} className="ligne-pointage">
          <strong>{e.surnom || e.nom}</strong>
          <div className="suggestions">
            {["present", "retard", "absent_justifie", "absent_non_justifie", "repos"].map((s) => (
              <button key={s} className={statut(e.id) === s ? "actif" : ""} onClick={() => pointer(e.id, s)}>
                {t(s)}
              </button>
            ))}
          </div>
        </div>
      ))}
      <p className="aide">Une correction ajoute une nouvelle saisie : l'historique est conservé (RG-EMP-07).</p>
    </div>
  );
}
