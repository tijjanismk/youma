import { useState } from "react";
import { get, post } from "../api";
import { Champ, ChampMontant, Choix, Modal, Montant, Onglets, TableauDonnees, Vide } from "../composants/Base";
import { useApp, useDonnees } from "../contexte";
import { dateHeure, fcfa, heure, nombre } from "../format";
import { t } from "../i18n";
import type { Compte, SessionCaisse } from "../types";

type EtatCaisse = { session: SessionCaisse | null; sessions_ouvertes: SessionCaisse[]; comptes: Compte[]; coupures: number[] };
type Mvt = { id: string; compte: string; type: string; montant: number; libelle: string; horodatage: number; utilisateur: string | null };
type LigneBillet = { coupure: number; nombre: number };

export function Billetage({ coupures, lignes, changer }: { coupures: number[]; lignes: LigneBillet[]; changer: (l: LigneBillet[]) => void }) {
  const n = (c: number) => lignes.find((l) => l.coupure === c)?.nombre ?? 0;
  const maj = (c: number, v: number) => changer([...lignes.filter((l) => l.coupure !== c), { coupure: c, nombre: Math.max(0, v) }].filter((l) => l.nombre > 0));
  return (
    <div className="billetage">
      {coupures.map((c) => (
        <label key={c} className="billet">
          <span>{nombre(c)}</span>
          <input type="number" min={0} inputMode="numeric" value={n(c) || ""} placeholder="0" onChange={(e) => maj(c, Number(e.target.value))} aria-label={`Nombre de ${c}`} />
          <span className="aide">{nombre(c * n(c))}</span>
        </label>
      ))}
      <div className="total">
        <span>Total compté</span>
        <strong>{fcfa(lignes.reduce((s, l) => s + l.coupure * l.nombre, 0))}</strong>
      </div>
    </div>
  );
}

export default function Caisse() {
  const { etat, peut } = useApp();
  const { donnees, recharger } = useDonnees(() => get<EtatCaisse>("/caisse"), ["caisse", "paiement"]);
  const [onglet, setOnglet] = useState<"session" | "depenses" | "comptes">("session");
  // Le rapport Z survit au retour à l'écran d'ouverture après la clôture.
  const [z, setZ] = useState<string | null>(null);
  if (!donnees) return <p className="aide">Chargement…</p>;
  if (!etat?.journee) return <Vide>Ouvrez la journée pour utiliser la caisse.</Vide>;
  return (
    <div>
      <h1>Caisse</h1>
      <Onglets
        onglets={[
          { cle: "session", libelle: "Ma session" },
          { cle: "depenses", libelle: "Dépenses du jour" },
          ...(peut("caisse.mouvement") ? [{ cle: "comptes" as const, libelle: "Comptes et transferts" }] : []),
        ]}
        actif={onglet}
        changer={setOnglet}
      />
      {onglet === "session" &&
        (donnees.session ? <SessionOuverte e={donnees} recharger={recharger} afficherZ={setZ} /> : <Ouverture e={donnees} recharger={recharger} />)}
      {z && (
        <Modal titre="Rapport de clôture (Z)" fermer={() => setZ(null)}>
          <pre className="apercu-ticket">{z}</pre>
          <button onClick={() => window.print()}>Imprimer</button>
        </Modal>
      )}
      {onglet === "depenses" && <Depenses />}
      {onglet === "comptes" && <Comptes comptes={donnees.comptes} recharger={recharger} />}
    </div>
  );
}

function Ouverture({ e, recharger }: { e: EtatCaisse; recharger: () => void }) {
  const { agir } = useApp();
  const caisse = e.comptes.find((c) => c.type === "especes" && c.actif);
  const [fond, setFond] = useState(caisse?.solde ?? 0);
  const [billets, setBillets] = useState<LigneBillet[]>([]);
  const [detail, setDetail] = useState(false);
  const [motif, setMotif] = useState("");
  const total = detail ? billets.reduce((s, l) => s + l.coupure * l.nombre, 0) : fond;
  const ecart = caisse ? total - caisse.solde : 0;
  return (
    <div className="carte">
      <h2>Ouvrir ma session</h2>
      {e.sessions_ouvertes.length > 0 && (
        <p className="attention-texte">
          Caisse tenue par {e.sessions_ouvertes.map((s) => s.caissier_nom).join(", ")} : pour une passation, la session précédente doit être clôturée.
        </p>
      )}
      <p>
        Solde attendu dans le tiroir : <Montant valeur={caisse?.solde ?? 0} />
      </p>
      <label className="case">
        <input type="checkbox" checked={detail} onChange={(x) => setDetail(x.target.checked)} />
        <span>Compter billet par billet</span>
      </label>
      {detail ? <Billetage coupures={e.coupures} lignes={billets} changer={setBillets} /> : <ChampMontant libelle="Fond de caisse compté" valeur={fond} changer={setFond} autoFocus />}
      {ecart !== 0 && (
        <>
          <p className="attention-texte">Écart de {fcfa(ecart)} avec le solde attendu.</p>
          <Champ libelle="Motif de l'écart" valeur={motif} changer={setMotif} />
        </>
      )}
      <button
        className="principal grand"
        onClick={() =>
          agir((pin) => post("/caisse/ouvrir", { fond_compte: total, billetage: detail ? billets : [], motif_ecart: motif }, pin), "Session ouverte").then(recharger)
        }
      >
        Ouvrir la caisse
      </button>
    </div>
  );
}

function SessionOuverte({ e, recharger, afficherZ }: { e: EtatCaisse; recharger: () => void; afficherZ: (z: string) => void }) {
  const s = e.session!;
  const { peut } = useApp();
  const { donnees: mvts, recharger: rechargerMvts } = useDonnees(() => get<Mvt[]>(`/caisse/mouvements?session=${s.id}`), ["caisse", "paiement"], [s.id]);
  const [mode, setMode] = useState<"" | "mouvement" | "depense" | "cloture">("");
  const tout = () => {
    recharger();
    rechargerMvts();
  };
  return (
    <div className="grille-2">
      <div className="carte">
        <h2>Session de {s.caissier_nom}</h2>
        <p>Ouverte à {heure(s.ouverte_le)} — fond {fcfa(s.fond_compte)}</p>
        <div className="total">
          <span>Espèces attendues</span>
          <Montant valeur={s.solde_actuel} fort />
        </div>
        <div className="menu-actions">
          <button className="grand" onClick={() => setMode("depense")}>
            ➖ Dépense
          </button>
          {(peut("caisse.mouvement") || peut("caisse.retrait_proprietaire")) && (
            <button className="grand" onClick={() => setMode("mouvement")}>
              ↕️ Entrée / retrait
            </button>
          )}
          <button className="attention grand" onClick={() => setMode("cloture")}>
            🔒 Clôturer ma caisse
          </button>
        </div>
      </div>
      <div className="carte">
        <h2>Mouvements</h2>
        <TableauDonnees
          colonnes={["Heure", "Type", "Libellé", "Montant"]}
          lignes={(mvts ?? []).map((m) => [heure(m.horodatage), t(m.type), m.libelle, <Montant valeur={m.montant} />])}
        />
      </div>
      {mode === "depense" && <NouvelleDepense fermer={() => setMode("")} fait={tout} />}
      {mode === "mouvement" && <MouvementCaisse fermer={() => setMode("")} fait={tout} />}
      {mode === "cloture" && (
        <Cloture
          s={s}
          coupures={e.coupures}
          fermer={() => setMode("")}
          fait={(texte) => {
            setMode("");
            afficherZ(texte);
            tout();
          }}
        />
      )}
    </div>
  );
}

function Cloture({ s, coupures, fermer, fait }: { s: SessionCaisse; coupures: number[]; fermer: () => void; fait: (z: string) => void }) {
  const { agir, etat } = useApp();
  const [billets, setBillets] = useState<LigneBillet[]>([]);
  const [montant, setMontant] = useState(0);
  const [detail, setDetail] = useState(true);
  const [motif, setMotif] = useState("");
  const compte = detail ? billets.reduce((x, l) => x + l.coupure * l.nombre, 0) : montant;
  const ecart = compte - s.solde_actuel;
  const seuil = etat?.parametres.seuil_ecart_caisse ?? 500;
  return (
    <Modal titre="Clôture de caisse" fermer={fermer} large>
      <p className="aide">Comptez l'argent du tiroir. L'écart est calculé et enregistré ; il ne sera plus modifiable.</p>
      <label className="case">
        <input type="checkbox" checked={detail} onChange={(x) => setDetail(x.target.checked)} />
        <span>Billetage (billet par billet)</span>
      </label>
      {detail ? <Billetage coupures={coupures} lignes={billets} changer={setBillets} /> : <ChampMontant libelle="Montant compté" valeur={montant} changer={setMontant} autoFocus />}
      <p>
        Attendu : <strong>{fcfa(s.solde_actuel)}</strong> — Écart : <strong className={ecart < 0 ? "negatif" : ""}>{fcfa(ecart)}</strong>
      </p>
      {Math.abs(ecart) > seuil && <Champ libelle="Motif de l'écart (obligatoire)" valeur={motif} changer={setMotif} obligatoire />}
      <div className="actions">
        <button onClick={fermer}>Annuler</button>
        <button
          className="principal"
          disabled={Math.abs(ecart) > seuil && !motif.trim()}
          onClick={async () => {
            const r = await agir((pin) => post<{ z: string }>(`/caisse/${s.id}/cloturer`, { compte_final: compte, billetage: detail ? billets : [], motif_ecart: motif }, pin), "Caisse clôturée");
            if (r) fait(r.z);
          }}
        >
          Clôturer
        </button>
      </div>
    </Modal>
  );
}

function MouvementCaisse({ fermer, fait }: { fermer: () => void; fait: () => void }) {
  const { agir, peut } = useApp();
  const types = [
    ...(peut("caisse.mouvement") ? [{ valeur: "entree_diverse", libelle: "Entrée diverse" }, { valeur: "apport", libelle: "Apport (monnaie, fond)" }, { valeur: "retrait", libelle: "Retrait (versement banque…)" }] : []),
    { valeur: "retrait_proprietaire", libelle: "Retrait du propriétaire" },
  ];
  const [type, setType] = useState(types[0].valeur);
  const [montant, setMontant] = useState(0);
  const [libelle, setLibelle] = useState("");
  return (
    <Modal titre="Entrée ou retrait d'argent" fermer={fermer}>
      <Choix libelle="Type" valeur={type} changer={setType} options={types} />
      {type === "retrait_proprietaire" && <p className="aide">Un retrait du propriétaire n'est pas une dépense : il n'entre pas dans le bénéfice (RG-CAI-10).</p>}
      <ChampMontant libelle="Montant" valeur={montant} changer={setMontant} autoFocus />
      <Champ libelle="Motif" valeur={libelle} changer={setLibelle} />
      <div className="actions">
        <button onClick={fermer}>Annuler</button>
        <button className="principal" disabled={montant <= 0} onClick={() => agir((pin) => post("/caisse/mouvement", { type, montant, libelle }, pin), "Enregistré").then((r) => r !== undefined && (fait(), fermer()))}>
          Enregistrer
        </button>
      </div>
    </Modal>
  );
}

export function NouvelleDepense({ fermer, fait }: { fermer: () => void; fait: () => void }) {
  const { agir } = useApp();
  const { donnees: cats } = useDonnees(() => get<{ id: string; nom: string }[]>("/depenses/categories"), []);
  const [categorie, setCategorie] = useState("");
  const [montant, setMontant] = useState(0);
  const [beneficiaire, setBeneficiaire] = useState("");
  const [libelle, setLibelle] = useState("");
  return (
    <Modal titre="Nouvelle dépense" fermer={fermer}>
      <div className="suggestions">
        {(cats ?? []).map((c) => (
          <button key={c.id} className={categorie === c.id ? "actif" : ""} onClick={() => setCategorie(c.id)}>
            {c.nom}
          </button>
        ))}
      </div>
      <ChampMontant libelle="Montant" valeur={montant} changer={setMontant} />
      <Champ libelle="Bénéficiaire" valeur={beneficiaire} changer={setBeneficiaire} placeholder="Vendeuse de charbon, EDM…" />
      <Champ libelle="Détail" valeur={libelle} changer={setLibelle} />
      <div className="actions">
        <button onClick={fermer}>Annuler</button>
        <button
          className="principal"
          disabled={!categorie || montant <= 0}
          onClick={() => agir((pin) => post("/depenses", { categorie_id: categorie, montant, beneficiaire, libelle }, pin), "Dépense enregistrée").then((r) => r !== undefined && (fait(), fermer()))}
        >
          Enregistrer
        </button>
      </div>
    </Modal>
  );
}

type Dep = { id: string; categorie: string; montant: number; beneficiaire: string; libelle: string; compte: string; horodatage: number; annulee: boolean; est_annulation: boolean };

function Depenses() {
  const { agir } = useApp();
  const { donnees, recharger } = useDonnees(() => get<Dep[]>("/depenses"), ["caisse"]);
  const [nouvelle, setNouvelle] = useState(false);
  const total = (donnees ?? []).reduce((s, d) => s + (d.est_annulation ? -d.montant : d.montant), 0);
  return (
    <div className="carte">
      <div className="titre-ligne">
        <h2>Dépenses : {fcfa(total)}</h2>
        <button className="principal" onClick={() => setNouvelle(true)}>
          + Dépense
        </button>
      </div>
      <TableauDonnees
        colonnes={["Heure", "Catégorie", "Bénéficiaire / détail", "Compte", "Montant", ""]}
        lignes={(donnees ?? []).map((d) => [
          heure(d.horodatage),
          d.categorie,
          `${d.beneficiaire} ${d.libelle}`.trim(),
          d.compte,
          <Montant valeur={d.est_annulation ? d.montant : -d.montant} />,
          d.annulee ? (
            "Annulée"
          ) : d.est_annulation ? (
            "Contre-passation"
          ) : (
            <button
              className="petit"
              onClick={() => {
                const motif = prompt("Motif de l'annulation de cette dépense ?");
                if (motif) agir((pin) => post(`/depenses/${d.id}/annuler`, { motif }, pin), "Dépense annulée").then(recharger);
              }}
            >
              Annuler
            </button>
          ),
        ])}
      />
      {nouvelle && <NouvelleDepense fermer={() => setNouvelle(false)} fait={recharger} />}
    </div>
  );
}

function Comptes({ comptes, recharger }: { comptes: Compte[]; recharger: () => void }) {
  const { agir } = useApp();
  const actifs = comptes.filter((c) => c.actif);
  const [de, setDe] = useState(actifs[0]?.id ?? "");
  const [vers, setVers] = useState(actifs[1]?.id ?? "");
  const [montant, setMontant] = useState(0);
  const [frais, setFrais] = useState(0);
  const [libelle, setLibelle] = useState("");
  return (
    <div className="grille-2">
      <div className="carte">
        <h2>Soldes</h2>
        <TableauDonnees colonnes={["Compte", "Type", "Solde"]} lignes={actifs.map((c) => [c.nom, t(c.type), <Montant valeur={c.solde} />])} />
        <p className="aide">Soldes calculés à partir des mouvements, jamais saisis à la main.</p>
      </div>
      <div className="carte">
        <h2>Transfert entre comptes</h2>
        <p className="aide">Ex. : dépôt des espèces sur Orange Money, retrait Mobile Money en espèces, versement au coffre.</p>
        <Choix libelle="De" valeur={de} changer={setDe} options={actifs.map((c) => ({ valeur: c.id, libelle: c.nom }))} />
        <Choix libelle="Vers" valeur={vers} changer={setVers} options={actifs.map((c) => ({ valeur: c.id, libelle: c.nom }))} />
        <ChampMontant libelle="Montant" valeur={montant} changer={setMontant} />
        <ChampMontant libelle="Frais de l'opérateur" valeur={frais} changer={setFrais} />
        <Champ libelle="Libellé" valeur={libelle} changer={setLibelle} />
        <button
          className="principal"
          disabled={montant <= 0 || de === vers}
          onClick={() => agir((pin) => post("/caisse/transfert", { de, vers, montant, frais, libelle }, pin), "Transfert enregistré").then(recharger)}
        >
          Transférer
        </button>
        <p className="aide">Dernière mise à jour : {dateHeure(Date.now())}</p>
      </div>
    </div>
  );
}
