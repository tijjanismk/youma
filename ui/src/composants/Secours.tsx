import { useEffect, useState } from "react";
import { get, post } from "../api";
import type { Session } from "../types";
import { Champ, Modal } from "./Base";

/** Code de secours affiché une seule fois (RG-AUT-07, fiche 0031) : à recopier sur papier. */
export function CodeSecours({ code }: { code: string }) {
  return (
    <div className="code-secours">
      <p className="aide">
        <strong>Code de secours</strong> : il permet de choisir un nouveau mot de passe d'administration en cas d'oubli. Recopiez-le sur papier et rangez-le à
        part. Il ne sera plus affiché.
      </p>
      <p className="code-secours-valeur" aria-label="Code de secours">
        {code}
      </p>
    </div>
  );
}

type Moyen = "code" | "fournisseur";

/** RG-AUT-07 : mot de passe oublié. Le propriétaire (connecté par PIN) prouve son identité par le code de secours ou le fournisseur. */
export function ReinitialisationMotDePasse({ fermer, retour }: { fermer: (s: Session | null) => void; retour: () => void }) {
  const [moyen, setMoyen] = useState<Moyen>("code");
  const [demande, setDemande] = useState("");
  const [preuve, setPreuve] = useState("");
  const [mdp, setMdp] = useState("");
  const [mdp2, setMdp2] = useState("");
  const [erreur, setErreur] = useState("");
  const [fait, setFait] = useState<{ code_secours: string; session: Session } | null>(null);

  useEffect(() => {
    get<{ demande: string }>("/secours")
      .then((r) => setDemande(r.demande))
      .catch((e: Error) => setErreur(e.message));
  }, []);

  const valider = async () => {
    setErreur("");
    if (mdp !== mdp2) return setErreur("Les deux mots de passe sont différents");
    try {
      setFait(await post("/secours/reinitialiser", moyen === "code" ? { code_secours: preuve, nouveau: mdp } : { reponse: preuve, nouveau: mdp }));
    } catch (e) {
      setErreur(e instanceof Error ? e.message : String(e));
    }
  };

  if (fait)
    return (
      <Modal titre="Mot de passe changé" fermer={() => fermer(fait.session)}>
        <p>Votre nouveau mot de passe d'administration est enregistré. L'ancien code de secours ne sert plus : voici le nouveau.</p>
        <CodeSecours code={fait.code_secours} />
        <div className="actions">
          <button className="principal" onClick={() => fermer(fait.session)}>
            J'ai noté le code
          </button>
        </div>
      </Modal>
    );

  return (
    <Modal titre="Mot de passe oublié" fermer={() => fermer(null)}>
      <p className="aide">Réservé au propriétaire. Un gérant ou un autre utilisateur demande au propriétaire de lui redéfinir son mot de passe.</p>
      <div className="choix-moyen" role="group" aria-label="Moyen de vérification">
        <button aria-pressed={moyen === "code"} className={moyen === "code" ? "actif" : ""} onClick={() => (setMoyen("code"), setPreuve(""))}>
          J'ai mon code de secours
        </button>
        <button
          aria-pressed={moyen === "fournisseur"}
          className={moyen === "fournisseur" ? "actif" : ""}
          onClick={() => (setMoyen("fournisseur"), setPreuve(""))}
        >
          Code perdu : appeler le fournisseur
        </button>
      </div>
      {moyen === "code" ? (
        <Champ libelle="Code de secours" valeur={preuve} changer={setPreuve} placeholder="XXXX-XXXX-XXXX-XXXX" autoFocus />
      ) : (
        <>
          <p>
            Envoyez ce code de demande au fournisseur (téléphone, WhatsApp) ; il vous renvoie une réponse à coller ci-dessous.
            <strong className="code-secours-valeur" aria-label="Code de demande">
              {demande}
            </strong>
          </p>
          <Champ libelle="Réponse du fournisseur" valeur={preuve} changer={setPreuve} autoFocus />
        </>
      )}
      <Champ libelle="Nouveau mot de passe (6 caractères au moins)" type="password" valeur={mdp} changer={setMdp} />
      <Champ libelle="Confirmez le nouveau mot de passe" type="password" valeur={mdp2} changer={setMdp2} />
      {erreur && <p className="erreur-texte">{erreur}</p>}
      <div className="actions">
        <button onClick={retour}>← Retour</button>
        <button className="principal" disabled={!preuve.trim() || mdp.length < 6} onClick={valider}>
          Changer le mot de passe
        </button>
      </div>
    </Modal>
  );
}
