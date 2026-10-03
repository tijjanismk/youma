// Géolocalisation des pages publiques (client, livreur), fiche 0040.

/** Le navigateur ne donne la position que sur une page sécurisée (https, ou le poste lui-même). */
export function positionPossible(): boolean {
  return typeof navigator !== "undefined" && !!navigator.geolocation && (typeof window === "undefined" || (window.isSecureContext ?? true));
}

/** Message selon la cause : refus mémorisé par le navigateur, GPS coupé, ou recherche trop longue. */
export function messageErreurPosition(code: number, repli: string): string {
  if (code === 1)
    return `Position refusée par le navigateur. Pour l'autoriser : touchez le cadenas à côté de l'adresse, puis Autorisations, Position. ${repli}`;
  if (code === 3) return `Position trop longue à trouver : réessayez près d'une fenêtre ou dehors. ${repli}`;
  return `Position introuvable : activez la localisation (GPS) du téléphone. ${repli}`;
}
