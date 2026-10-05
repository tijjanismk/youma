import type { LucideIcon } from "lucide-react";
import { appliquerMenuPerso, lireMenuPerso } from "../menuPerso";
import {
  Banknote,
  Bike,
  ChartColumn,
  ChartPie,
  ChefHat,
  Inbox,
  LayoutGrid,
  Package,
  ReceiptText,
  ScrollText,
  Settings,
  ShoppingBag,
  ShoppingCart,
  Smartphone,
  Ticket,
  Users,
  Wallet,
} from "lucide-react";
import { Link, useNavigate } from "react-router";
import { CarteInstallation } from "../composants/Installation";
import { CarteMenuDuJour } from "../composants/MenuDuJour";
import { get, post } from "../api";
import { useApp, useDonnees } from "../contexte";
import { dateFr } from "../format";
import type { EtatGeneral } from "../types";

export type EntreeMenu = { chemin: string; libelle: string; court: string; Icone: LucideIcon; permission?: string };

export const MENU: EntreeMenu[] = [
  { chemin: "/salle", libelle: "Salle et commandes", court: "Salle", Icone: LayoutGrid, permission: "commande.creer" },
  { chemin: "/entrantes", libelle: "Commandes reçues", court: "Reçues", Icone: Inbox, permission: "commande.valider_entrante" },
  { chemin: "/caisse", libelle: "Caisse", court: "Caisse", Icone: Wallet, permission: "caisse.session" },
  { chemin: "/cuisine", libelle: "Cuisine / Bar", court: "Cuisine", Icone: ChefHat, permission: "cuisine.voir" },
  { chemin: "/livraisons", libelle: "Livraisons", court: "Livraisons", Icone: Bike, permission: "livraison.gerer" },
  { chemin: "/sortie", libelle: "Contrôle de sortie", court: "Sortie", Icone: Ticket, permission: "sortie.controler" },
  { chemin: "/tableau-de-bord", libelle: "Ma journée", court: "Journée", Icone: ChartPie, permission: "rapport.voir" },
  { chemin: "/mobile-money", libelle: "Mobile Money", court: "Mobile M.", Icone: Smartphone, permission: "caisse.verifier_mm" },
  { chemin: "/stock", libelle: "Stock", court: "Stock", Icone: Package, permission: "stock.voir" },
  { chemin: "/achats", libelle: "Achats", court: "Achats", Icone: ShoppingCart, permission: "achat.gerer" },
  { chemin: "/clients", libelle: "Clients et crédit", court: "Clients", Icone: ReceiptText, permission: "client.gerer" },
  { chemin: "/employes", libelle: "Employés", court: "Employés", Icone: Users, permission: "employe.voir" },
  { chemin: "/paie", libelle: "Paie", court: "Paie", Icone: Banknote, permission: "paie.gerer" },
  { chemin: "/rapports", libelle: "Rapports", court: "Rapports", Icone: ChartColumn, permission: "rapport.voir" },
  { chemin: "/journal", libelle: "Journal d'audit", court: "Journal", Icone: ScrollText, permission: "audit.voir" },
  { chemin: "/administration", libelle: "Administration", court: "Réglages", Icone: Settings, permission: "catalogue.gerer" },
];

/**
 * Entrées du menu permises ; « Commandes reçues » seulement si le QR ou l'en ligne est activé. Avec `utilisateur` :
 * ordre et écrans cachés choisis par cet utilisateur sur cet appareil (fiche 0051).
 */
export function menuVisible(permissions: string[], etat: EtatGeneral | null, utilisateur?: string) {
  const distance = !!(etat?.parametres?.canaux?.qr_table || etat?.parametres?.canaux?.en_ligne);
  const permis = MENU.filter((m) => (!m.permission || permissions.includes(m.permission)) && (m.chemin !== "/entrantes" || distance));
  return utilisateur ? appliquerMenuPerso(permis, lireMenuPerso(utilisateur)) : permis;
}

export default function Accueil() {
  const { etat, session, peut, agir, rechargerEtat } = useApp();
  const nav = useNavigate();
  const liens = menuVisible(session?.permissions ?? [], etat, session?.utilisateur.id);

  const ouvrirJournee = () =>
    agir(async (pin) => {
      await post("/journee/ouvrir", {}, pin);
      await rechargerEtat();
    }, "Journée ouverte");

  const cloturerJournee = () =>
    agir(async (pin) => {
      await post("/journee/cloturer", {}, pin);
      await rechargerEtat();
    }, "Journée clôturée et sauvegardée");

  const comptoir = () =>
    agir(async (pin) => {
      const id = await post<string>("/commandes", { type: "comptoir" }, pin);
      nav(`/commande/${id}`);
    });

  return (
    <div>
      <h1>Bonjour {session?.utilisateur.nom}</h1>
      <div className="carte journee-carte">
        {etat?.journee ? (
          <>
            <span>
              Journée d'exploitation du <strong>{dateFr(etat.journee.date_exploitation)}</strong> ouverte
            </span>
            {peut("journee.gerer") && <ClotureJournee cloturer={cloturerJournee} />}
          </>
        ) : (
          <>
            <span>Aucune journée ouverte : les ventes sont impossibles.</span>
            {peut("journee.gerer") && (
              <button className="principal grand" onClick={ouvrirJournee}>
                Ouvrir la journée
              </button>
            )}
          </>
        )}
      </div>
      <CarteMenuDuJour />
      {etat?.journee && peut("commande.creer") && (
        <div className="actions-rapides">
          <Link className="bouton principal grand" to="/salle">
            <LayoutGrid size={20} aria-hidden /> Tables
          </Link>
          <button className="grand" onClick={comptoir}>
            <ShoppingBag size={20} aria-hidden /> Vente comptoir
          </button>
        </div>
      )}
      <div className="grille-menu">
        {liens.map((m) => (
          <Link key={m.chemin} to={m.chemin} className="tuile">
            <span className="icone" aria-hidden>
              <m.Icone size={26} strokeWidth={1.8} />
            </span>
            <strong>{m.libelle}</strong>
          </Link>
        ))}
      </div>
      <CarteInstallation />
    </div>
  );
}

/** RG-JOU-04 : le bouton n'est actif que si rien n'empêche la clôture ; sinon on dit pourquoi. */
function ClotureJournee({ cloturer }: { cloturer: () => void }) {
  const { donnees: raisons } = useDonnees(
    () => get<string[]>("/journee/blocages"),
    ["commande", "commande_entrante", "caisse", "paiement", "table", "journee"],
  );
  const bloquee = (raisons ?? []).length > 0;
  return (
    <div className="cloture-journee">
      <button onClick={cloturer} className="attention" disabled={bloquee}>
        Clôturer la journée
      </button>
      {bloquee && (
        <ul className="raison-bloquee" aria-label="Clôture impossible">
          {raisons!.map((r) => (
            <li key={r}>{r}</li>
          ))}
        </ul>
      )}
    </div>
  );
}
