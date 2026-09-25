import { useState } from "react";
import { get, post } from "../api";
import { Case, Champ, ChampMontant, Choix, Modal, TableauDonnees, Vide } from "../composants/Base";
import { useApp, useDonnees } from "../contexte";
import { dateHeure, fcfa, nombre } from "../format";
import { t } from "../i18n";
import type { Emballage, EtatEmballage, NiveauStock } from "../types";

type Fournisseur = { id: string; nom: string };

type Donnees = { emballages: EtatEmballage[]; fournisseurs: { fournisseur_id: string; nom: string; montant: number }[] };

/**
 * Consignes (fiche 0015) : bouteilles et casiers consignés.
 * Détenus = pleins + vides au restaurant ; vides = détenus − pleins en stock ; consigne versée = ce que le fournisseur doit rendre.
 */
export default function Consignes() {
  const { peut } = useApp();
  const { donnees, recharger } = useDonnees(() => get<Donnees>("/emballages"), ["stock"]);
  const [form, setForm] = useState<Emballage | null>(null);
  const [mouvement, setMouvement] = useState<EtatEmballage | null>(null);
  const [comptage, setComptage] = useState<EtatEmballage | null>(null);
  const [retour, setRetour] = useState<EtatEmballage | null>(null);
  const [historique, setHistorique] = useState<EtatEmballage | null>(null);
  if (!donnees) return <p className="aide">Chargement…</p>;
  const total = donnees.emballages.reduce((s, e) => s + e.consigne_versee, 0);
  return (
    <div>
      <div className="titre-ligne">
        <p>
          Consigne versée aux fournisseurs : <strong>{fcfa(total)}</strong>{" "}
          <small className="aide">(Σ consignes payées − consignes rendues)</small>
        </p>
        {peut("stock.mouvement") && (
          <button className="principal" onClick={() => setForm({ id: "", nom: "", valeur: 0, actif: true, articles: [] })}>
            + Emballage
          </button>
        )}
      </div>
      {donnees.emballages.length === 0 ? (
        <Vide>Aucun emballage consigné. Ajoutez par exemple « Bouteille bière 65 cl » ou « Casier de 12 ».</Vide>
      ) : (
        <TableauDonnees
          colonnes={["Emballage", "Consigne", "Détenus", "Pleins", "Vides", "Consigne versée", ""]}
          lignes={donnees.emballages.map((e) => [
            e.emballage.nom,
            fcfa(e.emballage.valeur),
            nombre(e.detenus),
            e.pleins === null ? "—" : nombre(e.pleins),
            e.vides === null ? "—" : <span className={e.vides < 0 ? "attention-texte" : ""}>{nombre(e.vides)}</span>,
            fcfa(e.consigne_versee),
            <span className="boutons-ligne">
              {peut("stock.mouvement") && (
                <>
                  <button className="petit" onClick={() => setMouvement(e)} aria-label={`Casse ou sortie : ${e.emballage.nom}`}>
                    Casse / sortie
                  </button>
                  <button className="petit" onClick={() => setForm(e.emballage)}>
                    Modifier
                  </button>
                </>
              )}
              {peut("stock.valider_inventaire") && (
                <button className="petit" onClick={() => setComptage(e)} aria-label={`Compter les vides : ${e.emballage.nom}`}>
                  Compter
                </button>
              )}
              {peut("achat.gerer") && (
                <button className="petit" onClick={() => setRetour(e)} aria-label={`Rendre au fournisseur : ${e.emballage.nom}`}>
                  Rendre
                </button>
              )}
              <button className="petit" onClick={() => setHistorique(e)}>
                Historique
              </button>
            </span>,
          ])}
        />
      )}
      <p className="formule">Vides = détenus − pleins en stock (articles liés). La vente ne change pas les détenus : la bouteille reste au restaurant.</p>
      {donnees.fournisseurs.length > 0 && (
        <>
          <h3>Consigne à récupérer par fournisseur</h3>
          <TableauDonnees colonnes={["Fournisseur", "Consigne versée"]} lignes={donnees.fournisseurs.map((f) => [f.nom, fcfa(f.montant)])} />
        </>
      )}
      {form && <FormEmballage e={form} fermer={() => setForm(null)} fait={recharger} />}
      {mouvement && <Mouvement e={mouvement} fermer={() => setMouvement(null)} fait={recharger} />}
      {comptage && <Comptage e={comptage} fermer={() => setComptage(null)} fait={recharger} />}
      {retour && <Retour e={retour} fermer={() => setRetour(null)} fait={recharger} />}
      {historique && <Historique e={historique} fermer={() => setHistorique(null)} />}
    </div>
  );
}

function FormEmballage({ e: initial, fermer, fait }: { e: Emballage; fermer: () => void; fait: () => void }) {
  const { agir } = useApp();
  const { donnees: articles } = useDonnees(() => get<NiveauStock[]>("/stock"), []);
  const [e, setE] = useState(initial);
  return (
    <Modal titre={e.id ? e.nom : "Nouvel emballage"} fermer={fermer}>
      <Champ libelle="Nom" valeur={e.nom} changer={(v) => setE({ ...e, nom: v })} placeholder="Bouteille bière 65 cl" obligatoire autoFocus />
      <ChampMontant libelle="Consigne d'un emballage" valeur={e.valeur} changer={(v) => setE({ ...e, valeur: v })} />
      <p className="aide">Articles dont une unité pleine est dans cet emballage (pour compter les vides automatiquement) :</p>
      {(articles ?? []).map((a) => (
        <Case
          key={a.article_id}
          libelle={a.nom}
          valeur={e.articles.includes(a.article_id)}
          changer={(v) => setE({ ...e, articles: v ? [...e.articles, a.article_id] : e.articles.filter((x) => x !== a.article_id) })}
        />
      ))}
      <Case libelle="Actif" valeur={e.actif} changer={(v) => setE({ ...e, actif: v })} />
      <div className="actions">
        <button onClick={fermer}>Annuler</button>
        <button
          className="principal"
          disabled={!e.nom.trim()}
          onClick={() => agir((pin) => post("/emballages", e, pin), "Emballage enregistré").then((r) => r !== undefined && (fait(), fermer()))}
        >
          Enregistrer
        </button>
      </div>
    </Modal>
  );
}

function Mouvement({ e, fermer, fait }: { e: EtatEmballage; fermer: () => void; fait: () => void }) {
  const { agir } = useApp();
  const [type, setType] = useState("casse");
  const [quantite, setQuantite] = useState(1);
  const [motif, setMotif] = useState("");
  return (
    <Modal titre={e.emballage.nom} fermer={fermer}>
      <Choix
        libelle="Mouvement"
        valeur={type}
        changer={setType}
        options={[
          { valeur: "casse", libelle: "Casse" },
          { valeur: "perte", libelle: "Perte" },
          { valeur: "sortie_client", libelle: "Emportée par un client" },
          { valeur: "retour_client", libelle: "Rapportée par un client" },
        ]}
      />
      <label className="champ">
        <span>Nombre</span>
        <input type="number" min={1} value={quantite} onChange={(x) => setQuantite(Number(x.target.value))} aria-label="Nombre" />
      </label>
      <Champ libelle="Motif" valeur={motif} changer={setMotif} obligatoire />
      <button
        className="principal"
        disabled={quantite <= 0 || !motif.trim()}
        onClick={() =>
          agir((pin) => post("/emballages/mouvement", { emballage_id: e.emballage.id, type, quantite, motif }, pin), "Enregistré").then(
            (r) => r !== undefined && (fait(), fermer()),
          )
        }
      >
        Enregistrer
      </button>
    </Modal>
  );
}

function Comptage({ e, fermer, fait }: { e: EtatEmballage; fermer: () => void; fait: () => void }) {
  const { agir, notifier } = useApp();
  const [comptes, setComptes] = useState(0);
  const attendu = e.vides ?? e.detenus;
  return (
    <Modal titre={`Compter : ${e.emballage.nom}`} fermer={fermer}>
      <p>
        {e.vides === null ? "Emballages détenus attendus" : "Vides attendus"} : <strong>{nombre(attendu)}</strong>
      </p>
      <label className="champ">
        <span>{e.vides === null ? "Emballages comptés" : "Vides comptés"}</span>
        <input type="number" min={0} value={comptes} onChange={(x) => setComptes(Number(x.target.value))} aria-label="Nombre compté" />
      </label>
      <p className="aide">Écart : {nombre(comptes - attendu)}</p>
      <button
        className="principal"
        disabled={comptes < 0}
        onClick={() =>
          agir((pin) => post<number>(`/emballages/${e.emballage.id}/inventaire`, { comptes }, pin)).then((ecart) => {
            if (ecart === undefined) return;
            notifier(ecart === 0 ? "Comptage conforme" : `Écart enregistré : ${nombre(ecart)}`, "succes");
            fait();
            fermer();
          })
        }
      >
        Valider le comptage
      </button>
    </Modal>
  );
}

function Retour({ e, fermer, fait }: { e: EtatEmballage; fermer: () => void; fait: () => void }) {
  const { agir } = useApp();
  const { donnees: fournisseurs } = useDonnees(() => get<Fournisseur[]>("/fournisseurs"), []);
  const [fournisseur, setFournisseur] = useState("");
  const [quantite, setQuantite] = useState(1);
  const [remboursement, setRemboursement] = useState("dette");
  const f = fournisseur || fournisseurs?.[0]?.id || "";
  return (
    <Modal titre={`Rendre au fournisseur : ${e.emballage.nom}`} fermer={fermer}>
      <Choix libelle="Fournisseur" valeur={f} changer={setFournisseur} options={(fournisseurs ?? []).map((x) => ({ valeur: x.id, libelle: x.nom }))} />
      <label className="champ">
        <span>Nombre rendu</span>
        <input type="number" min={1} value={quantite} onChange={(x) => setQuantite(Number(x.target.value))} aria-label="Nombre rendu" />
      </label>
      <Choix
        libelle="La consigne"
        valeur={remboursement}
        changer={setRemboursement}
        options={[
          { valeur: "dette", libelle: "Déduite de ce qu'on lui doit" },
          { valeur: "especes", libelle: "Remboursée en espèces (entre dans ma caisse)" },
        ]}
      />
      <p>
        Montant : <strong>{fcfa(quantite * e.emballage.valeur)}</strong>
      </p>
      <button
        className="principal"
        disabled={!f || quantite <= 0}
        onClick={() =>
          agir((pin) => post("/emballages/retour", { fournisseur_id: f, emballage_id: e.emballage.id, quantite, remboursement }, pin), "Retour enregistré").then(
            (r) => r !== undefined && (fait(), fermer()),
          )
        }
      >
        Enregistrer le retour
      </button>
    </Modal>
  );
}

function Historique({ e, fermer }: { e: EtatEmballage; fermer: () => void }) {
  const { donnees } = useDonnees(
    () => get<{ type: string; quantite: number; montant: number; fournisseur: string | null; motif: string; horodatage: number }[]>(`/emballages/${e.emballage.id}/historique`),
    [],
  );
  return (
    <Modal titre={`Historique : ${e.emballage.nom}`} fermer={fermer} large>
      <TableauDonnees
        colonnes={["Date", "Mouvement", "Nombre", "Consigne", "Fournisseur", "Motif"]}
        lignes={(donnees ?? []).map((m) => [dateHeure(m.horodatage), t(m.type), nombre(m.quantite), m.montant ? fcfa(m.montant) : "—", m.fournisseur ?? "—", m.motif])}
      />
    </Modal>
  );
}
