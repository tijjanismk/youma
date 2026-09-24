import { lazy, Suspense, useEffect, useState } from "react";
import { BrowserRouter, Link, Navigate, Route, Routes, useLocation, useNavigate } from "react-router-dom";
import { definirJetonAppareil, post } from "./api";
import { Fournisseur, useApp } from "./contexte";
import { dateFr, dateHeure } from "./format";
import Connexion from "./pages/Connexion";
import Installation from "./pages/Installation";
import Accueil, { MENU } from "./pages/Accueil";
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

export default function App() {
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
      {!enLigne && (
        <div className="bandeau erreur" role="alert">
          Poste central injoignable — vos saisies sont conservées, reconnexion automatique…
        </div>
      )}
      {etat?.demo && <div className="bandeau demo">BASE DE DÉMONSTRATION — aucune donnée réelle</div>}
      {etat && !etat.horloge.coherente && (
        <div className="bandeau erreur" role="alert">
          L'horloge du PC ({dateHeure(etat.horloge.maintenant)}) est antérieure au dernier enregistrement ({dateHeure(etat.horloge.dernier_evenement)}).
          Les ventes sont bloquées : corrigez la date de Windows.
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
  const { notifier } = useApp();
  useEffect(() => {
    const code = new URLSearchParams(loc.search).get("appairage");
    if (!code) return;
    const nom = prompt("Nom de cet appareil (ex. « Téléphone Awa »)") ?? "";
    post<{ jeton: string }>("/appareils/appairer", { code, nom })
      .then((r) => {
        definirJetonAppareil(r.jeton);
        notifier("Appareil autorisé", "succes");
        nav("/", { replace: true });
      })
      .catch((e) => notifier(e.message, "erreur"));
  }, [loc.search, nav, notifier]);
  return null;
}

function Coquille() {
  const { etat, session, deconnecter } = useApp();
  const [menu, setMenu] = useState(false);
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
  const liens = MENU.filter((m) => !m.permission || session.permissions.includes(m.permission));
  return (
    <div className="application">
      <Bandeaux />
      <header className="entete">
        <button className="burger" onClick={() => setMenu(!menu)} aria-label="Menu">
          ☰
        </button>
        <Link to="/" className="marque">
          {etat.restaurant}
        </Link>
        <span className="journee">{etat.journee ? `Journée du ${dateFr(etat.journee.date_exploitation)}` : "Journée fermée"}</span>
        <span className="utilisateur">{session.utilisateur.nom}</span>
        <button className="petit" onClick={deconnecter}>
          Changer d'utilisateur
        </button>
      </header>
      <div className="corps">
        <nav className={`menu ${menu ? "ouvert" : ""}`} aria-label="Menu principal">
          {liens.map((m) => (
            <Link key={m.chemin} to={m.chemin} className={loc.pathname.startsWith(m.chemin) && m.chemin !== "/" ? "actif" : ""}>
              <span aria-hidden>{m.icone}</span> {m.libelle}
            </Link>
          ))}
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
              <Route path="*" element={<Navigate to="/" replace />} />
            </Routes>
          </Suspense>
        </main>
      </div>
    </div>
  );
}
