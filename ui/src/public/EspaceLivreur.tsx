import MesCourses from "./MesCourses";
import { Page } from "./MenuClient";

/** Page `/livreur` du relais : le livreur se connecte (téléphone + PIN du restaurant) et voit ses courses (fiche 0047). */
export default function EspaceLivreur() {
  return (
    <Page titre="Mes courses" sousTitre="Connectez-vous avec le numéro et le PIN donnés au restaurant">
      <MesCourses />
    </Page>
  );
}
