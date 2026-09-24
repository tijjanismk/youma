import { createContext, ReactNode, useCallback, useContext, useEffect, useRef, useState } from "react";
import { Modal, PinPad } from "./composants/Base";
import { definirJeton, ErreurApi, estEnLigne, get, jeton, post, surDeconnexion, surReseau } from "./api";
import type { EtatGeneral, Session } from "./types";

type Evenement = { type: string; id: string | null };
type Abonne = (e: Evenement) => void;

type Contexte = {
  etat: EtatGeneral | null;
  session: Session | null;
  enLigne: boolean;
  peut: (permission: string) => boolean;
  connecter: (s: Session) => void;
  deconnecter: () => Promise<void>;
  rechargerEtat: () => Promise<void>;
  abonner: (f: Abonne) => () => void;
  notifier: (message: string, genre?: "info" | "erreur" | "succes") => void;
  /** Exécute une action ; si un responsable doit autoriser, demande son PIN puis réessaie. */
  agir: <T>(action: (pin?: string) => Promise<T>, succes?: string) => Promise<T | undefined>;
};

const Ctx = createContext<Contexte | null>(null);

export function useApp(): Contexte {
  const c = useContext(Ctx);
  if (!c) throw new Error("useApp hors du fournisseur");
  return c;
}

type Notification = { id: number; message: string; genre: string };
type DemandePin = { message: string; resoudre: (pin: string | null) => void };

export function Fournisseur({ children }: { children: ReactNode }) {
  const [etat, setEtat] = useState<EtatGeneral | null>(null);
  const [session, setSession] = useState<Session | null>(null);
  const [enLigne, setEnLigne] = useState(estEnLigne());
  const [notifications, setNotifications] = useState<Notification[]>([]);
  const [demandePin, setDemandePin] = useState<DemandePin | null>(null);
  const abonnes = useRef(new Set<Abonne>());

  const rechargerEtat = useCallback(async () => {
    try {
      setEtat(await get<EtatGeneral>("/etat"));
    } catch {
      /* hors ligne : le bandeau l'indique */
    }
  }, []);

  useEffect(() => {
    rechargerEtat();
    if (jeton()) {
      get<Session>("/session")
        .then(setSession)
        .catch(() => definirJeton(null));
    }
    const a = surReseau((v) => {
      setEnLigne(v);
      if (v) rechargerEtat();
    });
    const b = surDeconnexion(() => {
      definirJeton(null);
      setSession(null);
    });
    return () => {
      a();
      b();
    };
  }, [rechargerEtat]);

  // WebSocket : un seul par onglet, reconnexion automatique (scénario 13).
  useEffect(() => {
    if (!session) return;
    let ws: WebSocket | null = null;
    let arret = false;
    let delai = 1000;
    let minuterie: ReturnType<typeof setTimeout>;
    const ouvrir = () => {
      const proto = location.protocol === "https:" ? "wss" : "ws";
      ws = new WebSocket(`${proto}://${location.host}/api/ws?jeton=${encodeURIComponent(session.jeton)}`);
      ws.onopen = () => {
        delai = 1000;
        setEnLigne(true);
      };
      ws.onmessage = (m) => {
        try {
          const e = JSON.parse(m.data) as Evenement;
          abonnes.current.forEach((f) => f(e));
          if (e.type === "journee") rechargerEtat();
        } catch {
          /* message ignoré */
        }
      };
      ws.onclose = () => {
        if (arret) return;
        setEnLigne(false);
        minuterie = setTimeout(ouvrir, delai);
        delai = Math.min(delai * 2, 15_000);
      };
    };
    ouvrir();
    return () => {
      arret = true;
      clearTimeout(minuterie);
      ws?.close();
    };
  }, [session, rechargerEtat]);

  const notifier = useCallback((message: string, genre: "info" | "erreur" | "succes" = "info") => {
    const id = Date.now() + Math.random();
    setNotifications((n) => [...n, { id, message, genre }]);
    setTimeout(() => setNotifications((n) => n.filter((x) => x.id !== id)), genre === "erreur" ? 7000 : 3500);
  }, []);

  const demanderPin = (message: string) => new Promise<string | null>((resoudre) => setDemandePin({ message, resoudre }));

  const agir = useCallback(
    async <T,>(action: (pin?: string) => Promise<T>, succes?: string): Promise<T | undefined> => {
      let pin: string | undefined;
      for (let essai = 0; essai < 3; essai++) {
        try {
          const r = await action(pin);
          if (succes) notifier(succes, "succes");
          return r;
        } catch (e) {
          if (e instanceof ErreurApi && (e.autorisationRequise || (pin && e.code === "PIN_INCORRECT"))) {
            const msg = e.code === "PIN_INCORRECT" ? "PIN incorrect. Réessayez." : `Autorisation d'un responsable : ${e.message}`;
            const p = await demanderPin(msg);
            if (!p) return undefined;
            pin = p;
            continue;
          }
          notifier(e instanceof Error ? e.message : String(e), "erreur");
          return undefined;
        }
      }
      return undefined;
    },
    [notifier],
  );

  const valeur: Contexte = {
    etat,
    session,
    enLigne,
    peut: (p) => !!session?.permissions.includes(p),
    connecter: (s) => {
      definirJeton(s.jeton);
      setSession(s);
      rechargerEtat();
    },
    deconnecter: async () => {
      try {
        await post("/deconnexion");
      } catch {
        /* déjà déconnecté */
      }
      definirJeton(null);
      setSession(null);
    },
    rechargerEtat,
    abonner: (f) => {
      abonnes.current.add(f);
      return () => abonnes.current.delete(f);
    },
    notifier,
    agir,
  };

  return (
    <Ctx.Provider value={valeur}>
      {children}
      <div className="notifications" role="status" aria-live="polite">
        {notifications.map((n) => (
          <div key={n.id} className={`notification ${n.genre}`}>
            {n.message}
          </div>
        ))}
      </div>
      {demandePin && (
        <PinResponsable
          message={demandePin.message}
          fermer={(p) => {
            demandePin.resoudre(p);
            setDemandePin(null);
          }}
        />
      )}
    </Ctx.Provider>
  );
}


function PinResponsable({ message, fermer }: { message: string; fermer: (pin: string | null) => void }) {
  return (
    <Modal titre="PIN du responsable" fermer={() => fermer(null)}>
      <p className="aide">{message}</p>
      <PinPad valider={(p) => fermer(p)} libelle="Autoriser" />
    </Modal>
  );
}

/** Charge des données et les recharge quand un événement temps réel pertinent arrive. */
export function useDonnees<T>(charger: () => Promise<T>, evenements: string[] = [], deps: unknown[] = []) {
  const { abonner } = useApp();
  const [donnees, setDonnees] = useState<T | null>(null);
  const [erreur, setErreur] = useState<string | null>(null);
  const chargerRef = useRef(charger);
  chargerRef.current = charger;
  const recharger = useCallback(async () => {
    try {
      setDonnees(await chargerRef.current());
      setErreur(null);
    } catch (e) {
      setErreur(e instanceof Error ? e.message : String(e));
    }
  }, []);
  useEffect(() => {
    recharger();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, deps);
  const cle = evenements.join(",");
  useEffect(() => {
    if (!cle) return;
    const types = cle.split(",");
    return abonner((e) => {
      if (types.includes(e.type) || e.type === "resynchroniser") recharger();
    });
  }, [cle, abonner, recharger]);
  return { donnees, erreur, recharger, setDonnees };
}
