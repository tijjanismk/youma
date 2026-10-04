import { Bike, LogOut, MapPin, Phone } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { adresseRelais, APPLI, cheminPublic, choisirRelais, prefixeWeb } from "../appli";
import { Champ } from "../composants/Base";
import { fcfa, lienCarte } from "../format";
import { t } from "../i18n";

/** Course publiée par le poste au relais pour le livreur connecté (RG-LIV-05, fiche 0047). */
export type CourseAssignee = {
  code_livreur: string;
  numero: number;
  statut: string;
  client_nom: string;
  telephone: string | null;
  quartier: string | null;
  repere: string | null;
  lat: number | null;
  lon: number | null;
  reste: number;
};

/** Session du livreur sur un relais (sur le web : le début d'adresse de la page, vide ou `/r/<nom>`). */
export type SessionLivreur = { relais: string; restaurant: string; nom: string; jeton: string };

const CLE_SESSIONS = "youma.livreur.sessions";

export function lireSessions(): SessionLivreur[] {
  try {
    const l = JSON.parse(localStorage.getItem(CLE_SESSIONS) ?? "[]");
    return Array.isArray(l) ? l.filter((s) => s && typeof s.relais === "string" && typeof s.jeton === "string") : [];
  } catch {
    return [];
  }
}

function ecrireSessions(l: SessionLivreur[]) {
  try {
    localStorage.setItem(CLE_SESSIONS, JSON.stringify(l));
  } catch {
    /* stockage indisponible : il faudra se reconnecter */
  }
}

export function retenirSession(s: SessionLivreur) {
  ecrireSessions([s, ...lireSessions().filter((x) => x.relais !== s.relais)]);
}

export function oublierSession(relais: string) {
  ecrireSessions(lireSessions().filter((x) => x.relais !== relais));
}

class RefusRelais extends Error {
  constructor(
    message: string,
    readonly statut: number,
  ) {
    super(message);
  }
}

/** Appel direct au relais d'une session (le livreur peut travailler pour plusieurs restaurants). */
async function appel<T>(relais: string, chemin: string, init?: RequestInit): Promise<T> {
  let r: Response;
  try {
    r = await fetch(`${relais}/api${chemin}`, { ...init, headers: { "Content-Type": "application/json", ...init?.headers } });
  } catch {
    throw new RefusRelais("Pas de connexion Internet, ou restaurant injoignable.", 0);
  }
  const v = await r.json().catch(() => null);
  if (!r.ok) throw new RefusRelais(v?.message ?? `Erreur ${r.status}`, r.status);
  return v as T;
}

/**
 * Courses du livreur connecté par son téléphone et le PIN donné par le restaurant (RG-LIV-05) : plus de lien à coller.
 * Application Youma Livreur (plusieurs restaurants possibles) et page `/livreur` du relais.
 */
export default function MesCourses() {
  const [sessions, setSessions] = useState<SessionLivreur[]>(() => lireSessions().filter((s) => APPLI || s.relais === prefixeWeb()));
  const maj = useCallback(() => setSessions(lireSessions().filter((s) => APPLI || s.relais === prefixeWeb())), []);
  return (
    <>
      {sessions.map((s) => (
        <CoursesDuRestaurant key={s.relais} session={s} deconnecte={maj} />
      ))}
      {(APPLI || sessions.length === 0) && <Connexion connecte={maj} premiere={sessions.length === 0} />}
    </>
  );
}

function Connexion({ connecte, premiere }: { connecte: () => void; premiere: boolean }) {
  const [lien, setLien] = useState("");
  const [telephone, setTelephone] = useState("");
  const [pin, setPin] = useState("");
  const [erreur, setErreur] = useState("");
  const [envoi, setEnvoi] = useState(false);
  const relais = APPLI ? adresseRelais(lien) : prefixeWeb();
  const valide = relais !== null && telephone.replace(/\D/g, "").length >= 8 && /^\d{4,6}$/.test(pin);
  return (
    <form
      className="carte"
      onSubmit={async (e) => {
        e.preventDefault();
        if (relais === null) return;
        setEnvoi(true);
        setErreur("");
        try {
          const r = await appel<{ jeton: string; nom: string; restaurant: string }>(relais, "/public/livreur/connexion", {
            method: "POST",
            body: JSON.stringify({ telephone, pin }),
          });
          retenirSession({ relais, restaurant: r.restaurant, nom: r.nom, jeton: r.jeton });
          setPin("");
          setLien("");
          connecte();
        } catch (x) {
          setErreur(x instanceof Error ? x.message : String(x));
        } finally {
          setEnvoi(false);
        }
      }}
    >
      <h2>{premiere ? "Connexion du livreur" : "Autre restaurant"}</h2>
      <p className="aide">Le restaurant vous a inscrit avec votre numéro et vous a donné un PIN.</p>
      {APPLI && (
        <label className="champ">
          <span>Adresse du restaurant</span>
          <input value={lien} onChange={(e) => setLien(e.target.value)} placeholder="https://…" inputMode="url" autoCapitalize="off" />
        </label>
      )}
      <Champ libelle="Votre téléphone" valeur={telephone} changer={setTelephone} type="tel" />
      <Champ libelle="PIN donné par le restaurant" valeur={pin} changer={setPin} type="password" />
      <button className="principal grand" disabled={!valide || envoi}>
        {envoi ? "Connexion…" : "Se connecter"}
      </button>
      {erreur && <p className="erreur-texte">{erreur}</p>}
    </form>
  );
}

function CoursesDuRestaurant({ session, deconnecte }: { session: SessionLivreur; deconnecte: () => void }) {
  const [courses, setCourses] = useState<CourseAssignee[] | null>(null);
  const [erreur, setErreur] = useState("");
  useEffect(() => {
    let fini = false;
    const charger = async () => {
      try {
        const r = await appel<{ courses: CourseAssignee[] }>(session.relais, "/public/livreur/courses", {
          headers: { Authorization: `Bearer ${session.jeton}` },
        });
        if (fini) return;
        setCourses(r.courses);
        setErreur("");
      } catch (x) {
        if (fini) return;
        // PIN changé, accès retiré ou session expirée : on se reconnecte.
        if (x instanceof RefusRelais && x.statut === 401) {
          oublierSession(session.relais);
          deconnecte();
          return;
        }
        setErreur(x instanceof Error ? x.message : String(x));
      }
    };
    void charger();
    // Les nouvelles courses arrivent sans recharger la page.
    const id = setInterval(() => void charger(), 20_000);
    return () => {
      fini = true;
      clearInterval(id);
    };
  }, [session, deconnecte]);
  const ouvrir = (c: CourseAssignee) => {
    if (APPLI) choisirRelais(session.relais);
    location.assign(cheminPublic(`/livreur/${encodeURIComponent(c.code_livreur)}`));
  };
  return (
    <section className="carte">
      <div className="titre-ligne">
        <h2>
          <Bike size={20} className="icone-texte" aria-hidden /> {session.restaurant || "Mes courses"} — {session.nom}
        </h2>
        <button
          className="petit"
          onClick={() => {
            oublierSession(session.relais);
            deconnecte();
          }}
        >
          <LogOut size={16} aria-hidden /> Se déconnecter
        </button>
      </div>
      {courses === null && !erreur && <p className="aide">Chargement…</p>}
      {courses?.length === 0 && <p className="aide">Aucune course pour le moment : elle apparaît ici dès que le restaurant vous l'assigne.</p>}
      {courses?.map((c) => (
        <article key={c.code_livreur} className="course-livreur">
          <h3>
            n°{c.numero} — {c.client_nom || "Client"} <span className="etiquette">{t(c.statut)}</span>
          </h3>
          <p>
            {[c.quartier, c.repere].filter(Boolean).join(" — ")}
            {c.telephone && (
              <>
                {" "}
                <a href={`tel:${c.telephone}`}>
                  <Phone size={16} className="icone-texte" aria-hidden /> {c.telephone}
                </a>
              </>
            )}
          </p>
          {c.reste > 0 && (
            <p>
              À encaisser : <strong>{fcfa(c.reste)}</strong>
            </p>
          )}
          <div className="boutons-ligne">
            {c.lat !== null && c.lon !== null && (
              <a className="bouton" href={lienCarte(c.lat, c.lon)} target="_blank" rel="noreferrer">
                <MapPin size={16} className="icone-texte" aria-hidden /> Position du client
              </a>
            )}
            <button className="principal" onClick={() => ouvrir(c)}>
              Démarrer la course
            </button>
          </div>
        </article>
      ))}
      {erreur && <p className="erreur-texte">{erreur}</p>}
    </section>
  );
}
