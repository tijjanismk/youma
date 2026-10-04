import { MapPin } from "lucide-react";
import { lienCarte } from "../format";

/** Lien vers la position partagée par le client d'une livraison (fiche 0046) : vue par le restaurant et le livreur. */
export default function PositionClient({ lat, lon }: { lat: number | null; lon: number | null }) {
  if (lat === null || lon === null) return null;
  return (
    <a className="bouton petit" href={lienCarte(lat, lon)} target="_blank" rel="noreferrer">
      <MapPin size={16} className="icone-texte" aria-hidden /> Position du client
    </a>
  );
}
