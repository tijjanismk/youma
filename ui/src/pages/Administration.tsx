import { useEffect, useState } from "react";
import QRCode from "qrcode";
import { appel, get, post } from "../api";
import { Case, Champ, ChampMontant, Choix, Modal, Onglets, TableauDonnees } from "../composants/Base";
import { useApp, useDonnees } from "../contexte";
import { dateHeure, fcfa, nombre } from "../format";
import { t } from "../i18n";
import type { Catalogue, Categorie, NiveauStock, Parametres, Poste, Produit, Zone } from "../types";

type Onglet = "restaurant" | "paiements" | "catalogue" | "salle" | "postes" | "utilisateurs" | "roles" | "appareils" | "sauvegardes" | "licence";

/** RG-AUT-06 : onglets protégés par le mot de passe personnel. */
const PROTEGES: Onglet[] = ["restaurant", "paiements", "utilisateurs", "roles", "appareils", "sauvegardes", "licence"];

export default function Administration() {
  const { peut, session, confirmerMotDePasse } = useApp();
  const [onglet, setOnglet] = useState<Onglet>("catalogue");
  const onglets: { cle: Onglet; libelle: string; p: string }[] = [
    { cle: "catalogue", libelle: "Produits", p: "catalogue.gerer" },
    { cle: "salle", libelle: "Salle et tables", p: "salle.gerer" },
    { cle: "postes", libelle: "Postes et imprimantes", p: "catalogue.gerer" },
    { cle: "restaurant", libelle: "Restaurant et règles", p: "parametre.gerer" },
    { cle: "paiements", libelle: "Moyens de paiement", p: "parametre.gerer" },
    { cle: "utilisateurs", libelle: "Utilisateurs", p: "utilisateur.gerer" },
    { cle: "roles", libelle: "Rôles et droits", p: "utilisateur.gerer" },
    { cle: "appareils", libelle: "Téléphones et tablettes", p: "appareil.gerer" },
    { cle: "sauvegardes", libelle: "Sauvegardes et diagnostic", p: "sauvegarde.gerer" },
    { cle: "licence", libelle: "Licence", p: "licence.gerer" },
  ];
  const visibles = onglets.filter((o) => peut(o.p));
  return (
    <div>
      <h1>Administration</h1>
      <Onglets onglets={visibles} actif={onglet} changer={setOnglet} />
      {PROTEGES.includes(onglet) && !session?.eleve ? (
        <div className="carte etroite">
          <h2>🔒 Mot de passe requis</h2>
          <p>Cette partie de l'administration est protégée par votre mot de passe personnel, en plus du PIN.</p>
          <button className="principal grand" onClick={() => confirmerMotDePasse()}>
            Saisir mon mot de passe
          </button>
        </div>
      ) : (
        <OngletAdmin onglet={onglet} />
      )}
    </div>
  );
}

function OngletAdmin({ onglet }: { onglet: Onglet }) {
  return (
    <>
      {onglet === "catalogue" && <CatalogueAdmin />}
      {onglet === "salle" && <SalleAdmin />}
      {onglet === "postes" && <PostesAdmin />}
      {onglet === "restaurant" && <RestaurantAdmin />}
      {onglet === "utilisateurs" && <UtilisateursAdmin />}
      {onglet === "roles" && <RolesAdmin />}
      {onglet === "appareils" && <AppareilsAdmin />}
      {onglet === "sauvegardes" && <SauvegardesAdmin />}
      {onglet === "licence" && <LicenceAdmin />}
      {onglet === "paiements" && <PaiementsAdmin />}
    </>
  );
}

// ───────────── Moyens de paiement ─────────────

type CompteAdmin = { id: string; nom: string; type: string; operateur: string; employe_id: string | null; actif: boolean; solde: number };

/** Opérateurs Mobile Money : tous proposés, activables un par un ; d'autres s'ajoutent librement. */
function PaiementsAdmin() {
  const { agir } = useApp();
  const { donnees, recharger } = useDonnees(() => get<CompteAdmin[]>("/comptes"), []);
  const [nouveau, setNouveau] = useState<{ nom: string; type: string } | null>(null);
  const comptes = (donnees ?? []).filter((c) => c.type !== "livreur");
  const enregistrer = (c: Partial<CompteAdmin>) => agir((pin) => post("/comptes", c, pin), "Enregistré").then(recharger);
  return (
    <div className="carte">
      <p className="aide">Désactivez les opérateurs que le restaurant n'utilise pas : ils disparaissent de l'écran d'encaissement.</p>
      <TableauDonnees
        colonnes={["Compte", "Type", "Solde", "Actif"]}
        lignes={comptes.map((c) => [
          c.nom,
          t(c.type),
          fcfa(c.solde),
          <input type="checkbox" aria-label={`${c.nom} actif`} checked={c.actif} onChange={(e) => enregistrer({ ...c, actif: e.target.checked })} />,
        ])}
      />
      <button onClick={() => setNouveau({ nom: "", type: "mobile_money" })}>+ Autre opérateur ou compte</button>
      {nouveau && (
        <Modal titre="Nouveau moyen de paiement" fermer={() => setNouveau(null)}>
          <Champ libelle="Nom (ex. Orange Money pro, Ecobank…)" valeur={nouveau.nom} changer={(v) => setNouveau({ ...nouveau, nom: v })} obligatoire autoFocus />
          <Choix
            libelle="Type"
            valeur={nouveau.type}
            changer={(v) => setNouveau({ ...nouveau, type: v })}
            options={[
              { valeur: "mobile_money", libelle: "Mobile Money" },
              { valeur: "banque", libelle: "Banque (virement, carte)" },
              { valeur: "especes", libelle: "Autre caisse espèces" },
              { valeur: "coffre", libelle: "Coffre" },
            ]}
          />
          <button
            className="principal"
            disabled={!nouveau.nom.trim()}
            onClick={() => enregistrer({ nom: nouveau.nom, type: nouveau.type, operateur: nouveau.nom, actif: true }).then(() => setNouveau(null))}
          >
            Ajouter
          </button>
        </Modal>
      )}
    </div>
  );
}

// ───────────── Catalogue ─────────────

const PRODUIT_VIDE: Produit = {
  id: "",
  categorie_id: "",
  nom: "",
  nom_court: "",
  description: "",
  photo: "",
  prix: 0,
  poste_id: null,
  disponible: true,
  actif: true,
  code: "",
  code_barres: "",
  taux_tva_bp: 0,
  suivi_stock: "aucun",
  article_stock_id: null,
  prix_achat_estime: 0,
  ordre: 0,
  prix_zones: [],
  groupes_options: [],
};

function CatalogueAdmin() {
  const { agir } = useApp();
  const { donnees: cat, recharger } = useDonnees(() => get<Catalogue>("/catalogue"), ["catalogue"]);
  const [produit, setProduit] = useState<Produit | null>(null);
  const [categorie, setCategorie] = useState<Categorie | null>(null);
  const [csv, setCsv] = useState<string | null>(null);
  if (!cat) return null;
  return (
    <div>
      <div className="actions">
        <button className="principal" onClick={() => setProduit({ ...PRODUIT_VIDE, categorie_id: cat.categories[0]?.id ?? "" })}>
          + Produit
        </button>
        <button onClick={() => setCategorie({ id: "", nom: "", couleur: "#2e7d32", icone: "", ordre: cat.categories.length, actif: true })}>+ Catégorie</button>
        <button onClick={() => setCsv("categorie;nom;prix;poste\n")}>Importer (Excel → CSV)</button>
      </div>
      {cat.categories.map((c) => (
        <section key={c.id} className="carte">
          <div className="titre-ligne">
            <h3 style={{ color: c.couleur }}>
              {c.icone} {c.nom} {!c.actif && "(masquée)"}
            </h3>
            <button className="petit" onClick={() => setCategorie(c)}>
              Modifier
            </button>
          </div>
          <TableauDonnees
            colonnes={["Produit", "Prix", "Poste", "Stock", "Disponible", ""]}
            lignes={cat.produits
              .filter((p) => p.categorie_id === c.id)
              .map((p) => [
                `${p.nom}${p.actif ? "" : " (inactif)"}`,
                fcfa(p.prix) + (p.prix_zones.length ? " *" : ""),
                cat.postes.find((x) => x.id === p.poste_id)?.nom ?? "—",
                p.suivi_stock === "revendu" ? "Suivi à l'unité" : "—",
                <input
                  type="checkbox"
                  checked={p.disponible}
                  aria-label={`${p.nom} disponible`}
                  onChange={(e) => agir((pin) => post(`/produits/${p.id}/disponibilite`, { disponible: e.target.checked }, pin)).then(recharger)}
                />,
                <button className="petit" onClick={() => setProduit(p)}>
                  Modifier
                </button>,
              ])}
          />
        </section>
      ))}
      <p className="aide">* prix différent selon la zone (VIP, terrasse…).</p>
      {produit && <FormProduit p={produit} cat={cat} fermer={() => setProduit(null)} fait={recharger} />}
      {categorie && (
        <Modal titre="Catégorie" fermer={() => setCategorie(null)}>
          <Champ libelle="Nom" valeur={categorie.nom} changer={(v) => setCategorie({ ...categorie, nom: v })} obligatoire />
          <Champ libelle="Icône (emoji)" valeur={categorie.icone} changer={(v) => setCategorie({ ...categorie, icone: v })} />
          <Champ libelle="Couleur" type="color" valeur={categorie.couleur} changer={(v) => setCategorie({ ...categorie, couleur: v })} />
          <Case libelle="Visible" valeur={categorie.actif} changer={(v) => setCategorie({ ...categorie, actif: v })} />
          <button className="principal" onClick={() => agir((pin) => post("/categories", categorie, pin), "Enregistré").then(() => { setCategorie(null); recharger(); })}>
            Enregistrer
          </button>
        </Modal>
      )}
      {csv !== null && (
        <Modal titre="Importer des produits" fermer={() => setCsv(null)}>
          <p className="aide">Dans Excel : colonnes catégorie ; nom ; prix ; poste (facultatif), puis « Enregistrer sous → CSV ». Collez le contenu ici.</p>
          <textarea className="zone-texte" value={csv} onChange={(e) => setCsv(e.target.value)} rows={10} />
          <input
            type="file"
            accept=".csv,text/csv"
            onChange={async (e) => {
              const f = e.target.files?.[0];
              if (f) setCsv(await f.text());
            }}
          />
          <button className="principal" onClick={() => agir((pin) => appel<number>("/produits/import", { texte: csv, pin }), "Import terminé").then((n) => { if (n !== undefined) { setCsv(null); recharger(); } })}>
            Importer
          </button>
        </Modal>
      )}
    </div>
  );
}

function FormProduit({ p, cat, fermer, fait }: { p: Produit; cat: Catalogue; fermer: () => void; fait: () => void }) {
  const { agir } = useApp();
  const [x, setX] = useState(p);
  const { donnees: salle } = useDonnees(() => get<{ zones: Zone[] }>("/salle"), []);
  const { donnees: articles } = useDonnees(() => get<NiveauStock[]>("/stock").catch(() => [] as NiveauStock[]), []);
  const prixZone = (z: string) => x.prix_zones.find((pz) => pz.zone_id === z)?.prix ?? 0;
  const majZone = (z: string, prix: number) =>
    setX({ ...x, prix_zones: [...x.prix_zones.filter((pz) => pz.zone_id !== z), ...(prix > 0 ? [{ zone_id: z, prix }] : [])] });
  return (
    <Modal titre={p.id ? p.nom : "Nouveau produit"} fermer={fermer} large>
      <div className="grille-2">
        <Champ libelle="Nom" valeur={x.nom} changer={(v) => setX({ ...x, nom: v })} obligatoire autoFocus />
        <Champ libelle="Nom court (ticket)" valeur={x.nom_court} changer={(v) => setX({ ...x, nom_court: v })} />
        <Choix libelle="Catégorie" valeur={x.categorie_id} changer={(v) => setX({ ...x, categorie_id: v })} options={cat.categories.map((c) => ({ valeur: c.id, libelle: c.nom }))} />
        <ChampMontant libelle="Prix de vente" valeur={x.prix} changer={(v) => setX({ ...x, prix: v })} />
        <Choix
          libelle="Poste de préparation"
          valeur={x.poste_id ?? ""}
          changer={(v) => setX({ ...x, poste_id: v || null })}
          options={[{ valeur: "", libelle: "Aucun (servi directement)" }, ...cat.postes.map((po) => ({ valeur: po.id, libelle: po.nom }))]}
        />
        <Choix
          libelle="Suivi du stock"
          valeur={x.suivi_stock}
          changer={(v) => setX({ ...x, suivi_stock: v })}
          options={[
            { valeur: "aucun", libelle: "Aucun" },
            { valeur: "revendu", libelle: "Article revendu (1 vente = 1 unité)" },
          ]}
        />
        {x.suivi_stock === "revendu" && (
          <Choix
            libelle="Article de stock"
            valeur={x.article_stock_id ?? ""}
            changer={(v) => setX({ ...x, article_stock_id: v || null })}
            options={[{ valeur: "", libelle: "— choisir —" }, ...(articles ?? []).map((a) => ({ valeur: a.article_id, libelle: a.nom }))]}
          />
        )}
        <ChampMontant libelle="Coût d'achat estimé (pour le bénéfice)" valeur={x.prix_achat_estime} changer={(v) => setX({ ...x, prix_achat_estime: v })} />
        <Champ libelle="Code" valeur={x.code} changer={(v) => setX({ ...x, code: v })} />
        <Champ libelle="Photo (adresse d'image)" valeur={x.photo} changer={(v) => setX({ ...x, photo: v })} />
        <Case libelle="Actif" valeur={x.actif} changer={(v) => setX({ ...x, actif: v })} />
      </div>
      {salle && salle.zones.length > 0 && (
        <>
          <h3>Prix par zone (vide = prix normal)</h3>
          <div className="grille-3">
            {salle.zones.map((z) => (
              <ChampMontant key={z.id} libelle={z.nom} valeur={prixZone(z.id)} changer={(v) => majZone(z.id, v)} />
            ))}
          </div>
        </>
      )}
      <h3>Options et suppléments</h3>
      {x.groupes_options.map((g, gi) => (
        <div key={gi} className="carte">
          <div className="grille-3">
            <Champ libelle="Groupe" valeur={g.nom} changer={(v) => setX({ ...x, groupes_options: x.groupes_options.map((a, j) => (j === gi ? { ...a, nom: v } : a)) })} />
            <label className="champ">
              <span>Minimum</span>
              <input type="number" min={0} value={g.min_choix} onChange={(e) => setX({ ...x, groupes_options: x.groupes_options.map((a, j) => (j === gi ? { ...a, min_choix: Number(e.target.value) } : a)) })} />
            </label>
            <label className="champ">
              <span>Maximum</span>
              <input type="number" min={1} value={g.max_choix} onChange={(e) => setX({ ...x, groupes_options: x.groupes_options.map((a, j) => (j === gi ? { ...a, max_choix: Number(e.target.value) } : a)) })} />
            </label>
          </div>
          {g.options.map((o, oi) => (
            <div key={oi} className="grille-3">
              <Champ
                libelle="Option"
                valeur={o.nom}
                changer={(v) => setX({ ...x, groupes_options: x.groupes_options.map((a, j) => (j === gi ? { ...a, options: a.options.map((b, k) => (k === oi ? { ...b, nom: v } : b)) } : a)) })}
              />
              <ChampMontant
                libelle="Supplément"
                valeur={o.supplement}
                changer={(v) => setX({ ...x, groupes_options: x.groupes_options.map((a, j) => (j === gi ? { ...a, options: a.options.map((b, k) => (k === oi ? { ...b, supplement: v } : b)) } : a)) })}
              />
            </div>
          ))}
          <button className="lien" onClick={() => setX({ ...x, groupes_options: x.groupes_options.map((a, j) => (j === gi ? { ...a, options: [...a.options, { id: "", nom: "", supplement: 0 }] } : a)) })}>
            + option
          </button>
        </div>
      ))}
      <button className="lien" onClick={() => setX({ ...x, groupes_options: [...x.groupes_options, { id: "", nom: "Cuisson", min_choix: 0, max_choix: 1, options: [] }] })}>
        + Groupe d'options
      </button>
      <p className="aide">Un changement de prix n'affecte pas les commandes déjà saisies et reste dans l'historique.</p>
      <div className="actions">
        <button onClick={fermer}>Annuler</button>
        <button
          className="principal"
          disabled={!x.nom.trim() || !x.categorie_id}
          onClick={() =>
            agir(
              (pin) =>
                post(
                  "/produits",
                  { ...x, groupes_options: x.groupes_options.filter((g) => g.nom.trim()).map((g) => ({ ...g, options: g.options.filter((o) => o.nom.trim()) })) },
                  pin,
                ),
              "Produit enregistré",
            ).then((r) => r !== undefined && (fait(), fermer()))
          }
        >
          Enregistrer
        </button>
      </div>
    </Modal>
  );
}

// ───────────── Salle ─────────────

function SalleAdmin() {
  const { agir } = useApp();
  const { donnees, recharger } = useDonnees(() => get<{ zones: Zone[]; tables: { id: string; zone_id: string; nom: string; capacite: number }[] }>("/salle"), ["table"]);
  const [zone, setZone] = useState<Zone | null>(null);
  const [serie, setSerie] = useState<{ zone_id: string; prefixe: string; debut: number; nombre: number } | null>(null);
  if (!donnees) return null;
  return (
    <div>
      <div className="actions">
        <button className="principal" onClick={() => setZone({ id: "", nom: "", ordre: donnees.zones.length, actif: true })}>
          + Zone
        </button>
        {donnees.zones.length > 0 && <button onClick={() => setSerie({ zone_id: donnees.zones[0].id, prefixe: "", debut: 1, nombre: 5 })}>+ Tables</button>}
      </div>
      {donnees.zones.map((z) => (
        <div key={z.id} className="carte">
          <div className="titre-ligne">
            <h3>
              {z.nom} {!z.actif && "(fermée)"}
            </h3>
            <button className="petit" onClick={() => setZone(z)}>
              Modifier
            </button>
          </div>
          <p>{donnees.tables.filter((t) => t.zone_id === z.id).map((t) => t.nom).join(", ") || "Aucune table"}</p>
        </div>
      ))}
      <p className="aide">Sans tables (fast-food, comptoir) : utilisez simplement « Vente comptoir » depuis l'accueil.</p>
      {zone && (
        <Modal titre="Zone" fermer={() => setZone(null)}>
          <Champ libelle="Nom (Salle, Terrasse, VIP climatisé…)" valeur={zone.nom} changer={(v) => setZone({ ...zone, nom: v })} obligatoire />
          <Case libelle="Ouverte" valeur={zone.actif} changer={(v) => setZone({ ...zone, actif: v })} />
          <button className="principal" onClick={() => agir((pin) => post("/zones", zone, pin), "Enregistré").then(() => { setZone(null); recharger(); })}>
            Enregistrer
          </button>
        </Modal>
      )}
      {serie && (
        <Modal titre="Créer des tables" fermer={() => setSerie(null)}>
          <Choix libelle="Zone" valeur={serie.zone_id} changer={(v) => setSerie({ ...serie, zone_id: v })} options={donnees.zones.map((z) => ({ valeur: z.id, libelle: z.nom }))} />
          <Champ libelle="Préfixe (ex. T pour T1, T2…)" valeur={serie.prefixe} changer={(v) => setSerie({ ...serie, prefixe: v })} />
          <label className="champ">
            <span>Premier numéro</span>
            <input type="number" value={serie.debut} onChange={(e) => setSerie({ ...serie, debut: Number(e.target.value) })} />
          </label>
          <label className="champ">
            <span>Nombre de tables</span>
            <input type="number" value={serie.nombre} onChange={(e) => setSerie({ ...serie, nombre: Number(e.target.value) })} />
          </label>
          <button className="principal" onClick={() => agir((pin) => post("/tables/serie", serie, pin), "Tables créées").then(() => { setSerie(null); recharger(); })}>
            Créer
          </button>
        </Modal>
      )}
    </div>
  );
}

// ───────────── Postes et imprimantes ─────────────

function PostesAdmin() {
  const { agir } = useApp();
  const { donnees: cat, recharger } = useDonnees(() => get<Catalogue>("/catalogue"), []);
  const [poste, setPoste] = useState<Poste | null>(null);
  return (
    <div>
      <p className="aide">
        Imprimante réseau : <code>tcp:192.168.1.50:9100</code> — imprimante USB installée sous Windows : <code>windows:NOM</code> — vide : écran seulement.
      </p>
      <button className="principal" onClick={() => setPoste({ id: "", nom: "", imprimante: "", ecran: true, actif: true })}>
        + Poste
      </button>
      <TableauDonnees
        colonnes={["Poste", "Imprimante", "Écran", ""]}
        lignes={(cat?.postes ?? []).map((p) => [
          p.nom,
          p.imprimante || "—",
          p.ecran ? "Oui" : "Non",
          <button className="petit" onClick={() => setPoste(p)}>
            Modifier
          </button>,
        ])}
      />
      {poste && (
        <Modal titre="Poste de préparation" fermer={() => setPoste(null)}>
          <Champ libelle="Nom (Cuisine, Grill, Bar…)" valeur={poste.nom} changer={(v) => setPoste({ ...poste, nom: v })} obligatoire />
          <Champ libelle="Imprimante" valeur={poste.imprimante} changer={(v) => setPoste({ ...poste, imprimante: v })} placeholder="tcp:192.168.1.50:9100" />
          <Case libelle="Écran cuisine" valeur={poste.ecran} changer={(v) => setPoste({ ...poste, ecran: v })} />
          <Case libelle="Actif" valeur={poste.actif} changer={(v) => setPoste({ ...poste, actif: v })} />
          <button className="principal" onClick={() => agir((pin) => post("/postes", poste, pin), "Enregistré").then(() => { setPoste(null); recharger(); })}>
            Enregistrer
          </button>
        </Modal>
      )}
    </div>
  );
}

// ───────────── Restaurant et règles ─────────────

type Restaurant = { nom: string; adresse: string; telephone: string; ville: string; nif: string; pied_ticket: string };

function RestaurantAdmin() {
  const { agir, rechargerEtat } = useApp();
  const { donnees: r } = useDonnees(() => get<Restaurant>("/restaurant"), []);
  const { donnees: p } = useDonnees(() => get<Parametres>("/parametres"), []);
  const [resto, setResto] = useState<Restaurant | null>(null);
  const [params, setParams] = useState<Parametres | null>(null);
  useEffect(() => {
    if (r) setResto(r);
  }, [r]);
  useEffect(() => {
    if (p) setParams(p);
  }, [p]);
  if (!resto || !params) return null;
  const pc = (bp: number) => (bp / 100).toFixed(2).replace(".", ",");
  const lirePc = (s: string) => Math.round(parseFloat(s.replace(",", ".")) * 100) || 0;
  return (
    <div className="grille-2">
      <div className="carte">
        <h2>Restaurant</h2>
        <Champ libelle="Nom" valeur={resto.nom} changer={(v) => setResto({ ...resto, nom: v })} />
        <Champ libelle="Adresse" valeur={resto.adresse} changer={(v) => setResto({ ...resto, adresse: v })} />
        <Champ libelle="Téléphone" valeur={resto.telephone} changer={(v) => setResto({ ...resto, telephone: v })} />
        <Champ libelle="NIF (facultatif)" valeur={resto.nif} changer={(v) => setResto({ ...resto, nif: v })} />
        <Champ libelle="Pied de ticket" valeur={resto.pied_ticket} changer={(v) => setResto({ ...resto, pied_ticket: v })} />
        <button className="principal" onClick={() => agir((pin) => appel("/restaurant", { methode: "PUT", corps: resto, pin }), "Enregistré").then(rechargerEtat)}>
          Enregistrer
        </button>
      </div>
      <div className="carte">
        <h2>Règles de fonctionnement</h2>
        <label className="champ">
          <span>Heure de bascule de la journée (ventes après minuit rattachées à la veille)</span>
          <input type="number" min={0} max={12} value={params.heure_bascule} onChange={(e) => setParams({ ...params, heure_bascule: Number(e.target.value) })} />
        </label>
        <ChampMontant libelle="Arrondi des parts et remises (FCFA)" valeur={params.arrondi} changer={(v) => setParams({ ...params, arrondi: v || 1 })} />
        <ChampMontant libelle="Écart de caisse toléré sans motif" valeur={params.seuil_ecart_caisse} changer={(v) => setParams({ ...params, seuil_ecart_caisse: v })} />
        <Case libelle="Référence Mobile Money obligatoire" valeur={params.reference_mm_obligatoire} changer={(v) => setParams({ ...params, reference_mm_obligatoire: v })} />
        <Case libelle="Comptoir : payer avant l'envoi en cuisine" valeur={params.paiement_avant_comptoir} changer={(v) => setParams({ ...params, paiement_avant_comptoir: v })} />
        <Case libelle="À emporter : payer avant l'envoi" valeur={params.paiement_avant_emporter} changer={(v) => setParams({ ...params, paiement_avant_emporter: v })} />
        <label className="champ">
          <span>Verrouillage après inactivité (minutes)</span>
          <input type="number" min={1} value={params.verrouillage_minutes} onChange={(e) => setParams({ ...params, verrouillage_minutes: Number(e.target.value) })} />
        </label>
        <Champ libelle="Imprimante de caisse (tickets clients)" valeur={params.imprimante_caisse} changer={(v) => setParams({ ...params, imprimante_caisse: v })} placeholder="tcp:192.168.1.51:9100" />
        <Case libelle="Ouvrir le tiroir-caisse à l'impression du ticket" valeur={params.ouvrir_tiroir} changer={(v) => setParams({ ...params, ouvrir_tiroir: v })} />
        <h3>Paie</h3>
        <Case libelle="Déduire les absences non justifiées du salaire mensuel" valeur={params.paie.deduire_absences} changer={(v) => setParams({ ...params, paie: { ...params.paie, deduire_absences: v } })} />
        <label className="champ">
          <span>Jours ouvrables par mois</span>
          <input type="number" value={params.paie.jours_ouvrables_mois} onChange={(e) => setParams({ ...params, paie: { ...params.paie, jours_ouvrables_mois: Number(e.target.value) } })} />
        </label>
        <label className="champ">
          <span>Plafond d'avance (% du salaire, 0 = aucun)</span>
          <input type="number" value={params.paie.plafond_avance_pct} onChange={(e) => setParams({ ...params, paie: { ...params.paie, plafond_avance_pct: Number(e.target.value) } })} />
        </label>
        <h3>Cotisations sociales (facultatives)</h3>
        <p className="aide">
          Désactivées par défaut : la plupart des employés ne sont pas déclarés. Si vous les activez, elles ne s'appliquent qu'aux employés cochés « déclaré INPS » /
          « affilié AMO ». Saisissez vous-même les taux (fournis par votre comptable ou la caisse).
        </p>
        <Case libelle="Prélever la cotisation INPS" valeur={params.cotisations.inps_active} changer={(v) => setParams({ ...params, cotisations: { ...params.cotisations, inps_active: v } })} />
        {params.cotisations.inps_active && (
          <div className="grille-2">
            <Champ libelle="INPS part salarié (%)" valeur={pc(params.cotisations.inps_salarie_bp)} changer={(v) => setParams({ ...params, cotisations: { ...params.cotisations, inps_salarie_bp: lirePc(v) } })} />
            <Champ libelle="INPS part employeur (%)" valeur={pc(params.cotisations.inps_employeur_bp)} changer={(v) => setParams({ ...params, cotisations: { ...params.cotisations, inps_employeur_bp: lirePc(v) } })} />
          </div>
        )}
        <Case libelle="Prélever la cotisation AMO" valeur={params.cotisations.amo_active} changer={(v) => setParams({ ...params, cotisations: { ...params.cotisations, amo_active: v } })} />
        {params.cotisations.amo_active && (
          <div className="grille-2">
            <Champ libelle="AMO part salarié (%)" valeur={pc(params.cotisations.amo_salarie_bp)} changer={(v) => setParams({ ...params, cotisations: { ...params.cotisations, amo_salarie_bp: lirePc(v) } })} />
            <Champ libelle="AMO part employeur (%)" valeur={pc(params.cotisations.amo_employeur_bp)} changer={(v) => setParams({ ...params, cotisations: { ...params.cotisations, amo_employeur_bp: lirePc(v) } })} />
          </div>
        )}
        <h3>Livraison</h3>
        {params.quartiers.map((q, i) => (
          <div key={i} className="grille-2">
            <Champ libelle="Quartier" valeur={q.nom} changer={(v) => setParams({ ...params, quartiers: params.quartiers.map((x, j) => (j === i ? { ...x, nom: v } : x)) })} />
            <ChampMontant libelle="Frais" valeur={q.frais} changer={(v) => setParams({ ...params, quartiers: params.quartiers.map((x, j) => (j === i ? { ...x, frais: v } : x)) })} />
          </div>
        ))}
        <button className="lien" onClick={() => setParams({ ...params, quartiers: [...params.quartiers, { nom: "", frais: 500 }] })}>
          + Quartier
        </button>
        <h3>Sauvegardes</h3>
        <Champ
          libelle="Second emplacement (clé USB, autre disque) — ex. E:\Youma"
          valeur={params.dossier_sauvegarde_externe}
          changer={(v) => setParams({ ...params, dossier_sauvegarde_externe: v })}
        />
        <button className="principal" onClick={() => agir((pin) => appel("/parametres", { methode: "PUT", corps: params, pin }), "Paramètres enregistrés").then(rechargerEtat)}>
          Enregistrer les règles
        </button>
      </div>
    </div>
  );
}

// ───────────── Utilisateurs et rôles ─────────────

type Utilisateur = { id: string; nom: string; role_code: string; role_nom: string; actif: boolean; employe_id: string | null };
type Role = { id: string; code: string; nom: string; plafond_remise_pct: number; permissions: string[] };

function UtilisateursAdmin() {
  const { agir } = useApp();
  const { donnees, recharger } = useDonnees(() => get<Utilisateur[]>("/utilisateurs"), []);
  const { donnees: roles } = useDonnees(() => get<{ roles: Role[] }>("/roles"), []);
  const [nouveau, setNouveau] = useState<{ nom: string; role_code: string; pin: string; mot_de_passe: string } | null>(null);
  return (
    <div>
      <p className="aide">Un employé n'est pas forcément un utilisateur : le plongeur n'a pas besoin de compte.</p>
      <button className="principal" onClick={() => setNouveau({ nom: "", role_code: "serveur", pin: "", mot_de_passe: "" })}>
        + Utilisateur
      </button>
      <TableauDonnees
        colonnes={["Nom", "Rôle", "Actif", ""]}
        lignes={(donnees ?? []).map((u) => [
          u.nom,
          u.role_nom,
          u.actif ? "Oui" : "Non",
          <span className="boutons-ligne">
            <button
              className="petit"
              onClick={() => {
                const pin = prompt(`Nouveau PIN pour ${u.nom} (4 à 6 chiffres)`);
                if (pin) agir((p) => appel(`/utilisateurs/${u.id}`, { methode: "PUT", corps: { id: u.id, pin }, pin: p }), "PIN changé");
              }}
            >
              Changer le PIN
            </button>
            <button
              className="petit"
              onClick={() => {
                const nouveau = prompt(`Nouveau mot de passe d'administration pour ${u.nom} (6 caractères au moins)`);
                if (nouveau) agir((p) => post(`/utilisateurs/${u.id}/mot-de-passe`, { nouveau }, p), "Mot de passe défini");
              }}
            >
              Mot de passe
            </button>
            <button className="petit" onClick={() => agir((p) => appel(`/utilisateurs/${u.id}`, { methode: "PUT", corps: { id: u.id, actif: !u.actif }, pin: p }), "Enregistré").then(recharger)}>
              {u.actif ? "Désactiver" : "Réactiver"}
            </button>
          </span>,
        ])}
      />
      {nouveau && (
        <Modal titre="Nouvel utilisateur" fermer={() => setNouveau(null)}>
          <Champ libelle="Nom" valeur={nouveau.nom} changer={(v) => setNouveau({ ...nouveau, nom: v })} obligatoire autoFocus />
          <Choix libelle="Rôle" valeur={nouveau.role_code} changer={(v) => setNouveau({ ...nouveau, role_code: v })} options={(roles?.roles ?? []).map((r) => ({ valeur: r.code, libelle: r.nom }))} />
          <Champ libelle="Code PIN (4 à 6 chiffres)" type="password" valeur={nouveau.pin} changer={(v) => setNouveau({ ...nouveau, pin: v })} obligatoire />
          <Champ
            libelle="Mot de passe d'administration (gérant, propriétaire ; facultatif)"
            type="password"
            valeur={nouveau.mot_de_passe}
            changer={(v) => setNouveau({ ...nouveau, mot_de_passe: v })}
          />
          <button className="principal" onClick={() => agir((p) => post("/utilisateurs", { ...nouveau, mot_de_passe: nouveau.mot_de_passe || null }, p), "Utilisateur créé").then((r) => {
                if (r !== undefined) {
                  setNouveau(null);
                  recharger();
                }
              })}>
            Créer
          </button>
        </Modal>
      )}
    </div>
  );
}

function RolesAdmin() {
  const { agir } = useApp();
  const { donnees, recharger } = useDonnees(() => get<{ roles: Role[]; permissions: string[] }>("/roles"), []);
  const [edition, setEdition] = useState<Role | null>(null);
  if (!donnees) return null;
  return (
    <div>
      <p className="aide">Matrice rôles × permissions. Le rôle propriétaire garde toujours tous les droits.</p>
      <div className="tableau-conteneur">
        <table className="tableau matrice">
          <thead>
            <tr>
              <th>Permission</th>
              {donnees.roles.map((r) => (
                <th key={r.id}>
                  {r.nom}
                  {r.code !== "proprietaire" && (
                    <button className="petit" onClick={() => setEdition(r)}>
                      ✎
                    </button>
                  )}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {donnees.permissions.map((p) => (
              <tr key={p}>
                <td>{p}</td>
                {donnees.roles.map((r) => (
                  <td key={r.id} className="centre">
                    {r.permissions.includes(p) ? "✓" : ""}
                  </td>
                ))}
              </tr>
            ))}
            <tr>
              <td>Plafond de remise</td>
              {donnees.roles.map((r) => (
                <td key={r.id} className="centre">
                  {r.plafond_remise_pct} %
                </td>
              ))}
            </tr>
          </tbody>
        </table>
      </div>
      {edition && (
        <Modal titre={`Droits : ${edition.nom}`} fermer={() => setEdition(null)} large>
          <label className="champ">
            <span>Plafond de remise (%)</span>
            <input type="number" min={0} max={100} value={edition.plafond_remise_pct} onChange={(e) => setEdition({ ...edition, plafond_remise_pct: Number(e.target.value) })} />
          </label>
          <div className="grille-3">
            {donnees.permissions.map((p) => (
              <Case
                key={p}
                libelle={p}
                valeur={edition.permissions.includes(p)}
                changer={(v) => setEdition({ ...edition, permissions: v ? [...edition.permissions, p] : edition.permissions.filter((x) => x !== p) })}
              />
            ))}
          </div>
          <button
            className="principal"
            onClick={() => agir((pin) => appel("/roles", { methode: "PUT", corps: { code: edition.code, plafond_remise_pct: edition.plafond_remise_pct, permissions: edition.permissions }, pin }), "Droits enregistrés").then(() => { setEdition(null); recharger(); })}
          >
            Enregistrer
          </button>
        </Modal>
      )}
    </div>
  );
}

// ───────────── Appareils (mode réseau) ─────────────

type Appareil = { id: string; nom: string; type: string; actif: boolean; cree_le: number; derniere_vue: number | null };

function AppareilsAdmin() {
  const { agir } = useApp();
  const { donnees: reseau } = useDonnees(() => get<{ actif: boolean; port: number; adresses: string[] }>("/reseau"), []);
  const { donnees: appareils, recharger } = useDonnees(() => get<Appareil[]>("/appareils"), []);
  const [code, setCode] = useState<{ code: string; expire_le: number } | null>(null);
  const [qr, setQr] = useState("");
  const adresse = reseau?.adresses[0] ?? `${location.origin}/`;
  useEffect(() => {
    if (code) QRCode.toDataURL(`${adresse}?appairage=${code.code}`, { width: 260, margin: 1 }).then(setQr).catch(() => setQr(""));
  }, [code, adresse]);
  return (
    <div className="grille-2">
      <div className="carte">
        <h2>Connecter un téléphone</h2>
        {!reseau?.actif && <p className="attention-texte">Le poste central est en mode mono-poste. Démarrez-le en mode réseau pour connecter des téléphones.</p>}
        <p>1. Le téléphone se connecte au Wi-Fi du restaurant.</p>
        <p>2. Il scanne ce QR code (ou ouvre {adresse} et saisit le code).</p>
        <button className="principal grand" onClick={() => agir((pin) => post<{ code: string; expire_le: number }>("/appareils/code", {}, pin)).then((c) => c && setCode(c))}>
          Générer un code (10 min)
        </button>
        {code && (
          <div className="qr">
            {qr && <img src={qr} alt="QR de connexion" />}
            <p className="code-appairage">{code.code}</p>
          </div>
        )}
      </div>
      <div className="carte">
        <h2>Appareils autorisés</h2>
        <TableauDonnees
          colonnes={["Nom", "Ajouté", "Dernière activité", ""]}
          lignes={(appareils ?? []).map((a) => [
            a.nom,
            dateHeure(a.cree_le),
            a.derniere_vue ? dateHeure(a.derniere_vue) : "—",
            a.actif ? (
              <button className="petit attention" onClick={() => agir((pin) => post(`/appareils/${a.id}/revoquer`, {}, pin), "Appareil révoqué").then(recharger)}>
                Révoquer
              </button>
            ) : (
              "Révoqué"
            ),
          ])}
        />
      </div>
    </div>
  );
}

// ───────────── Sauvegardes, diagnostic ─────────────

type Diagnostic = {
  version: string;
  schema: number;
  base: string;
  taille_base: number;
  mode: string;
  postes_connectes: number;
  horloge: { coherente: boolean };
  sauvegardes: { derniere: number | null; derniere_externe: number | null; alerte_externe: boolean; espace_libre: number | null; alerte_disque: boolean; dernier_controle: { ok: boolean; messages: string[]; horodatage: number } | null };
};
type Sauvegarde = { chemin: string; taille: number; horodatage: number; motif: string };

function SauvegardesAdmin() {
  const { agir } = useApp();
  const { donnees: d, recharger } = useDonnees(() => get<Diagnostic>("/diagnostic"), []);
  const { donnees: liste, recharger: rechargerListe } = useDonnees(() => get<Sauvegarde[]>("/sauvegardes"), []);
  const [usb, setUsb] = useState("E:\\");
  const [restauration, setRestauration] = useState("");
  const tout = () => {
    recharger();
    rechargerListe();
  };
  const mo = (o: number) => `${nombre(Math.round(o / 1_048_576))} Mo`;
  return (
    <div className="grille-2">
      <div className="carte">
        <h2>État</h2>
        {d && (
          <>
            <div className="ligne-valeur">
              <span>Version</span>
              <strong>
                {d.version} (schéma {d.schema})
              </strong>
            </div>
            <div className="ligne-valeur">
              <span>Mode</span>
              <strong>{d.mode}</strong>
            </div>
            <div className="ligne-valeur">
              <span>Base</span>
              <strong>{mo(d.taille_base)}</strong>
            </div>
            <div className="ligne-valeur">
              <span>Postes connectés</span>
              <strong>{d.postes_connectes}</strong>
            </div>
            <div className="ligne-valeur">
              <span>Dernière sauvegarde</span>
              <strong>{d.sauvegardes.derniere ? dateHeure(d.sauvegardes.derniere) : "jamais"}</strong>
            </div>
            <div className="ligne-valeur">
              <span>Dernière copie externe</span>
              <strong className={d.sauvegardes.alerte_externe ? "negatif" : ""}>{d.sauvegardes.derniere_externe ? dateHeure(d.sauvegardes.derniere_externe) : "jamais"}</strong>
            </div>
            {d.sauvegardes.alerte_externe && <p className="attention-texte">⚠️ Aucune sauvegarde sur clé USB récemment : faites un export.</p>}
            {d.sauvegardes.espace_libre !== null && (
              <div className="ligne-valeur">
                <span>Espace disque libre</span>
                <strong className={d.sauvegardes.alerte_disque ? "negatif" : ""}>{mo(d.sauvegardes.espace_libre)}</strong>
              </div>
            )}
            {d.sauvegardes.dernier_controle && (
              <p className={d.sauvegardes.dernier_controle.ok ? "aide" : "attention-texte"}>
                Contrôle d'intégrité du {dateHeure(d.sauvegardes.dernier_controle.horodatage)} : {d.sauvegardes.dernier_controle.messages.join(", ")}
              </p>
            )}
          </>
        )}
        <div className="menu-actions">
          <button onClick={() => agir(() => post("/sauvegardes"), "Sauvegarde faite").then(tout)}>Sauvegarder maintenant</button>
          <button onClick={() => agir(() => post("/integrite"), "Contrôle terminé").then(tout)}>Contrôle complet de la base</button>
        </div>
        <h3>Export sur clé USB</h3>
        <Champ libelle="Dossier de la clé (ex. E:\)" valeur={usb} changer={setUsb} />
        <button className="principal" onClick={() => agir((pin) => post("/sauvegardes/exporter", { chemin: usb }, pin), "Copie sur la clé terminée").then(tout)}>
          Exporter
        </button>
      </div>
      <div className="carte">
        <h2>Sauvegardes locales</h2>
        <TableauDonnees
          colonnes={["Date", "Motif", "Taille", ""]}
          lignes={(liste ?? []).slice(0, 30).map((s) => [
            dateHeure(s.horodatage),
            s.motif,
            mo(s.taille),
            <button className="petit" onClick={() => setRestauration(s.chemin)}>
              Restaurer
            </button>,
          ])}
        />
        <h3>Restaurer depuis un fichier</h3>
        <Champ libelle="Chemin du fichier de sauvegarde" valeur={restauration} changer={setRestauration} />
        <button
          className="attention"
          disabled={!restauration}
          onClick={() => {
            if (confirm("Restaurer remplace toutes les données actuelles (une sauvegarde de l'état actuel est faite avant). Continuer ?"))
              agir((pin) => post("/sauvegardes/restaurer", { chemin: restauration }, pin), "Restauration terminée").then(() => location.reload());
          }}
        >
          Restaurer
        </button>
      </div>
    </div>
  );
}

// ───────────── Licence ─────────────

type EtatLicence = { code_machine: string; valide: boolean; maintenance_active: boolean; modules: string[]; message: string; licence: { restaurant: string; maintenance_jusqua: string | null; numero: string } | null };

function LicenceAdmin() {
  const { agir } = useApp();
  const { donnees, recharger } = useDonnees(() => get<EtatLicence>("/licence"), []);
  const [texte, setTexte] = useState("");
  if (!donnees) return null;
  return (
    <div className="carte">
      <h2>Licence</h2>
      <p>
        Code de ce PC : <code className="code-appairage">{donnees.code_machine}</code>
      </p>
      <p className="aide">Envoyez ce code au fournisseur (téléphone, WhatsApp). Il vous renvoie un code de licence à coller ci-dessous.</p>
      <p className={donnees.valide ? "" : "attention-texte"}>{donnees.message}</p>
      {donnees.licence && (
        <p>
          Licence {donnees.licence.numero} — {donnees.licence.restaurant} — maintenance jusqu'au {donnees.licence.maintenance_jusqua ?? "illimitée"} — modules :{" "}
          {donnees.modules.map((m) => t(m)).join(", ") || "aucun"}
        </p>
      )}
      <label className="champ">
        <span>Code de licence</span>
        <textarea className="zone-texte" rows={4} value={texte} onChange={(e) => setTexte(e.target.value)} />
      </label>
      <button className="principal" disabled={!texte.trim()} onClick={() => agir((pin) => appel("/licence", { texte, pin }), "Licence installée").then(recharger)}>
        Installer la licence
      </button>
      <p className="aide">Une licence expirée ne bloque jamais les ventes : seuls les mises à jour et les modules cloud s'arrêtent.</p>
    </div>
  );
}

