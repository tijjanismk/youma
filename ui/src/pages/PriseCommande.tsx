import { Bike, Receipt } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { useNavigate, useParams } from "react-router";
import { ErreurApi, get, post } from "../api";
import { Champ, ChampMontant, DemandeMotif, Modal, Montant } from "../composants/Base";
import { VisuelPlat } from "../composants/Plat";
import { useApp, useDonnees } from "../contexte";
import { fcfa, nombre } from "../format";
import { t } from "../i18n";
import { ajouter, ArticlePanier, changerQuantite, chargerPanier, optionsValides, prixZone, sauverPanier, totalPanier, versLignes } from "../panier";
import type { Catalogue, Client, Commande, Employe, Ligne, PrixDuMoment, Produit, TablePlan } from "../types";

export default function PriseCommande() {
  const { id = "" } = useParams();
  const nav = useNavigate();
  const { agir, peut, notifier } = useApp();
  const { donnees: cmd, recharger } = useDonnees(() => get<Commande>(`/commandes/${id}`), ["commande", "envoi", "envoi_pret", "paiement"], [id]);
  const { donnees: cat } = useDonnees(() => get<Catalogue>("/catalogue"), ["catalogue"]);
  // Happy hour (fiche 0017) : prix en cours pour la zone de la table, rafraîchis chaque minute.
  const { donnees: enCours, recharger: rechargerPromos } = useDonnees(
    () => get<Record<string, PrixDuMoment>>(`/promotions/prix${cmd?.zone_id ? `?zone=${cmd.zone_id}` : ""}`).catch(() => ({}) as Record<string, PrixDuMoment>),
    ["catalogue"],
    [cmd?.zone_id],
  );
  useEffect(() => {
    const t = setInterval(rechargerPromos, 60_000);
    return () => clearInterval(t);
  }, [rechargerPromos]);
  const promos: Record<string, number> = Object.fromEntries(Object.entries(enCours ?? {}).map(([k, v]) => [k, v.prix]));
  const [categorie, setCategorie] = useState<string | null>(null);
  const [recherche, setRecherche] = useState("");
  const [panier, setPanier] = useState<ArticlePanier[]>(() => chargerPanier(id));
  const [produitOptions, setProduitOptions] = useState<Produit | null>(null);
  const [articleEdite, setArticleEdite] = useState<ArticlePanier | null>(null);
  const [ligneAction, setLigneAction] = useState<Ligne | null>(null);
  const [plus, setPlus] = useState(false);
  const [envoiEnCours, setEnvoiEnCours] = useState(false);
  // Téléphone : le ticket est un tiroir qui monte du bas.
  const [ticketOuvert, setTicketOuvert] = useState(false);

  useEffect(() => sauverPanier(id, panier), [id, panier]);
  useEffect(() => {
    if (cat && !categorie) setCategorie(cat.categories.find((c) => c.actif)?.id ?? null);
  }, [cat, categorie]);

  const produits = useMemo(() => {
    if (!cat) return [];
    const r = recherche.trim().toLowerCase();
    return cat.produits.filter((p) => p.actif && (r ? p.nom.toLowerCase().includes(r) || p.code === r : p.categorie_id === categorie));
  }, [cat, categorie, recherche]);

  if (!cmd || !cat) return <p className="aide">Chargement…</p>;
  const modifiable = cmd.statut === "ouverte";
  const categorieDe = (p: Produit) => cat.categories.find((c) => c.id === p.categorie_id);
  const categorieCourante = cat.categories.find((c) => c.id === categorie);

  const toucherProduit = (p: Produit) => {
    if (!p.disponible) return notifier(`${p.nom} : rupture aujourd'hui`, "erreur");
    // Options obligatoires seulement : sinon ajout direct, options modifiables en touchant la ligne.
    if (p.groupes_options.some((g) => g.min_choix > 0)) return setProduitOptions(p);
    setPanier((x) => ajouter(x, p, cmd.zone_id, [], "", promos));
  };

  /** Pousse le panier sur le serveur ; envoie en cuisine sauf « payer d'abord ». */
  const envoyer = async (envoyerCuisine = true) => {
    if (envoiEnCours) return false;
    setEnvoiEnCours(true);
    try {
      const r = await agir(async (pin) => {
        if (panier.length) await post(`/commandes/${id}/lignes`, versLignes(panier), pin);
        setPanier([]);
        if (envoyerCuisine) await post(`/commandes/${id}/envoyer`, {}, pin);
        return true;
      });
      await recharger();
      return !!r;
    } finally {
      setEnvoiEnCours(false);
    }
  };

  const encaisser = async () => {
    if (panier.length && !(await envoyer(cmd.ordre_paiement === "apres"))) return;
    nav(`/encaisser/${id}`);
  };

  const titre = cmd.table_nom ? `Table ${cmd.table_nom}` : `${t(cmd.type)} n°${cmd.numero}`;
  const totalAffiche = cmd.totaux.total + totalPanier(panier);
  const nbArticles = cmd.lignes.reduce((n, l) => n + (l.quantite - l.quantite_annulee), 0) + panier.reduce((n, a) => n + a.quantite, 0);

  return (
    <div className="prise-commande">
      <section className="catalogue">
        <input
          className="recherche"
          placeholder="Rechercher un produit…"
          value={recherche}
          onChange={(e) => setRecherche(e.target.value)}
          aria-label="Rechercher un produit"
        />
        <div className="categories" role="tablist">
          {cat.categories
            .filter((c) => c.actif)
            .map((c) => (
              <button
                key={c.id}
                role="tab"
                aria-selected={categorie === c.id && !recherche}
                className={categorie === c.id && !recherche ? "actif" : ""}
                onClick={() => {
                  setCategorie(c.id);
                  setRecherche("");
                }}
              >
                <span aria-hidden>{c.icone}</span> {c.nom}
              </button>
            ))}
        </div>
        <div className="titre-catalogue">
          <h2>{recherche ? `Recherche « ${recherche} »` : `Choisir : ${categorieCourante?.nom ?? ""}`}</h2>
          <span className="aide">
            {produits.length} {produits.length > 1 ? "résultats" : "résultat"}
          </span>
        </div>
        <div className="produits">
          {produits.map((p) => {
            const dispo = cat.disponibles?.[p.id];
            return (
              <button
                key={p.id}
                className={`produit ${p.disponible ? "" : "rupture"}`}
                onClick={() => modifiable && toucherProduit(p)}
                disabled={!modifiable}
                aria-label={`${p.nom} ${nombre(prixZone(p, cmd.zone_id, promos))} FCFA`}
              >
                <VisuelPlat photo={p.photo} categorie={categorieDe(p)} />
                <span className="nom-plat">{p.nom}</span>
                <span className="prix-plat">{p.disponible ? fcfa(prixZone(p, cmd.zone_id, promos)) : "Rupture"}</span>
                {p.disponible && dispo !== undefined && (
                  <span className="dispo">
                    <strong>{nombre(dispo)}</strong> disponible{dispo > 1 ? "s" : ""}
                  </span>
                )}
                {p.disponible && enCours?.[p.id] && <span className="pastille promo">{enCours[p.id].promotion}</span>}
              </button>
            );
          })}
        </div>
      </section>

      {!ticketOuvert && (
        <button className="principal barre-ticket" onClick={() => setTicketOuvert(true)}>
          <span>Voir la commande ({nbArticles})</span>
          <strong>{fcfa(totalAffiche)}</strong>
        </button>
      )}
      {ticketOuvert && <div className="voile-ticket" onClick={() => setTicketOuvert(false)} aria-hidden />}
      <section className={`ticket ${ticketOuvert ? "ouvert" : ""}`} aria-label="Addition">
        <span className="poignee" aria-hidden />
        <div className="ticket-entete">
          <div>
            <small>Commande en cours · n°{cmd.numero}</small>
            <h2>{titre}</h2>
          </div>
          <span className={`statut ${cmd.statut}`}>{t(cmd.statut)}</span>
          <button className="fermer-ticket" onClick={() => setTicketOuvert(false)} aria-label="Fermer la commande">
            ✕
          </button>
        </div>
        <div className="types-commande" aria-label="Type de commande">
          {(["sur_place", "comptoir", "emporter", "livraison"] as const)
            .filter((x) => x === cmd.type || (x !== "comptoir" && x !== "livraison"))
            .map((x) => (
              <span key={x} className={`type-pastille ${cmd.type === x ? "actif" : ""}`}>
                {t(x)}
              </span>
            ))}
        </div>
        {cmd.client_nom && <p className="aide">Client : {cmd.client_nom}</p>}
        {cmd.type === "livraison" && (
          <p className="aide">
            <Bike size={16} className="icone-texte" aria-hidden /> {cmd.livraison_quartier} — {cmd.livraison_repere} — {cmd.livraison_telephone}
          </p>
        )}
        <ul className="lignes">
          {cmd.lignes
            .filter((l) => l.quantite > l.quantite_annulee)
            .map((l) => {
              const p = cat.produits.find((x) => x.id === l.produit_id);
              return (
                <li key={l.id} className={`ligne ${l.statut} ${l.offert ? "offerte" : ""}`}>
                  <button className="ligne-bouton" onClick={() => modifiable && setLigneAction(l)} disabled={!modifiable}>
                    <VisuelPlat photo={p?.photo} categorie={p && categorieDe(p)} petit />
                    <span className="detail">
                      <span className="lib">
                        {l.libelle}
                        {l.options.length > 0 && <small>{l.options.map((o) => o.nom).join(", ")}</small>}
                        {l.commentaire && <small className="note">« {l.commentaire} »</small>}
                      </span>
                      <span>
                        <span className="qte">{l.quantite - l.quantite_annulee}×</span> <span className="etat">{l.offert ? "Offert" : t(l.statut)}</span>
                      </span>
                    </span>
                    <span className="prix">{nombre(l.offert ? 0 : l.montant)}</span>
                  </button>
                </li>
              );
            })}
          {panier.map((a) => {
            const p = cat.produits.find((x) => x.id === a.produit_id);
            return (
              <li key={a.cle} className="ligne panier">
                <VisuelPlat photo={p?.photo} categorie={p && categorieDe(p)} petit />
                <span className="detail">
                  <button className="lib lien-ligne" onClick={() => setArticleEdite(a)} aria-label={`Options et commentaire ${a.libelle}`}>
                    {a.libelle}
                    {a.options.length > 0 && <small>{a.options.map((o) => o.nom).join(", ")}</small>}
                    {a.commentaire && <small className="note">« {a.commentaire} »</small>}
                  </button>
                  <span className="boutons-qte">
                    <button onClick={() => setPanier((x) => changerQuantite(x, a.cle, -1))} aria-label={`Retirer un ${a.libelle}`}>
                      −
                    </button>
                    <strong className="qte">{a.quantite}×</strong>
                    <button onClick={() => setPanier((x) => changerQuantite(x, a.cle, 1))} aria-label={`Ajouter un ${a.libelle}`}>
                      +
                    </button>
                  </span>
                </span>
                <span className="prix">{nombre(a.quantite * (a.prix + a.options.reduce((s, o) => s + o.supplement, 0)))}</span>
              </li>
            );
          })}
        </ul>
        <div className="resume-ticket">
          <div>
            <span>Articles ({nbArticles})</span>
            <span>{fcfa(cmd.totaux.brut + totalPanier(panier))}</span>
          </div>
        </div>
        {cmd.remises.filter((r) => r.montant > 0).length > 0 && <p className="aide">Remises : −{fcfa(cmd.totaux.remises)}</p>}
        <div className="total">
          <span>Total</span>
          <Montant valeur={totalAffiche} fort />
        </div>
        {cmd.totaux.paye > 0 && (
          <div className="sous-total">
            Payé {fcfa(cmd.totaux.paye)} — reste <strong>{fcfa(cmd.totaux.reste)}</strong>
          </div>
        )}
        <div className="actions-ticket">
          {modifiable && panier.length > 0 && cmd.ordre_paiement === "apres" && (
            <button className="principal grand" onClick={() => envoyer(true)} disabled={envoiEnCours}>
              Envoyer ({panier.reduce((s, a) => s + a.quantite, 0)})
            </button>
          )}
          {modifiable && cmd.employe_id && (
            <button
              className="principal grand"
              onClick={() =>
                agir(async (pin) => {
                  if (panier.length) await post(`/commandes/${id}/lignes`, versLignes(panier), pin);
                  setPanier([]);
                  await post(`/commandes/${id}/imputer`, {}, pin);
                  nav("/salle");
                }, "Imputé sur le compte de l'employé")
              }
            >
              Imputer à l'employé
            </button>
          )}
          {modifiable && !cmd.employe_id && peut("caisse.encaisser") && (cmd.totaux.reste > 0 || panier.length > 0) && (
            <button className={cmd.ordre_paiement === "avant" ? "principal grand" : "grand"} onClick={encaisser} disabled={envoiEnCours}>
              Encaisser {fcfa(cmd.totaux.reste + totalPanier(panier))}
            </button>
          )}
          <button onClick={() => setPlus(true)}>Plus…</button>
          <button onClick={() => nav("/salle")}>← Salle</button>
        </div>
        {cmd.totaux.paye !== 0 && <HistoriquePaiements commandeId={id} />}
        {cmd.envois.length > 0 && (
          <details className="envois">
            <summary>Envois ({cmd.envois.length})</summary>
            {cmd.envois.map((e) => (
              <div key={e.id} className={`envoi ${e.statut}`}>
                Tournée {e.numero} — {e.poste_nom ?? "direct"} — {t(e.statut)} {e.message && `: ${e.message}`}
              </div>
            ))}
          </details>
        )}
      </section>

      {produitOptions && (
        <ChoixOptions
          produit={produitOptions}
          fermer={() => setProduitOptions(null)}
          valider={(options, commentaire) => {
            setPanier((x) => ajouter(x, produitOptions, cmd.zone_id, options, commentaire, promos));
            setProduitOptions(null);
          }}
        />
      )}
      {articleEdite && cat.produits.find((p) => p.id === articleEdite.produit_id) && (
        <ChoixOptions
          produit={cat.produits.find((p) => p.id === articleEdite.produit_id)!}
          initial={articleEdite}
          fermer={() => setArticleEdite(null)}
          valider={(options, commentaire) => {
            setPanier((x) => x.map((a) => (a.cle === articleEdite.cle ? { ...a, options, commentaire } : a)));
            setArticleEdite(null);
          }}
        />
      )}
      {ligneAction && <ActionsLigne ligne={ligneAction} commande={cmd} fermer={() => setLigneAction(null)} recharger={recharger} />}
      {plus && <PlusDActions commande={cmd} fermer={() => setPlus(false)} recharger={recharger} />}
    </div>
  );
}

function ChoixOptions({
  produit,
  initial,
  fermer,
  valider,
}: {
  produit: Produit;
  initial?: ArticlePanier;
  fermer: () => void;
  valider: (o: ArticlePanier["options"], c: string) => void;
}) {
  const [choix, setChoix] = useState<string[]>(initial?.options.map((o) => o.id) ?? []);
  const [commentaire, setCommentaire] = useState(initial?.commentaire ?? "");
  const erreur = optionsValides(produit, choix);
  const basculer = (gid: string, oid: string, max: number) =>
    setChoix((c) => {
      if (c.includes(oid)) return c.filter((x) => x !== oid);
      const g = produit.groupes_options.find((g) => g.id === gid)!;
      const dansGroupe = c.filter((x) => g.options.some((o) => o.id === x));
      const base = max === 1 ? c.filter((x) => !dansGroupe.includes(x)) : c;
      return [...base, oid];
    });
  const toutes = produit.groupes_options.flatMap((g) => g.options);
  return (
    <Modal titre={produit.nom} fermer={fermer}>
      {produit.groupes_options.map((g) => (
        <div key={g.id} className="groupe-options">
          <h3>
            {g.nom} {g.min_choix > 0 && <small>(obligatoire)</small>}
          </h3>
          <div className="suggestions">
            {g.options.map((o) => (
              <button key={o.id} className={choix.includes(o.id) ? "actif" : ""} onClick={() => basculer(g.id, o.id, g.max_choix)}>
                {o.nom} {o.supplement > 0 && `+${nombre(o.supplement)}`}
              </button>
            ))}
          </div>
        </div>
      ))}
      <Champ libelle="Commentaire pour la cuisine" valeur={commentaire} changer={setCommentaire} placeholder="bien cuit, sans piment…" />
      {erreur && <p className="erreur-texte">{erreur}</p>}
      <div className="actions">
        <button onClick={fermer}>Annuler</button>
        <button
          className="principal"
          disabled={!!erreur}
          onClick={() =>
            valider(
              toutes.filter((o) => choix.includes(o.id)),
              commentaire.trim(),
            )
          }
        >
          {initial ? "Valider" : "Ajouter"}
        </button>
      </div>
    </Modal>
  );
}

function ActionsLigne({ ligne, commande, fermer, recharger }: { ligne: Ligne; commande: Commande; fermer: () => void; recharger: () => void }) {
  const { agir } = useApp();
  const [mode, setMode] = useState<"menu" | "annuler" | "offrir" | "remise">("menu");
  const [quantite, setQuantite] = useState(ligne.quantite - ligne.quantite_annulee);
  const [perte, setPerte] = useState(false);
  const [montant, setMontant] = useState(0);
  const envoyee = ligne.statut !== "brouillon";
  const fin = () => {
    recharger();
    fermer();
  };
  if (mode === "annuler")
    return (
      <DemandeMotif
        titre={`Annuler ${ligne.libelle}`}
        suggestions={envoyee ? ["Client a changé d'avis", "Erreur de saisie", "Trop d'attente", "Plat raté"] : ["Erreur de saisie"]}
        fermer={fermer}
        enfants={
          <>
            {ligne.quantite - ligne.quantite_annulee > 1 && (
              <label className="champ">
                <span>Quantité à annuler</span>
                <input
                  type="number"
                  min={1}
                  max={ligne.quantite - ligne.quantite_annulee}
                  value={quantite}
                  onChange={(e) => setQuantite(Number(e.target.value))}
                />
              </label>
            )}
            {envoyee && (
              <label className="case">
                <input type="checkbox" checked={perte} onChange={(e) => setPerte(e.target.checked)} />
                <span>Déjà préparé / perdu (ne retourne pas en stock)</span>
              </label>
            )}
          </>
        }
        valider={(motif) => agir((pin) => post(`/lignes/${ligne.id}/annuler`, { quantite, motif, perte }, pin), "Article annulé").then(fin)}
      />
    );
  if (mode === "offrir")
    return (
      <DemandeMotif
        titre={`Offrir ${ligne.libelle}`}
        suggestions={["Client fidèle", "Geste commercial", "Erreur de service", "Anniversaire"]}
        fermer={fermer}
        valider={(motif) => agir((pin) => post(`/lignes/${ligne.id}/offrir`, { motif }, pin), "Article offert").then(fin)}
      />
    );
  if (mode === "remise")
    return (
      <DemandeMotif
        titre={`Remise sur ${ligne.libelle}`}
        suggestions={["Client fidèle", "Geste commercial"]}
        fermer={fermer}
        enfants={<ChampMontant libelle="Montant de la remise" valeur={montant} changer={setMontant} autoFocus />}
        valider={(motif) => agir((pin) => post(`/commandes/${commande.id}/remise`, { ligne_id: ligne.id, montant, motif }, pin), "Remise accordée").then(fin)}
      />
    );
  return (
    <Modal titre={ligne.libelle} fermer={fermer}>
      <div className="menu-actions">
        <button className="attention" onClick={() => setMode("annuler")}>
          Annuler {envoyee && "(motif + autorisation)"}
        </button>
        {!ligne.offert && <button onClick={() => setMode("offrir")}>Offrir</button>}
        {!ligne.offert && <button onClick={() => setMode("remise")}>Remise</button>}
      </div>
    </Modal>
  );
}

function PlusDActions({ commande, fermer, recharger }: { commande: Commande; fermer: () => void; recharger: () => void }) {
  const { agir, peut } = useApp();
  const nav = useNavigate();
  const [mode, setMode] = useState<"menu" | "remise" | "transfert" | "fusion" | "client" | "ticket" | "employe">("menu");
  const [montant, setMontant] = useState(0);
  const [pourcentage, setPourcentage] = useState(0);
  const { donnees: salle } = useDonnees(() => get<{ tables: TablePlan[] }>("/salle"), [], [mode]);
  const [clients, setClients] = useState<Client[]>([]);
  const [q, setQ] = useState("");
  const [ticket, setTicket] = useState("");
  const fin = () => {
    recharger();
    fermer();
  };

  useEffect(() => {
    if (mode === "client")
      get<Client[]>(`/clients?q=${encodeURIComponent(q)}`)
        .then(setClients)
        .catch(() => {});
  }, [mode, q]);

  if (mode === "remise")
    return (
      <DemandeMotif
        titre="Remise sur l'addition"
        suggestions={["Client fidèle", "Geste commercial", "Réclamation"]}
        fermer={fermer}
        enfants={
          <>
            <ChampMontant
              libelle="Montant (FCFA)"
              valeur={montant}
              changer={(v) => {
                setMontant(v);
                setPourcentage(0);
              }}
            />
            <div className="suggestions">
              {[5, 10, 15, 20].map((p) => (
                <button
                  key={p}
                  className={pourcentage === p ? "actif" : ""}
                  onClick={() => {
                    setPourcentage(p);
                    setMontant(0);
                  }}
                >
                  {p} %
                </button>
              ))}
            </div>
          </>
        }
        valider={(motif) =>
          agir(
            (pin) => post(`/commandes/${commande.id}/remise`, { montant: montant || null, pourcentage: pourcentage || null, motif }, pin),
            "Remise accordée",
          ).then(fin)
        }
      />
    );
  if (mode === "transfert" || mode === "fusion") {
    const tables = (salle?.tables ?? []).filter((t) => t.id !== commande.table_id && (mode === "transfert" ? t.statut !== "occupee" : t.statut === "occupee"));
    return (
      <Modal titre={mode === "transfert" ? "Transférer vers une table libre" : "Fusionner avec la table…"} fermer={fermer}>
        <div className="plan-salle petit">
          {tables.map((t) => (
            <button
              key={t.id}
              className={`table-salle ${t.statut}`}
              onClick={() =>
                agir(async (pin) => {
                  if (mode === "transfert") await post(`/commandes/${commande.id}/transferer`, { cible: t.id }, pin);
                  else {
                    await post(`/commandes/${commande.id}/fusionner`, { cible: t.commande_id }, pin);
                    nav(`/commande/${t.commande_id}`);
                  }
                }, "Fait").then(fin)
              }
            >
              <strong>{t.nom}</strong>
            </button>
          ))}
        </div>
      </Modal>
    );
  }
  if (mode === "client")
    return (
      <Modal titre="Client de l'addition" fermer={fermer}>
        <Champ libelle="Nom ou téléphone" valeur={q} changer={setQ} autoFocus />
        <div className="liste-commandes">
          {clients.map((c) => (
            <button
              key={c.id}
              className="ligne-commande"
              onClick={() => agir((pin) => post(`/commandes/${commande.id}/client`, { client_id: c.id }, pin), `Client : ${c.nom}`).then(fin)}
            >
              <strong>{c.nom}</strong> <span>{c.telephone}</span>{" "}
              <span>{c.credit_autorise ? `Crédit ${fcfa(c.limite_credit - c.dette)} dispo.` : "Pas de crédit"}</span>
            </button>
          ))}
        </div>
      </Modal>
    );
  if (mode === "employe") return <ChoixEmploye fermer={fermer} />;
  if (mode === "ticket")
    return (
      <Modal titre="Addition" fermer={fermer}>
        <pre className="apercu-ticket">{ticket}</pre>
        <div className="actions">
          <button onClick={() => window.print()}>Imprimer (navigateur)</button>
          <button className="principal" onClick={() => agir(() => post(`/commandes/${commande.id}/imprimer`), "Envoyé à l'imprimante de caisse")}>
            Imprimante ticket
          </button>
        </div>
      </Modal>
    );
  return (
    <Modal titre="Actions" fermer={fermer}>
      <div className="menu-actions">
        <button
          onClick={async () => {
            setTicket(await get<string>(`/commandes/${commande.id}/ticket`));
            setMode("ticket");
          }}
        >
          <Receipt size={20} aria-hidden /> Addition / ticket
        </button>
        {commande.statut === "ouverte" && (
          <>
            <button onClick={() => setMode("remise")}>Remise sur l'addition</button>
            {commande.table_id && <button onClick={() => setMode("transfert")}>Transférer de table</button>}
            {commande.table_id && <button onClick={() => setMode("fusion")}>Fusionner des tables</button>}
            <button onClick={() => setMode("client")}>Associer un client (crédit)</button>
            {peut("commande.conso_employe") && commande.lignes.length === 0 && !commande.employe_id && (
              <button onClick={() => setMode("employe")}>Consommation d'un employé…</button>
            )}
            <button
              className="attention"
              onClick={() => agir((pin) => post(`/commandes/${commande.id}/abandonner`, {}, pin), "Addition abandonnée").then(() => nav("/salle"))}
            >
              Abandonner l'addition vide
            </button>
          </>
        )}
      </div>
    </Modal>
  );
}

/** Nouvelle commande « consommation employé » (RG-CMD-07). */
function ChoixEmploye({ fermer }: { fermer: () => void }) {
  const { agir } = useApp();
  const nav = useNavigate();
  const { donnees } = useDonnees(
    () =>
      get<Employe[]>("/employes").catch((e: ErreurApi) => {
        throw e;
      }),
    [],
  );
  return (
    <Modal titre="Repas imputé à un employé" fermer={fermer}>
      <p className="aide">Pas de chiffre d'affaires : le montant sera retenu sur son compte.</p>
      <div className="liste-commandes">
        {(donnees ?? []).map((e) => (
          <button
            key={e.id}
            className="ligne-commande"
            onClick={() =>
              agir(async (pin) => {
                const id = await post<string>("/commandes", { type: "comptoir", employe_id: e.id }, pin);
                nav(`/commande/${id}`);
                fermer();
              })
            }
          >
            <strong>{e.nom}</strong> <span>{e.fonction}</span>
          </button>
        ))}
      </div>
    </Modal>
  );
}

type PaiementLu = {
  id: string;
  numero: number;
  montant: number;
  recu: number;
  rendu: number;
  horodatage: number;
  annule: boolean;
  est_annulation: boolean;
  parts: [string, number, string | null][];
};

/** Paiements de l'addition : argent reçu et monnaie rendue (RG-CAI-14). */
function HistoriquePaiements({ commandeId }: { commandeId: string }) {
  const { donnees } = useDonnees(() => get<PaiementLu[]>(`/commandes/${commandeId}/paiements`), ["paiement"], [commandeId]);
  return (
    <details className="envois" open>
      <summary>Paiements ({donnees?.length ?? 0})</summary>
      {(donnees ?? []).map((p) => (
        <div key={p.id} className={`envoi ${p.annule || p.est_annulation ? "probleme" : ""}`}>
          Reçu n°{p.numero} — {fcfa(p.montant)} ({p.parts.map(([m, v, c]) => `${m === "mobile_money" ? c : t(m)} ${nombre(v)}`).join(", ")})
          {p.recu > 0 && (
            <div>
              Reçu du client {fcfa(p.recu)} · monnaie rendue {fcfa(p.rendu)}
            </div>
          )}
          {p.annule && " — annulé"}
        </div>
      ))}
    </details>
  );
}
