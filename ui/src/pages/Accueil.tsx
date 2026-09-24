import { Link, useNavigate } from "react-router-dom";
import { post } from "../api";
import { useApp } from "../contexte";
import { dateFr } from "../format";

export const MENU: { chemin: string; libelle: string; icone: string; permission?: string }[] = [
  { chemin: "/salle", libelle: "Salle et commandes", icone: "🍽️", permission: "commande.creer" },
  { chemin: "/caisse", libelle: "Caisse", icone: "💰", permission: "caisse.session" },
  { chemin: "/cuisine", libelle: "Cuisine / Bar", icone: "👨‍🍳", permission: "cuisine.voir" },
  { chemin: "/livraisons", libelle: "Livraisons", icone: "🛵", permission: "livraison.gerer" },
  { chemin: "/sortie", libelle: "Contrôle de sortie", icone: "🎫", permission: "sortie.controler" },
  { chemin: "/tableau-de-bord", libelle: "Ma journée", icone: "📊", permission: "rapport.voir" },
  { chemin: "/mobile-money", libelle: "Mobile Money", icone: "📱", permission: "caisse.verifier_mm" },
  { chemin: "/stock", libelle: "Stock", icone: "📦", permission: "stock.voir" },
  { chemin: "/achats", libelle: "Achats", icone: "🛒", permission: "achat.gerer" },
  { chemin: "/clients", libelle: "Clients et crédit", icone: "🧾", permission: "client.gerer" },
  { chemin: "/employes", libelle: "Employés", icone: "👥", permission: "employe.voir" },
  { chemin: "/paie", libelle: "Paie", icone: "💵", permission: "paie.gerer" },
  { chemin: "/rapports", libelle: "Rapports", icone: "📈", permission: "rapport.voir" },
  { chemin: "/journal", libelle: "Journal d'audit", icone: "🔎", permission: "audit.voir" },
  { chemin: "/administration", libelle: "Administration", icone: "⚙️", permission: "catalogue.gerer" },
];

export default function Accueil() {
  const { etat, session, peut, agir, rechargerEtat } = useApp();
  const nav = useNavigate();
  const liens = MENU.filter((m) => !m.permission || peut(m.permission));

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
            {peut("journee.gerer") && (
              <button onClick={cloturerJournee} className="attention">
                Clôturer la journée
              </button>
            )}
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
      {etat?.journee && peut("commande.creer") && (
        <div className="actions-rapides">
          <Link className="bouton principal grand" to="/salle">
            🍽️ Tables
          </Link>
          <button className="grand" onClick={comptoir}>
            🥤 Vente comptoir
          </button>
        </div>
      )}
      <div className="grille-menu">
        {liens.map((m) => (
          <Link key={m.chemin} to={m.chemin} className="tuile">
            <span className="icone" aria-hidden>
              {m.icone}
            </span>
            <strong>{m.libelle}</strong>
          </Link>
        ))}
      </div>
    </div>
  );
}
