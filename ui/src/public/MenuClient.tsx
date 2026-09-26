import { MapPin } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { ErreurApi, get, post } from "../api";
import { Case, Champ, Choix, Modal, Onglets } from "../composants/Base";
import { fcfa, versMicro } from "../format";
import type { GroupeOptions, MenuPublic, ReponseEntrante } from "../types";
import { cleOptions, lirePanierClient, PanierClient, retenirSuivi, totalPanierClient } from "./panierClient";

type Produit = MenuPublic["produits"][number];

/**
 * Menu du client, sans connexion : QR collé sur la table (`/menu?table=CODE`) ou commande en ligne (`/menu`).
 * Les prix affichés sont indicatifs : le poste central les recalcule toujours.
 */
export default function MenuClient() {
  const table = new URLSearchParams(location.search).get("table");
  const clePanier = `youma.panier-client.${table ?? "en-ligne"}`;
  const [menu, setMenu] = useState<MenuPublic | null>(null);
  const [erreur, setErreur] = useState("");
  const [categorie, setCategorie] = useState("");
  const [panier, setPanier] = useState<PanierClient>(() => lirePanierClient(clePanier));
  const [choixOptions, setChoixOptions] = useState<Produit | null>(null);
  const [commander, setCommander] = useState(false);
  const [refus, setRefus] = useState("");

  useEffect(() => {
    get<MenuPublic>(`/public/menu${table ? `?table=${encodeURIComponent(table)}` : ""}`)
      .then((m) => {
        setMenu(m);
        setCategorie(m.categories.find((c) => m.produits.some((p) => p.categorie_id === c.id))?.id ?? "");
      })
      .catch((e) => setErreur(messageClient(e, !!table)));
  }, [table]);

  useEffect(() => {
    try {
      localStorage.setItem(clePanier, JSON.stringify(panier));
    } catch {
      /* stockage indisponible */
    }
  }, [panier, clePanier]);

  const total = useMemo(() => (menu ? totalPanierClient(panier, menu.produits) : 0), [panier, menu]);
  const nombreArticles = panier.reduce((s, l) => s + l.quantite, 0);

  if (erreur) return <Page titre="Menu">{<p className="erreur-texte">{erreur}</p>}</Page>;
  if (!menu) return <Page titre="Menu">{<p className="aide">Chargement du menu…</p>}</Page>;

  const ajouter = (p: Produit, options: string[] = []) => {
    const cle = cleOptions(p.id, options);
    setPanier((pa) => {
      const l = pa.find((x) => cleOptions(x.produit_id, x.options) === cle);
      return l ? pa.map((x) => (x === l ? { ...x, quantite: x.quantite + 1 } : x)) : [...pa, { produit_id: p.id, quantite: 1, options, commentaire: "" }];
    });
  };
  const retirer = (produitId: string) =>
    setPanier((pa) => {
      const i = pa.map((l) => l.produit_id).lastIndexOf(produitId);
      if (i < 0) return pa;
      const l = pa[i];
      return l.quantite > 1 ? pa.map((x, j) => (j === i ? { ...x, quantite: x.quantite - 1 } : x)) : pa.filter((_, j) => j !== i);
    });
  const quantite = (id: string) => panier.filter((l) => l.produit_id === id).reduce((s, l) => s + l.quantite, 0);
  const cats = menu.categories.filter((c) => menu.produits.some((p) => p.categorie_id === c.id));

  return (
    <Page titre={menu.restaurant} sousTitre={menu.table ? `Table ${menu.table}` : "Commande en ligne"}>
      {!menu.ouvert && <p className="bandeau erreur">Le restaurant est fermé pour le moment : vous pouvez consulter le menu.</p>}
      {refus && (
        <p className="bandeau erreur" role="alert">
          {refus}
        </p>
      )}
      <Onglets onglets={cats.map((c) => ({ cle: c.id, libelle: `${c.icone} ${c.nom}` }))} actif={categorie} changer={setCategorie} />
      <div className="produits-client">
        {menu.produits
          .filter((p) => p.categorie_id === categorie)
          .map((p) => (
            <div key={p.id} className="produit-client">
              <div>
                <strong>{p.nom}</strong>
                {p.description && <p>{p.description}</p>}
                <div>{fcfa(p.prix)}</div>
              </div>
              <div className="quantite-client">
                {quantite(p.id) > 0 && (
                  <>
                    <button onClick={() => retirer(p.id)} aria-label={`Retirer ${p.nom}`}>
                      −
                    </button>
                    <strong>{quantite(p.id)}</strong>
                  </>
                )}
                <button className="principal" onClick={() => (p.groupes_options.length ? setChoixOptions(p) : ajouter(p))} aria-label={`Ajouter ${p.nom}`}>
                  +
                </button>
              </div>
            </div>
          ))}
      </div>
      {nombreArticles > 0 && menu.ouvert && (
        <div className="barre-panier">
          <button className="principal grand" onClick={() => setCommander(true)}>
            Commander ({nombreArticles}) — {fcfa(total)}
          </button>
        </div>
      )}
      {choixOptions && (
        <ChoixOptionsClient
          p={choixOptions}
          fermer={() => setChoixOptions(null)}
          valider={(o) => {
            ajouter(choixOptions, o);
            setChoixOptions(null);
          }}
        />
      )}
      {commander && (
        <Validation
          menu={menu}
          table={table}
          panier={panier}
          total={total}
          fermer={() => setCommander(false)}
          fait={(r) => {
            if (r.statut === "en_attente" && r.code_suivi) {
              setPanier([]);
              retenirSuivi(r.code_suivi);
              location.assign(`/suivi/${r.code_suivi}`);
            } else {
              setCommander(false);
              setRefus(r.message);
            }
          }}
        />
      )}
    </Page>
  );
}

/** Message pour le client : jamais le vocabulaire du personnel (« permission », codes d'erreur). */
export function messageClient(e: unknown, qr: boolean): string {
  if (e instanceof ErreurApi) {
    if (e.horsLigne) return "Le restaurant est injoignable pour le moment. Réessayez dans un instant.";
    if (e.statut === 403) return qr ? "La commande depuis la table n'est pas active : appelez le serveur." : "Ce restaurant ne prend pas de commandes en ligne pour le moment.";
    if (e.statut === 503) return "Le restaurant n'est pas encore connecté. Réessayez plus tard.";
    if (e.regle === "RG-CAN-02") return "Ce QR code n'est plus valable : demandez au serveur.";
  }
  return e instanceof Error ? e.message : String(e);
}

export function Page({ titre, sousTitre, children }: { titre: string; sousTitre?: string; children: React.ReactNode }) {
  return (
    <div className="public">
      <header className="public-entete">
        <h1>{titre}</h1>
        {sousTitre && <p className="aide">{sousTitre}</p>}
      </header>
      {children}
    </div>
  );
}

function ChoixOptionsClient({ p, fermer, valider }: { p: Produit; fermer: () => void; valider: (options: string[]) => void }) {
  const [choisies, setChoisies] = useState<string[]>([]);
  const basculer = (g: GroupeOptions, id: string) =>
    setChoisies((c) => {
      if (c.includes(id)) return c.filter((x) => x !== id);
      const duGroupe = c.filter((x) => g.options.some((o) => o.id === x));
      const reste = g.max_choix === 1 ? c.filter((x) => !duGroupe.includes(x)) : c;
      return g.max_choix > 1 && duGroupe.length >= g.max_choix ? c : [...reste, id];
    });
  const valide = p.groupes_options.every((g) => choisies.filter((x) => g.options.some((o) => o.id === x)).length >= g.min_choix);
  return (
    <Modal titre={p.nom} fermer={fermer}>
      {p.groupes_options.map((g) => (
        <fieldset key={g.id} className="champ">
          <legend>
            {g.nom}
            {g.min_choix > 0 && " *"}
          </legend>
          <div className="suggestions">
            {g.options.map((o) => (
              <button key={o.id} className={choisies.includes(o.id) ? "actif" : ""} aria-pressed={choisies.includes(o.id)} onClick={() => basculer(g, o.id)}>
                {o.nom}
                {o.supplement > 0 && ` (+${fcfa(o.supplement)})`}
              </button>
            ))}
          </div>
        </fieldset>
      ))}
      <button className="principal grand" disabled={!valide} onClick={() => valider(choisies)}>
        Ajouter
      </button>
    </Modal>
  );
}

function Validation({
  menu,
  table,
  panier,
  total,
  fermer,
  fait,
}: {
  menu: MenuPublic;
  table: string | null;
  panier: PanierClient;
  total: number;
  fermer: () => void;
  fait: (r: ReponseEntrante) => void;
}) {
  const enLigne = !table;
  const [type, setType] = useState<"livraison" | "emporter">("livraison");
  const [nom, setNom] = useState("");
  const [telephone, setTelephone] = useState("");
  const [quartier, setQuartier] = useState(menu.quartiers[0]?.nom ?? "");
  const [repere, setRepere] = useState("");
  const [position, setPosition] = useState<[number, number] | null>(null);
  const [partager, setPartager] = useState(false);
  const [mode, setMode] = useState(menu.paiement_a_la_livraison ? "a_la_livraison" : "avance");
  const [operateur, setOperateur] = useState(menu.operateurs[0] ?? "");
  const [reference, setReference] = useState("");
  const [note, setNote] = useState("");
  const [envoi, setEnvoi] = useState(false);
  // RG-CAN-04 : code SMS (serveur relais et fournisseur SMS configurés).
  const sms = enLigne && menu.verification_numero === "sms";
  const [codeSms, setCodeSms] = useState("");
  const [smsEnvoye, setSmsEnvoye] = useState(false);
  const [codeSimule, setCodeSimule] = useState("");
  const demanderCode = async () => {
    setErreur("");
    try {
      const r = await post<{ envoye: boolean; simulation?: boolean; code?: string }>("/public/verification", { telephone });
      setSmsEnvoye(true);
      // Relais sans contrat Orange Mali : envoi simulé, le code s'affiche ici.
      setCodeSimule(r.simulation && r.code ? r.code : "");
    } catch (e) {
      setErreur(e instanceof Error ? e.message : String(e));
    }
  };
  const [erreur, setErreur] = useState("");
  const frais = enLigne && type === "livraison" ? (menu.quartiers.find((q) => q.nom === quartier)?.frais ?? 0) : 0;

  useEffect(() => {
    if (!partager) return setPosition(null);
    navigator.geolocation?.getCurrentPosition(
      (p) => setPosition([versMicro(p.coords.latitude), versMicro(p.coords.longitude)]),
      () => {
        setPartager(false);
        setErreur("Position indisponible : indiquez un point de repère précis.");
      },
    );
  }, [partager]);

  const valide =
    !enLigne ||
    (telephone.replace(/\D/g, "").length >= 8 && (!sms || codeSms.trim().length === 4) && (type === "emporter" || (quartier.trim() && repere.trim())) && (mode !== "avance" || (operateur && reference.trim())));

  const envoyer = async () => {
    setEnvoi(true);
    setErreur("");
    try {
      const corps = enLigne
        ? {
            canal: "en_ligne",
            type,
            client_nom: nom,
            telephone,
            livraison:
              type === "livraison" ? { quartier, repere, telephone, lat: position?.[0] ?? null, lon: position?.[1] ?? null } : null,
            paiement_mode: mode,
            paiement_operateur: mode === "avance" ? operateur : null,
            paiement_reference: mode === "avance" ? reference : null,
            code_verification: sms ? codeSms.trim() : undefined,
            lignes: panier,
            note,
          }
        : { canal: "qr_table", code_table: table, lignes: panier, note };
      fait(await post<ReponseEntrante>("/public/commandes", corps));
    } catch (e) {
      setErreur(e instanceof Error ? e.message : String(e));
    } finally {
      setEnvoi(false);
    }
  };

  return (
    <Modal titre="Ma commande" fermer={fermer}>
      <ul className="lignes-entrante">
        {panier.map((l, i) => (
          <li key={i}>
            {l.quantite} × {menu.produits.find((p) => p.id === l.produit_id)?.nom}
          </li>
        ))}
      </ul>
      {enLigne && (
        <>
          <Onglets
            onglets={[
              { cle: "livraison", libelle: "Livraison" },
              { cle: "emporter", libelle: "Je viens chercher" },
            ]}
            actif={type}
            changer={setType}
          />
          <Champ libelle="Votre nom" valeur={nom} changer={setNom} />
          <Champ libelle="Votre téléphone" valeur={telephone} changer={setTelephone} type="tel" obligatoire />
          {sms && (
            <div className="grille-2">
              <button type="button" disabled={telephone.replace(/\D/g, "").length < 8} onClick={demanderCode}>
                {smsEnvoye ? "Renvoyer le code" : "Recevoir un code par SMS"}
              </button>
              <Champ libelle="Code reçu par SMS" valeur={codeSms} changer={setCodeSms} />
              {codeSimule && <p className="aide">Mode test (SMS simulé) : votre code est {codeSimule}</p>}
            </div>
          )}
          {type === "livraison" && (
            <>
              {menu.quartiers.length > 0 ? (
                <Choix
                  libelle="Quartier"
                  valeur={quartier}
                  changer={setQuartier}
                  options={menu.quartiers.map((q) => ({ valeur: q.nom, libelle: `${q.nom} (livraison ${fcfa(q.frais)})` }))}
                />
              ) : (
                <Champ libelle="Quartier" valeur={quartier} changer={setQuartier} obligatoire />
              )}
              <Champ libelle="Point de repère" valeur={repere} changer={setRepere} placeholder="Derrière la mosquée, portail bleu…" obligatoire />
              <Case libelle="Partager ma position pour le livreur" valeur={partager} changer={setPartager} />
              {position && (
                <p className="aide">
                  <MapPin size={16} className="icone-texte" aria-hidden /> Position enregistrée
                </p>
              )}
            </>
          )}
          <Choix
            libelle="Paiement"
            valeur={mode}
            changer={setMode}
            options={[
              ...(menu.paiement_a_la_livraison ? [{ valeur: "a_la_livraison", libelle: type === "livraison" ? "À la livraison" : "Sur place, en récupérant" }] : []),
              ...(menu.paiement_avance && menu.operateurs.length ? [{ valeur: "avance", libelle: "Mobile Money maintenant" }] : []),
            ]}
          />
          {mode === "avance" && (
            <>
              <p className="aide">
                Envoyez <strong>{fcfa(total + frais)}</strong> au {menu.telephone}, puis indiquez la référence reçue par SMS.
              </p>
              <Choix libelle="Opérateur" valeur={operateur} changer={setOperateur} options={menu.operateurs.map((o) => ({ valeur: o, libelle: o }))} />
              <Champ libelle="Référence de la transaction" valeur={reference} changer={setReference} obligatoire />
            </>
          )}
        </>
      )}
      <Champ libelle="Remarque (facultatif)" valeur={note} changer={setNote} placeholder="Sans piment…" />
      <p>
        Total : <strong>{fcfa(total + frais)}</strong>
        {frais > 0 && <small> (dont livraison {fcfa(frais)})</small>}
      </p>
      {erreur && <p className="erreur-texte">{erreur}</p>}
      <button className="principal grand" disabled={!valide || envoi} onClick={envoyer}>
        {envoi ? "Envoi…" : "Envoyer la commande"}
      </button>
      <p className="aide">Le restaurant confirme votre commande avant de la préparer.</p>
    </Modal>
  );
}
