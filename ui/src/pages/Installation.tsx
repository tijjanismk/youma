import { useState } from "react";
import { post } from "../api";
import { Champ } from "../composants/Base";
import { useApp } from "../contexte";

/**
 * Première installation : nom du restaurant et propriétaire.
 * La suite (zones, tables, catégories, produits, imprimantes) se fait dans Administration,
 * guidée par la liste « À configurer » de l'accueil.
 */
export default function Installation() {
  const { rechargerEtat, notifier } = useApp();
  const [restaurant, setRestaurant] = useState("");
  const [nom, setNom] = useState("");
  const [pin, setPin] = useState("");
  const [pin2, setPin2] = useState("");
  const [mdp, setMdp] = useState("");
  const [mdp2, setMdp2] = useState("");
  const ok = restaurant.trim() && nom.trim() && /^\d{4,6}$/.test(pin) && pin === pin2 && mdp.length >= 6 && mdp === mdp2;

  const installer = async () => {
    try {
      await post("/installation", { restaurant, nom, pin, mot_de_passe: mdp });
      notifier("Installation terminée. Connectez-vous.", "succes");
      await rechargerEtat();
    } catch (e) {
      notifier(e instanceof Error ? e.message : String(e), "erreur");
    }
  };

  return (
    <div className="plein-ecran">
      <div className="carte etroite">
        <h1>Bienvenue dans Youma</h1>
        <p className="aide">Première configuration du poste. Tout fonctionne sans Internet.</p>
        <Champ libelle="Nom du restaurant" valeur={restaurant} changer={setRestaurant} obligatoire autoFocus />
        <Champ libelle="Votre nom (propriétaire)" valeur={nom} changer={setNom} obligatoire />
        <Champ libelle="Votre code PIN (4 à 6 chiffres)" valeur={pin} changer={setPin} type="password" obligatoire />
        <Champ libelle="Confirmez le code PIN" valeur={pin2} changer={setPin2} type="password" obligatoire />
        {pin2 && pin !== pin2 && <p className="erreur-texte">Les deux codes ne sont pas identiques.</p>}
        <p className="aide">Le PIN sert au service. Le mot de passe protège l'administration (utilisateurs, paramètres, sauvegardes, licence).</p>
        <Champ libelle="Mot de passe d'administration (6 caractères au moins)" valeur={mdp} changer={setMdp} type="password" obligatoire />
        <Champ libelle="Confirmez le mot de passe" valeur={mdp2} changer={setMdp2} type="password" obligatoire />
        {mdp2 && mdp !== mdp2 && <p className="erreur-texte">Les deux mots de passe ne sont pas identiques.</p>}
        <button className="principal grand" disabled={!ok} onClick={installer}>
          Installer
        </button>
      </div>
    </div>
  );
}
