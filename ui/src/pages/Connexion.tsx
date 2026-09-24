import { useEffect, useState } from "react";
import { get, post } from "../api";
import { PinPad } from "../composants/Base";
import { useApp } from "../contexte";
import type { Session } from "../types";

type U = { id: string; nom: string; role: string };

/** Changement d'utilisateur rapide : je touche mon nom, je tape mon PIN. */
export default function Connexion() {
  const { connecter, notifier, etat } = useApp();
  const [utilisateurs, setUtilisateurs] = useState<U[]>([]);
  const [choisi, setChoisi] = useState<U | null>(null);

  useEffect(() => {
    get<U[]>("/connexion/utilisateurs")
      .then(setUtilisateurs)
      .catch((e) => notifier(e.message, "erreur"));
  }, [notifier]);

  const valider = async (pin: string) => {
    if (!choisi) return;
    try {
      connecter(await post<Session>("/connexion", { utilisateur_id: choisi.id, pin }));
    } catch (e) {
      notifier(e instanceof Error ? e.message : String(e), "erreur");
    }
  };

  return (
    <div className="connexion">
      <h1>{etat?.restaurant ?? "Youma"}</h1>
      {!choisi ? (
        <>
          <p className="aide">Qui êtes-vous ?</p>
          <div className="grille-utilisateurs">
            {utilisateurs.map((u) => (
              <button key={u.id} className="tuile" onClick={() => setChoisi(u)}>
                <span className="avatar" aria-hidden>
                  {u.nom.slice(0, 1)}
                </span>
                <strong>{u.nom}</strong>
                <small>{u.role}</small>
              </button>
            ))}
          </div>
        </>
      ) : (
        <div className="connexion-pin">
          <p>
            <strong>{choisi.nom}</strong> — tapez votre code
          </p>
          <PinPad valider={valider} libelle="Entrer" />
          <button className="lien" onClick={() => setChoisi(null)}>
            ← Changer de personne
          </button>
        </div>
      )}
    </div>
  );
}
