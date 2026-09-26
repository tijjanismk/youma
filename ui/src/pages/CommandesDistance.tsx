import { MapPin, Printer } from "lucide-react";
import { useEffect, useState } from "react";
import QRCode from "qrcode";
import { appel, get, post } from "../api";
import { Case, Champ, ChampMontant, Choix, ChoixOuAutre, Modal, TableauDonnees } from "../composants/Base";
import { useApp, useDonnees } from "../contexte";
import { dateHeure, depuisMicro, hhmm, JOURS, joursLibelle, lireHhmm, versMicro } from "../format";
import { t } from "../i18n";
import type { Canaux, Parametres, ZoneRisque } from "../types";

/**
 * Administration des commandes à distance (fiche 0013) : canaux, paiement, vérification du numéro,
 * zones à risque, liste noire, QR des tables. Tout est facultatif : le menu papier reste toujours actif.
 */
export default function CommandesDistance() {
  const { peut } = useApp();
  return (
    <div>
      {peut("parametre.gerer") && <CanauxAdmin />}
      <div className="grille-2">
        <ZonesAdmin />
        <ListeNoireAdmin />
      </div>
      {peut("salle.gerer") && <QrTables />}
    </div>
  );
}

function CanauxAdmin() {
  const { agir, rechargerEtat, etat } = useApp();
  // Paramètres publics de l'état général : l'enregistrement demandera le mot de passe (RG-AUT-06).
  const [p, setP] = useState<Parametres | null>(etat?.parametres ?? null);
  useEffect(() => {
    if (etat?.parametres && !p) setP(etat.parametres);
  }, [etat, p]);
  if (!p) return null;
  const c = p.canaux;
  const maj = (v: Partial<Canaux>) => setP({ ...p, canaux: { ...c, ...v } });
  return (
    <div className="carte">
      <h2>Canaux de commande</h2>
      <p className="aide">Le menu papier (commande prise par le serveur) est toujours possible. Les autres canaux s'activent selon le restaurant.</p>
      <div className="grille-2">
        <div>
          <Case libelle="Commandes par téléphone (saisies par le personnel)" valeur={c.telephone} changer={(v) => maj({ telephone: v })} />
          <Case libelle="QR code sur les tables (le client commande depuis son téléphone)" valeur={c.qr_table} changer={(v) => maj({ qr_table: v })} />
          <Case libelle="Commandes en ligne (livraison, à emporter)" valeur={c.en_ligne} changer={(v) => maj({ en_ligne: v })} />
          <Choix
            libelle="Vérification du numéro du client"
            valeur={c.verification_numero}
            changer={(v) => maj({ verification_numero: v })}
            options={[
              { valeur: "rappel", libelle: "Rappel par la caisse avant d'accepter" },
              { valeur: "sms", libelle: "Code SMS Orange Mali (serveur relais)" },
            ]}
          />
        </div>
        <div>
          <Case libelle="Paiement Mobile Money d'avance" valeur={c.paiement_avance} changer={(v) => maj({ paiement_avance: v })} />
          <Case libelle="Paiement à la livraison" valeur={c.paiement_a_la_livraison} changer={(v) => maj({ paiement_a_la_livraison: v })} />
          <Case libelle="Nouveau client : paiement d'avance obligatoire" valeur={c.avance_nouveau_client} changer={(v) => maj({ avance_nouveau_client: v })} />
          <ChampMontant
            libelle="Au-delà de ce montant, paiement d'avance obligatoire (0 = pas de plafond)"
            valeur={c.plafond_paiement_livraison}
            changer={(v) => maj({ plafond_paiement_livraison: v })}
          />
        </div>
      </div>
      <details>
        <summary>Serveur relais Internet (facultatif)</summary>
        <p className="aide">Sans relais, le menu QR et les commandes fonctionnent sur le Wi-Fi du restaurant. Le relais publie le menu sur Internet.</p>
        <Champ libelle="Adresse du relais" valeur={c.relais_url} changer={(v) => maj({ relais_url: v })} placeholder="https://commande.exemple.ml" />
        <Champ libelle="Clé du relais" valeur={c.relais_cle} changer={(v) => maj({ relais_cle: v })} type="password" />
        <p className="aide">
          SMS : Orange Mali, configuré sur le relais (identifiants Orange). Sans contrat Orange, le relais simule l'envoi et affiche le code au client.
        </p>
        <EtatRelais />
      </details>
      <button className="principal" onClick={() => agir((pin) => appel("/parametres", { methode: "PUT", corps: p, pin }), "Canaux enregistrés").then(rechargerEtat)}>
        Enregistrer
      </button>
    </div>
  );
}

type EtatRelaisT = { actif: boolean; dernier_succes: number | null; derniere_erreur: string | null; commandes_recues: number; sms: string | null };

function EtatRelais() {
  const { donnees } = useDonnees(() => get<EtatRelaisT>("/relais/etat").catch(() => null), []);
  if (!donnees?.actif) return <p className="aide">Relais : non configuré (ou commande en ligne désactivée).</p>;
  return (
    <p className={donnees.derniere_erreur ? "attention-texte" : "aide"}>
      Relais : {donnees.dernier_succes ? `dernier contact ${dateHeure(donnees.dernier_succes)}` : "jamais joint"}
      {donnees.derniere_erreur && ` — erreur : ${donnees.derniere_erreur}`} · {donnees.commandes_recues} commande(s) reçue(s) depuis le démarrage
      {donnees.sms && ` · SMS : ${donnees.sms === "orange_mali" ? "Orange Mali" : "simulation (pas de SMS réel)"}`}
    </p>
  );
}

const ZONE_VIDE: ZoneRisque = {
  id: "",
  nom: "",
  quartier: "",
  lat: null,
  lon: null,
  rayon_m: null,
  debut_min: 21 * 60,
  fin_min: 6 * 60,
  jours: 127,
  action: "bloquer",
  message: "",
  actif: true,
};

function ZonesAdmin() {
  const { agir, etat } = useApp();
  const { donnees, recharger } = useDonnees(() => get<ZoneRisque[]>("/zones-risque"), []);
  const [z, setZ] = useState<ZoneRisque | null>(null);
  const quartiers = etat?.parametres.quartiers ?? [];
  return (
    <div className="carte">
      <h2>Zones à risque</h2>
      <p className="aide">
        Une livraison vers tel quartier (ou autour d'un point GPS), à telle heure, peut être bloquée, payée d'avance ou soumise à l'accord d'un responsable.
      </p>
      <TableauDonnees
        colonnes={["Zone", "Lieu", "Horaire", "Action", ""]}
        lignes={(donnees ?? []).map((zr) => [
          `${zr.nom}${zr.actif ? "" : " (inactive)"}`,
          [zr.quartier, zr.rayon_m ? `${zr.rayon_m} m autour d'un point` : null].filter(Boolean).join(" + "),
          `${hhmm(zr.debut_min)} → ${hhmm(zr.fin_min)} · ${joursLibelle(zr.jours)}`,
          t(zr.action),
          <button className="petit" onClick={() => setZ(zr)}>
            Modifier
          </button>,
        ])}
      />
      <button className="principal" onClick={() => setZ({ ...ZONE_VIDE, quartier: quartiers[0]?.nom ?? "" })}>
        + Zone à risque
      </button>
      {z && (
        <FormZone
          z={z}
          quartiers={quartiers.map((q) => q.nom)}
          fermer={() => setZ(null)}
          enregistrer={(v) =>
            agir((pin) => post("/zones-risque", v, pin), "Zone enregistrée").then((r) => {
              if (r !== undefined) {
                setZ(null);
                recharger();
              }
            })
          }
        />
      )}
    </div>
  );
}

function FormZone({ z: initiale, quartiers, fermer, enregistrer }: { z: ZoneRisque; quartiers: string[]; fermer: () => void; enregistrer: (z: ZoneRisque) => void }) {
  const [z, setZ] = useState(initiale);
  const [debut, setDebut] = useState(hhmm(initiale.debut_min));
  const [fin, setFin] = useState(hhmm(initiale.fin_min));
  const [gps, setGps] = useState(initiale.lat !== null);
  const [lat, setLat] = useState(initiale.lat !== null ? String(depuisMicro(initiale.lat)) : "");
  const [lon, setLon] = useState(initiale.lon !== null ? String(depuisMicro(initiale.lon)) : "");
  const d = lireHhmm(debut);
  const f = lireHhmm(fin);
  const pointValide = !gps || (Number.isFinite(parseFloat(lat)) && Number.isFinite(parseFloat(lon)) && (z.rayon_m ?? 0) > 0);
  const valide = z.nom.trim() && (z.quartier?.trim() || gps) && d !== null && f !== null && d !== f && pointValide && z.jours > 0;
  const ici = () =>
    navigator.geolocation?.getCurrentPosition((p) => {
      setLat(p.coords.latitude.toFixed(6));
      setLon(p.coords.longitude.toFixed(6));
    });
  return (
    <Modal titre="Zone à risque" fermer={fermer}>
      <Champ libelle="Nom" valeur={z.nom} changer={(v) => setZ({ ...z, nom: v })} placeholder="Kalaban la nuit" obligatoire autoFocus />
      <ChoixOuAutre libelle="Quartier" valeur={z.quartier ?? ""} changer={(v) => setZ({ ...z, quartier: v })} groupes={[{ nom: "", options: quartiers }]} />
      <Case libelle="Cercle autour d'un point GPS" valeur={gps} changer={setGps} />
      {gps && (
        <div className="grille-2">
          <Champ libelle="Latitude" valeur={lat} changer={setLat} placeholder="12.6392" />
          <Champ libelle="Longitude" valeur={lon} changer={setLon} placeholder="-8.0029" />
          <label className="champ">
            <span>Rayon (mètres)</span>
            <input type="number" value={z.rayon_m ?? ""} onChange={(e) => setZ({ ...z, rayon_m: Number(e.target.value) || null })} aria-label="Rayon (mètres)" />
          </label>
          <button type="button" onClick={ici}>
            <MapPin size={20} aria-hidden /> Ma position actuelle
          </button>
        </div>
      )}
      <div className="grille-2">
        <Champ libelle="De (heure)" valeur={debut} changer={setDebut} placeholder="21:00" />
        <Champ libelle="À (heure)" valeur={fin} changer={setFin} placeholder="06:00" />
      </div>
      <div className="suggestions" role="group" aria-label="Jours">
        {JOURS.map((j, i) => (
          <button key={j} type="button" className={z.jours & (1 << i) ? "actif" : ""} aria-pressed={!!(z.jours & (1 << i))} onClick={() => setZ({ ...z, jours: z.jours ^ (1 << i) })}>
            {j}
          </button>
        ))}
      </div>
      <Choix
        libelle="Action"
        valeur={z.action}
        changer={(v) => setZ({ ...z, action: v })}
        options={[
          { valeur: "bloquer", libelle: "Bloquer la commande" },
          { valeur: "paiement_avance", libelle: "Paiement Mobile Money d'avance obligatoire" },
          { valeur: "validation_manuelle", libelle: "Accord d'un responsable" },
        ]}
      />
      <Champ libelle="Message au client (facultatif)" valeur={z.message} changer={(v) => setZ({ ...z, message: v })} />
      <Case libelle="Active" valeur={z.actif} changer={(v) => setZ({ ...z, actif: v })} />
      <div className="actions">
        <button onClick={fermer}>Annuler</button>
        <button
          className="principal"
          disabled={!valide}
          onClick={() =>
            enregistrer({
              ...z,
              quartier: z.quartier?.trim() || null,
              debut_min: d!,
              fin_min: f!,
              lat: gps ? versMicro(parseFloat(lat)) : null,
              lon: gps ? versMicro(parseFloat(lon)) : null,
              rayon_m: gps ? z.rayon_m : null,
            })
          }
        >
          Enregistrer
        </button>
      </div>
    </Modal>
  );
}

function ListeNoireAdmin() {
  const { agir } = useApp();
  const { donnees, recharger } = useDonnees(() => get<{ telephone: string; motif: string; cree_le: number }[]>("/numeros-bloques"), []);
  const [tel, setTel] = useState("");
  const [motif, setMotif] = useState("");
  return (
    <div className="carte">
      <h2>Numéros bloqués</h2>
      <p className="aide">Faux clients, commandes jamais récupérées : ces numéros ne peuvent plus commander en ligne.</p>
      <TableauDonnees
        colonnes={["Numéro", "Motif", "Depuis", ""]}
        lignes={(donnees ?? []).map((n) => [
          n.telephone,
          n.motif,
          dateHeure(n.cree_le),
          <button className="petit" onClick={() => agir((pin) => post("/numeros-bloques/debloquer", { telephone: n.telephone }, pin), "Numéro débloqué").then(recharger)}>
            Débloquer
          </button>,
        ])}
      />
      <Champ libelle="Numéro à bloquer" valeur={tel} changer={setTel} type="tel" />
      <Champ libelle="Motif du blocage" valeur={motif} changer={setMotif} />
      <button
        className="attention"
        disabled={tel.replace(/\D/g, "").length < 8 || !motif.trim()}
        onClick={() =>
          agir((pin) => post("/numeros-bloques", { telephone: tel, motif }, pin), "Numéro bloqué").then((r) => {
            if (r !== undefined) {
              setTel("");
              setMotif("");
              recharger();
            }
          })
        }
      >
        Bloquer ce numéro
      </button>
    </div>
  );
}

type CodeQr = { table_id: string; nom: string; code: string | null };

/** QR à imprimer et coller sur chaque table : il ouvre le menu de cette table sur le téléphone du client. */
function QrTables() {
  const { agir } = useApp();
  const { donnees, recharger } = useDonnees(() => get<CodeQr[]>("/tables/codes-qr"), ["table"]);
  const { donnees: reseau } = useDonnees(() => get<{ actif: boolean; adresses: string[] }>("/reseau").catch(() => null), []);
  const [images, setImages] = useState<Record<string, string>>({});
  const base = reseau?.adresses[0] ?? `${location.origin}/`;
  useEffect(() => {
    let actif = true;
    Promise.all(
      (donnees ?? [])
        .filter((d) => d.code)
        .map(async (d) => [d.table_id, await QRCode.toDataURL(`${base}menu?table=${d.code}`, { width: 220, margin: 1 }).catch(() => "")] as const),
    ).then((l) => actif && setImages(Object.fromEntries(l)));
    return () => {
      actif = false;
    };
  }, [donnees, base]);
  const generer = (regenerer: boolean) => agir((pin) => post<number>("/tables/codes-qr", { regenerer }, pin), regenerer ? "Nouveaux codes : réimprimez les QR" : "Codes créés").then(recharger);
  const sansCode = (donnees ?? []).filter((d) => !d.code).length;
  return (
    <div className="carte">
      <div className="titre-ligne">
        <h2>QR codes des tables</h2>
        <span className="boutons-ligne">
          {sansCode > 0 && (
            <button className="principal" onClick={() => generer(false)}>
              Créer les codes ({sansCode})
            </button>
          )}
          <button
            onClick={() => {
              document.body.classList.add("impression-qr");
              window.print();
              document.body.classList.remove("impression-qr");
            }}
          >
            <Printer size={20} aria-hidden /> Imprimer
          </button>
          <button
            className="attention petit"
            onClick={() => confirm("Les anciens QR ne fonctionneront plus. Continuer ?") && generer(true)}
          >
            Changer tous les codes
          </button>
        </span>
      </div>
      {!reseau?.actif && <p className="attention-texte">Le poste central est en mode mono-poste : démarrez-le en mode réseau pour que les clients accèdent au menu par le Wi-Fi.</p>}
      <div className="qr-tables">
        {(donnees ?? [])
          .filter((d) => d.code)
          .map((d) => (
            <figure key={d.table_id} className="qr-table">
              {images[d.table_id] && <img src={images[d.table_id]} alt={`QR ${d.nom}`} />}
              <figcaption>
                <strong>{d.nom}</strong>
                <br />
                Scannez pour commander · code {d.code}
              </figcaption>
            </figure>
          ))}
      </div>
    </div>
  );
}
