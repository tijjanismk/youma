import { ArrowRight, Bike, ClipboardPaste, Store, Trash2 } from "lucide-react";
import { lazy, Suspense, useEffect, useState } from "react";
import {
  APPLI,
  choisirRelais,
  type Course,
  lireCourses,
  lireLien,
  lireRestaurants,
  oublierCourse,
  oublierRestaurant,
  relaisDuSuivi,
  type Restaurant,
  retenirCourse,
  retenirRestaurant,
} from "../appli";
import { dateHeure } from "../format";
import { Page } from "../public/MenuClient";
import { codesSuivis } from "../public/panierClient";
import MesCourses from "../public/MesCourses";
import type { MenuPublic } from "../types";

const MenuClient = lazy(() => import("../public/MenuClient"));
const SuiviClient = lazy(() => import("../public/Suivi"));
const Livreur = lazy(() => import("../public/Livreur"));

/**
 * Applications Android (fiche 0041) : accueil propre à chaque application, puis les pages publiques habituelles
 * (menu, suivi, livreur) servies depuis l'APK et branchées sur le relais du restaurant choisi.
 */
export default function Appli() {
  const chemin = location.pathname;
  useEffect(() => brancherNatif(), []);
  const code = decodeURIComponent(chemin.split("/")[2] ?? "");
  return (
    <Suspense fallback={<p className="aide">Chargement…</p>}>
      {chemin.startsWith("/menu") ? (
        <MenuClient />
      ) : chemin.startsWith("/suivi/") ? (
        <SuiviClient code={code} />
      ) : chemin.startsWith("/livreur/") ? (
        <Livreur code={code} />
      ) : APPLI === "livreur" ? (
        <AccueilLivreur />
      ) : (
        <AccueilClient />
      )}
    </Suspense>
  );
}

/** Ouvre ce qu'un lien désigne : menu ou suivi d'un restaurant, course d'un livreur. */
async function ouvrirLien(lien: string): Promise<string | null> {
  const l = lireLien(lien);
  if (!l) return "Lien non reconnu : collez le lien envoyé par le restaurant (il commence par https://).";
  if (APPLI === "livreur") {
    if (l.page !== "course" || !l.code) return "Ce lien n'est pas un lien de course : demandez le lien « livreur » au restaurant.";
    retenirCourse({ relais: l.relais, code: l.code, nom: await nomRestaurant(l.relais), ajoutee: Date.now() });
    choisirRelais(l.relais);
    location.assign(`/livreur/${encodeURIComponent(l.code)}`);
    return null;
  }
  const nom = await nomRestaurant(l.relais);
  if (!nom) return "Restaurant introuvable à cette adresse. Vérifiez le lien ou la connexion Internet.";
  retenirRestaurant({ nom, relais: l.relais });
  choisirRelais(l.relais);
  location.assign(l.page === "suivi" && l.code ? `/suivi/${encodeURIComponent(l.code)}` : "/menu");
  return null;
}

async function nomRestaurant(relais: string): Promise<string> {
  try {
    const r = await fetch(`${relais}/api/public/menu`);
    return r.ok ? ((await r.json()) as MenuPublic).restaurant : "";
  } catch {
    return "";
  }
}

/** Liens d'ouverture (`youma-client://`, `youma-livreur://`) et bouton Retour d'Android. */
function brancherNatif(): () => void {
  let fini = false;
  const retraits: (() => void)[] = [];
  void import("@capacitor/app")
    .then(async ({ App }) => {
      if (fini) return;
      const lien = await App.addListener("appUrlOpen", (e) => void ouvrirLien(e.url));
      const retour = await App.addListener("backButton", () => {
        if (location.pathname !== "/") location.assign("/");
        else void App.exitApp();
      });
      retraits.push(
        () => void lien.remove(),
        () => void retour.remove(),
      );
      // Application ouverte par un lien alors qu'elle était fermée.
      const depart = await App.getLaunchUrl();
      if (depart?.url && location.pathname === "/") void ouvrirLien(depart.url);
    })
    .catch(() => {
      /* navigateur (essais) : pas de Capacitor */
    });
  return () => {
    fini = true;
    retraits.forEach((r) => r());
  };
}

function AjoutLien({ libelle, aide }: { libelle: string; aide: string }) {
  const [lien, setLien] = useState("");
  const [erreur, setErreur] = useState("");
  const [envoi, setEnvoi] = useState(false);
  const coller = async () => {
    try {
      setLien(await navigator.clipboard.readText());
    } catch {
      setErreur("Collez le lien à la main (appui long dans la case).");
    }
  };
  return (
    <form
      className="carte"
      onSubmit={async (e) => {
        e.preventDefault();
        setEnvoi(true);
        setErreur((await ouvrirLien(lien)) ?? "");
        setEnvoi(false);
      }}
    >
      <label className="champ">
        <span>{libelle}</span>
        <input value={lien} onChange={(e) => setLien(e.target.value)} placeholder="https://…" inputMode="url" autoCapitalize="off" />
      </label>
      <p className="aide">{aide}</p>
      <div className="grille-2">
        <button type="button" onClick={coller}>
          <ClipboardPaste size={18} aria-hidden /> Coller
        </button>
        <button className="principal" disabled={envoi || !lien.trim()}>
          Ouvrir <ArrowRight size={18} aria-hidden />
        </button>
      </div>
      {erreur && <p className="erreur-texte">{erreur}</p>}
    </form>
  );
}

function AccueilClient() {
  const [restaurants, setRestaurants] = useState<Restaurant[]>(lireRestaurants);
  const ouvrir = (r: Restaurant, page = "/menu") => {
    choisirRelais(r.relais);
    location.assign(page);
  };
  return (
    <Page titre="Youma" sousTitre="Commandez à vos restaurants et suivez la livraison">
      {restaurants.length > 0 && (
        <section className="carte">
          <h2>Mes restaurants</h2>
          <ul className="liste-appli">
            {restaurants.map((r) => (
              <RestaurantLigne
                key={r.relais}
                r={r}
                ouvrir={ouvrir}
                retirer={() => {
                  oublierRestaurant(r.relais);
                  setRestaurants(lireRestaurants());
                }}
              />
            ))}
          </ul>
        </section>
      )}
      <AjoutLien libelle="Ajouter un restaurant" aide="Collez le lien de commande en ligne que le restaurant vous a envoyé (WhatsApp, SMS, affiche)." />
    </Page>
  );
}

function RestaurantLigne({ r, ouvrir, retirer }: { r: Restaurant; ouvrir: (r: Restaurant, page?: string) => void; retirer: () => void }) {
  // Commandes encore suivies dans ce restaurant (codes retenus à l'envoi).
  const suivis = codesSuivis().filter((c) => relaisDuSuivi(c) === r.relais);
  return (
    <li>
      <button className="grand" onClick={() => ouvrir(r)}>
        <Store size={20} aria-hidden /> {r.nom}
      </button>
      {suivis.slice(0, 3).map((c) => (
        <button key={c} className="petit" onClick={() => ouvrir(r, `/suivi/${encodeURIComponent(c)}`)}>
          Suivre la commande {c}
        </button>
      ))}
      <button className="petit" onClick={retirer} aria-label={`Retirer ${r.nom}`}>
        <Trash2 size={16} aria-hidden />
      </button>
    </li>
  );
}

function AccueilLivreur() {
  const [courses, setCourses] = useState<Course[]>(lireCourses);
  return (
    <Page titre="Youma Livreur" sousTitre="Votre position est envoyée au client pendant la course, même écran verrouillé">
      <MesCourses />
      {courses.length > 0 && (
        <section className="carte">
          <h2>Courses reçues par lien</h2>
          <ul className="liste-appli">
            {courses.map((c) => (
              <li key={c.code}>
                <button
                  className="grand"
                  onClick={() => {
                    choisirRelais(c.relais);
                    location.assign(`/livreur/${encodeURIComponent(c.code)}`);
                  }}
                >
                  <Bike size={20} aria-hidden /> {c.nom || "Course"} — reçue le {dateHeure(c.ajoutee)}
                </button>
                <button
                  className="petit"
                  aria-label="Retirer la course"
                  onClick={() => {
                    oublierCourse(c.code);
                    setCourses(lireCourses());
                  }}
                >
                  <Trash2 size={16} aria-hidden />
                </button>
              </li>
            ))}
          </ul>
        </section>
      )}
      <AjoutLien
        libelle="Course reçue par lien"
        aide="Sans connexion, collez le lien « livreur » envoyé par le restaurant, ou touchez « Ouvrir dans l'application » sur la page du lien."
      />
    </Page>
  );
}
