import { useEffect, useState } from "react";
import { appel, get, post } from "../api";
import { Case, Champ, TableauDonnees } from "../composants/Base";
import { useApp, useDonnees } from "../contexte";
import { dateHeure } from "../format";
import type { CloudParams, Parametres } from "../types";

type EtatCloud = { actif: boolean; dernier_succes: number | null; derniere_erreur: string | null; derniere_sauvegarde: string | null; sms: string | null };
type Distante = { id: string; nom: string; taille: number; recu_le: number };

/**
 * Cloud facultatif (fiche 0018) : résumé de fin de journée par SMS, sauvegardes chiffrées hors du restaurant,
 * consultation à distance par le propriétaire (tous ses restaurants). Le restaurant n'en dépend jamais pour vendre.
 */
export default function CloudAdmin() {
  const { agir, rechargerEtat, etat } = useApp();
  const [p, setP] = useState<Parametres | null>(etat?.parametres ?? null);
  useEffect(() => {
    if (etat?.parametres && !p) setP(etat.parametres);
  }, [etat, p]);
  const { donnees: suivi, recharger } = useDonnees(() => get<EtatCloud>("/cloud/etat"), []);
  const [mdp, setMdp] = useState("");
  const [distantes, setDistantes] = useState<Distante[] | null>(null);
  if (!p) return null;
  const c = p.cloud;
  const maj = (v: Partial<CloudParams>) => setP({ ...p, cloud: { ...c, ...v } });
  const phraseCourte = c.phrase_chiffrement !== "********" && c.phrase_chiffrement.length > 0 && c.phrase_chiffrement.length < 12;
  return (
    <div className="grille-2">
      <div className="carte">
        <h2>Cloud (facultatif)</h2>
        <p className="aide">Le restaurant vend et encaisse sans Internet. Le cloud reçoit les résumés et les sauvegardes dès qu'une connexion est disponible.</p>
        <Champ libelle="Adresse du serveur" valeur={c.url} changer={(v) => maj({ url: v })} placeholder="https://commande.exemple.ml" />
        <Champ libelle="Clé du restaurant" valeur={c.cle} changer={(v) => maj({ cle: v })} type="password" />
        <Champ libelle="Téléphone du propriétaire" valeur={c.telephone_proprietaire} changer={(v) => maj({ telephone_proprietaire: v })} type="tel" />
        <Case libelle="Résumé de la journée par SMS à la clôture" valeur={c.sms_resume} changer={(v) => maj({ sms_resume: v })} />
        <Champ libelle="Phrase de chiffrement des sauvegardes" valeur={c.phrase_chiffrement} changer={(v) => maj({ phrase_chiffrement: v })} type="password" />
        {phraseCourte && <p className="erreur-texte">12 caractères au moins.</p>}
        <p className="attention-texte">
          Notez cette phrase sur papier et gardez-la hors du restaurant : sans elle, personne (ni le cloud, ni le fournisseur) ne peut relire les
          sauvegardes.
        </p>
        <button
          className="principal"
          disabled={phraseCourte}
          onClick={() => agir((pin) => appel("/parametres", { methode: "PUT", corps: p, pin }), "Cloud enregistré").then(rechargerEtat)}
        >
          Enregistrer
        </button>
      </div>
      <div className="carte">
        <h2>Espace propriétaire</h2>
        <p className="aide">
          Consultez vos restaurants depuis n'importe où ({c.url ? `${c.url.replace(/\/$/, "")}/proprietaire` : "adresse du serveur/proprietaire"}) avec votre
          numéro et ce mot de passe — le même sur tous vos restaurants pour les voir ensemble.
        </p>
        <Champ libelle="Nouveau mot de passe distant" valeur={mdp} changer={setMdp} type="password" />
        <button
          disabled={mdp.length < 8}
          onClick={() => agir((pin) => post("/cloud/mot-de-passe", { mot_de_passe: mdp }, pin), "Mot de passe distant enregistré").then(() => setMdp(""))}
        >
          Enregistrer le mot de passe
        </button>
        <h2>État</h2>
        {!suivi?.actif ? (
          <p className="aide">Cloud non configuré.</p>
        ) : (
          <p className={suivi.derniere_erreur ? "attention-texte" : "aide"}>
            {suivi.dernier_succes ? `Dernier envoi ${dateHeure(suivi.dernier_succes)}` : "Jamais joint"}
            {suivi.derniere_erreur && ` — erreur : ${suivi.derniere_erreur}`}
            {suivi.derniere_sauvegarde && ` · dernière sauvegarde envoyée : ${suivi.derniere_sauvegarde}`}
            {suivi.sms && ` · SMS : ${suivi.sms === "orange_mali" ? "Orange Mali" : "simulation"}`}
          </p>
        )}
        <div className="actions">
          <button onClick={() => agir((pin) => post<EtatCloud>("/cloud/synchroniser", {}, pin), "Envoyé au cloud").then(recharger)}>Synchroniser maintenant</button>
          <button onClick={() => agir(() => get<Distante[]>("/cloud/sauvegardes")).then((d) => d && setDistantes(d))}>Sauvegardes dans le cloud</button>
        </div>
        {distantes && (
          <TableauDonnees
            colonnes={["Reçue le", "Fichier", "Taille", ""]}
            lignes={distantes.map((d) => [
              dateHeure(d.recu_le),
              d.nom,
              `${Math.round(d.taille / 1024)} Ko`,
              <button
                className="petit attention"
                onClick={async () => {
                  const r = await agir((pin) => post<{ chemin: string }>(`/cloud/sauvegardes/${d.id}/recuperer`, {}, pin), "Sauvegarde récupérée et déchiffrée");
                  if (r && confirm("Restaurer maintenant cette sauvegarde ? Les données actuelles seront remplacées (une sauvegarde est faite avant).")) {
                    await agir((pin) => post("/sauvegardes/restaurer", { chemin: r.chemin }, pin), "Restauration terminée");
                    location.reload();
                  }
                }}
              >
                Récupérer
              </button>,
            ])}
          />
        )}
      </div>
    </div>
  );
}
