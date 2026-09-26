import { ArrowLeftRight, House, Menu as MenuIcone, Moon, PanelLeftClose, PanelLeftOpen, Sun } from "lucide-react";
import { lazy, Suspense, useEffect, useState } from "react";
import { BrowserRouter, Link, Navigate, Route, Routes, useLocation, useNavigate } from "react-router-dom";
import { definirJetonAppareil, get, post } from "./api";
import { BandeauMiseAJour } from "./composants/Installation";
import { Fournisseur, useApp } from "./contexte";
import { dateFr, dateHeure } from "./format";
import Connexion from "./pages/Connexion";
import { choisirTheme, lireTheme, Theme } from "./theme";
import Installation from "./pages/Installation";
import Accueil, { menuVisible } from "./pages/Accueil";
import Salle from "./pages/Salle";
import PriseCommande from "./pages/PriseCommande";
import Encaissement from "./pages/Encaissement";
import Caisse from "./pages/Caisse";
import Cuisine from "./pages/Cuisine";
import TableauDeBord from "./pages/TableauDeBord";

// Écrans moins fréquents : chargés à la demande (téléphones modestes).
const Stock = lazy(() => import("./pages/Stock"));
const Achats = lazy(() => import("./pages/Achats"));
const Clients = lazy(() => import("./pages/Clients"));
const Employes = lazy(() => import("./pages/Employes"));
const FicheEmploye = lazy(() => import("./pages/FicheEmploye"));
const Paie = lazy(() => import("./pages/Paie"));
const Livraisons = lazy(() => import("./pages/Livraisons"));
const Rapports = lazy(() => import("./pages/Rapports"));
const MobileMoney = lazy(() => import("./pages/MobileMoney"));
const Administration = lazy(() => import("./pages/Administration"));
const Journal = lazy(() => import("./pages/Journal"));
const Sortie = lazy(() => import("./pages/Sortie"));
const Entrantes = lazy(() => import("./pages/Entrantes"));
// Pages publiques (client, livreur) : sans connexion ni appairage.
const MenuClient = lazy(() => import("./public/MenuClient"));
const SuiviClient = lazy(() => import("./public/Suivi"));
const Livreur = lazy(() => import("./public/Livreur"));
const Proprietaire = lazy(() => import("./public/Proprietaire"));

/** Pages ouvertes par un QR ou un lien envoyé au client : hors de l'application du personnel. */
function PagePublique() {
  const chemin = location.pathname;
  // Espace propriétaire : installable à part, avec sa propre icône d'accueil.
  useEffect(() => {
    if (chemin.startsWith("/proprietaire")) document.querySelector("link[rel=manifest]")?.setAttribute("href", "/manifest-proprietaire.webmanifest");
  }, [chemin]);
  const code = decodeURIComponent(chemin.split("/")[2] ?? "");
  return (
    <Suspense fallback={<p className="aide">Chargement…</p>}>
      {chemin.startsWith("/menu") && <MenuClient />}
      {chemin.startsWith("/suivi/") && <SuiviClient code={code} />}
      {chemin.startsWith("/livreur/") && <Livreur code={code} />}
      {chemin.startsWith("/proprietaire") && <Proprietaire />}
    </Suspense>
  );
}

export function estPagePublique(chemin: string) {
  return (
    chemin === "/menu" || chemin.startsWith("/menu/") || chemin.startsWith("/suivi/") || chemin.startsWith("/livreur/") || chemin.startsWith("/proprietaire")
  );
}

export default function App() {
  if (estPagePublique(location.pathname)) return <PagePublique />;
  return (
    <Fournisseur>
      <BrowserRouter future={{ v7_startTransition: true, v7_relativeSplatPath: true }}>
        <Coquille />
      </BrowserRouter>
    </Fournisseur>
  );
}

function Bandeaux() {
  const { etat, enLigne, session, agir, rechargerEtat, peut } = useApp();
  return (
    <>
      <BandeauMiseAJour />
      {!enLigne && (
        <div className="bandeau erreur" role="alert">
          Poste central injoignable — vos saisies sont conservées, reconnexion automatique…
        </div>
      )}
      {etat?.demo && <div className="bandeau demo">BASE DE DÉMONSTRATION — aucune donnée réelle</div>}
      {etat && !etat.horloge.coherente && (
        <div className="bandeau erreur" role="alert">
          L'horloge du PC ({dateHeure(etat.horloge.maintenant)}) est antérieure au dernier enregistrement ({dateHeure(etat.horloge.dernier_evenement)}). Les
          ventes sont bloquées : corrigez la date de Windows.
          {session && peut("horloge.forcer") && (
            <button
              className="petit"
              onClick={() =>
                agir(async (pin) => {
                  await post("/horloge/accepter", {}, pin);
                  await rechargerEtat();
                }, "Nouvelle heure acceptée")
              }
            >
              L'heure du PC est la bonne
            </button>
          )}
        </div>
      )}
    </>
  );
}

function Appairage() {
  const loc = useLocation();
  const nav = useNavigate();
  const { notifier, rechargerEtat } = useApp();
  useEffect(() => {
    const code = new URLSearchParams(loc.search).get("appairage");
    if (!code) return;
    const nom = prompt("Nom de cet appareil (ex. « Téléphone Awa »)") ?? "";
    post<{ jeton: string }>("/appareils/appairer", { code, nom })
      .then((r) => {
        definirJetonAppareil(r.jeton);
        notifier("Appareil autorisé", "succes");
        nav("/", { replace: true });
        rechargerEtat();
      })
      .catch((e) => notifier(e.message, "erreur"));
  }, [loc.search, nav, notifier, rechargerEtat]);
  return null;
}

/** Nombre de commandes QR / en ligne en attente (pastille du menu), mis à jour en temps réel. */
function useEntrantesEnAttente(): number {
  const { session, etat, abonner } = useApp();
  const [n, setN] = useState(0);
  const actif = !!session?.permissions.includes("commande.valider_entrante") && !!(etat?.parametres?.canaux?.qr_table || etat?.parametres?.canaux?.en_ligne);
  useEffect(() => {
    if (!actif) return setN(0);
    const charger = () =>
      get<unknown[]>("/entrantes")
        .then((l) => setN(l.length))
        .catch(() => undefined);
    charger();
    return abonner((e) => {
      if (e.type === "commande_entrante" || e.type === "commande" || e.type === "resynchroniser") charger();
    });
  }, [actif, abonner]);
  return n;
}

/** Clair / Sombre, mémorisé par poste. */
function ChoixTheme() {
  const [theme, setTheme] = useState<Theme>(() => lireTheme());
  const sombre = theme === "sombre";
  const basculer = () => {
    const t: Theme = sombre ? "clair" : "sombre";
    choisirTheme(t);
    setTheme(t);
  };
  return (
    <button
      className="petit theme"
      onClick={basculer}
      aria-label="Thème sombre"
      aria-pressed={sombre}
      title={sombre ? "Passer en thème clair" : "Passer en thème sombre"}
    >
      {sombre ? <Sun size={20} aria-hidden /> : <Moon size={20} aria-hidden />}
    </button>
  );
}

const CLE_REPLIE = "youma.menu-replie";

function lireReplie(): boolean {
  try {
    return localStorage.getItem(CLE_REPLIE) === "1";
  } catch {
    return false;
  }
}

function Coquille() {
  const { etat, session, deconnecter } = useApp();
  const [menu, setMenu] = useState(false);
  const [replie, setReplie] = useState(() => lireReplie());
  const enAttente = useEntrantesEnAttente();
  const loc = useLocation();
  useEffect(() => setMenu(false), [loc.pathname]);

  if (!etat) {
    return (
      <div className="plein-ecran">
        <Bandeaux />
        <p className="aide">Connexion au poste central…</p>
      </div>
    );
  }
  if (etat.appairage_requis) {
    return (
      <div className="plein-ecran">
        <Appairage />
        <div className="carte etroite">
          <h1>Appareil non autorisé</h1>
          <p>{etat.message}</p>
          <p className="aide">
            Sur le poste central : Administration → Téléphones et tablettes → « Générer un code », puis scannez le QR code avec ce téléphone.
          </p>
        </div>
      </div>
    );
  }
  if (!etat.installe) return <Installation />;
  if (!session) {
    return (
      <>
        <Appairage />
        <Bandeaux />
        <Connexion />
      </>
    );
  }
  const liens = menuVisible(session.permissions, etat);
  const actif = (chemin: string) => loc.pathname.startsWith(chemin) && chemin !== "/";
  // Barre du bas (téléphone) : l'accueil, puis les trois écrans du quotidien permis à cet utilisateur.
  const priorite = ["/salle", "/entrantes", "/caisse", "/cuisine", "/livraisons", "/tableau-de-bord"];
  const bas = priorite
    .map((c) => liens.find((m) => m.chemin === c))
    .filter((m) => !!m)
    .slice(0, 3);
  const basculerReplie = () => {
    setReplie(!replie);
    try {
      localStorage.setItem(CLE_REPLIE, replie ? "0" : "1");
    } catch {
      /* stockage indisponible : réglage pour cette visite seulement */
    }
  };
  const badge = (chemin: string) => chemin === "/entrantes" && enAttente > 0 && <span className="badge">{enAttente}</span>;
  return (
    <div className={`application ${replie ? "replie" : ""}`}>
      <Bandeaux />
      <header className="entete">
        <button className="burger" onClick={() => setMenu(!menu)} aria-label="Menu">
          <MenuIcone size={22} />
        </button>
        <Link to="/" className="marque">
          {etat.restaurant}
        </Link>
        <span className="journee">{etat.journee ? `Journée du ${dateFr(etat.journee.date_exploitation)}` : "Journée fermée"}</span>
        <span className="avatar-mini" aria-hidden>
          {session.utilisateur.nom.trim().charAt(0).toUpperCase()}
        </span>
        <span className="utilisateur">{session.utilisateur.nom}</span>
        <ChoixTheme />
        <button className="petit changer" onClick={deconnecter} aria-label="Changer d'utilisateur" title="Changer d'utilisateur">
          <span className="texte-long">Changer d'utilisateur</span>
          <span className="texte-court" aria-hidden>
            <ArrowLeftRight size={20} />
          </span>
        </button>
      </header>
      <div className="corps">
        <nav className={`menu ${menu ? "ouvert" : ""}`} aria-label="Menu principal">
          <button
            className="replier"
            onClick={basculerReplie}
            aria-label={replie ? "Déplier le menu" : "Replier le menu"}
            title={replie ? "Déplier le menu" : "Replier le menu"}
          >
            {replie ? <PanelLeftOpen size={20} /> : <PanelLeftClose size={20} />}
          </button>
          <Link to="/" className={loc.pathname === "/" ? "actif" : ""} aria-label="Accueil" title="Accueil">
            <House size={22} strokeWidth={1.8} aria-hidden />
            <span>Accueil</span>
          </Link>
          {liens.map((m) => (
            <Link key={m.chemin} to={m.chemin} className={actif(m.chemin) ? "actif" : ""} aria-label={m.libelle} title={m.libelle}>
              <m.Icone size={22} strokeWidth={1.8} aria-hidden />
              <span>{replie ? m.court : m.libelle}</span>
              {badge(m.chemin)}
            </Link>
          ))}
        </nav>
        <nav className="barre-bas" aria-label="Accès rapide">
          <Link to="/" className={loc.pathname === "/" ? "actif" : ""} aria-label="Accueil">
            <House size={22} aria-hidden />
            <span className="libelle">Accueil</span>
          </Link>
          {bas.map((m) => (
            <Link key={m.chemin} to={m.chemin} className={actif(m.chemin) ? "actif" : ""} aria-label={m.libelle}>
              <span className="avec-badge">
                <m.Icone size={22} aria-hidden />
                {badge(m.chemin)}
              </span>
              <span className="libelle">{m.court}</span>
            </Link>
          ))}
          <button onClick={() => setMenu(!menu)} aria-label="Autres écrans">
            <MenuIcone size={22} aria-hidden />
            <span className="libelle">Plus</span>
          </button>
        </nav>
        <main className="contenu">
          <Suspense fallback={<p className="aide">Chargement…</p>}>
            <Routes>
              <Route path="/" element={<Accueil />} />
              <Route path="/salle" element={<Salle />} />
              <Route path="/commande/:id" element={<PriseCommande />} />
              <Route path="/encaisser/:id" element={<Encaissement />} />
              <Route path="/caisse" element={<Caisse />} />
              <Route path="/cuisine" element={<Cuisine />} />
              <Route path="/tableau-de-bord" element={<TableauDeBord />} />
              <Route path="/stock" element={<Stock />} />
              <Route path="/achats" element={<Achats />} />
              <Route path="/clients" element={<Clients />} />
              <Route path="/employes" element={<Employes />} />
              <Route path="/employes/:id" element={<FicheEmploye />} />
              <Route path="/paie" element={<Paie />} />
              <Route path="/livraisons" element={<Livraisons />} />
              <Route path="/rapports" element={<Rapports />} />
              <Route path="/mobile-money" element={<MobileMoney />} />
              <Route path="/administration" element={<Administration />} />
              <Route path="/journal" element={<Journal />} />
              <Route path="/sortie" element={<Sortie />} />
              <Route path="/entrantes" element={<Entrantes />} />
              <Route path="*" element={<Navigate to="/" replace />} />
            </Routes>
          </Suspense>
        </main>
      </div>
    </div>
  );
}
