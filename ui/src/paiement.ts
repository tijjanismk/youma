/** Calculs de l'écran d'encaissement (le serveur revalide tout : RG-CAI-02). */

export type Part = {
  moyen: "especes" | "mobile_money" | "virement" | "carte" | "credit";
  montant: number;
  compte_id?: string;
  reference?: string;
  numero_payeur?: string;
  client_id?: string;
  par_livreur?: boolean;
};

export function sommeParts(parts: Part[]): number {
  return parts.reduce((s, p) => s + p.montant, 0);
}

export function rendu(parts: Part[], especesRecues: number): number {
  const especes = parts.filter((p) => p.moyen === "especes").reduce((s, p) => s + p.montant, 0);
  return Math.max(0, especesRecues - especes);
}

/** Message bloquant, ou null si le paiement peut partir. */
export function verifierPaiement(parts: Part[], aPayer: number, especesRecues: number, referenceObligatoire: boolean): string | null {
  if (!parts.length) return "Choisissez un moyen de paiement";
  if (parts.some((p) => p.montant <= 0)) return "Chaque montant doit être positif";
  const total = sommeParts(parts);
  if (total > aPayer) return `Le total (${total}) dépasse le montant à payer (${aPayer})`;
  const especes = parts.filter((p) => p.moyen === "especes").reduce((s, p) => s + p.montant, 0);
  if (especes > 0 && especesRecues > 0 && especesRecues < especes) return "Espèces reçues insuffisantes";
  for (const p of parts) {
    if (p.moyen === "mobile_money" && !p.compte_id) return "Choisissez l'opérateur Mobile Money";
    if (p.moyen === "mobile_money" && referenceObligatoire && !p.reference?.trim()) return "Saisissez la référence de la transaction Mobile Money";
    if (p.moyen === "credit" && !p.client_id) return "Choisissez le client pour le crédit";
  }
  return null;
}

/** Billets proposés pour « espèces reçues » : montant exact puis coupures supérieures. */
export function billetsProposes(montant: number): number[] {
  const coupures = [500, 1000, 2000, 5000, 10000];
  const r = new Set<number>([montant]);
  for (const c of coupures) {
    const arrondi = Math.ceil(montant / c) * c;
    if (arrondi > montant) r.add(arrondi);
  }
  return [...r].sort((a, b) => a - b).slice(0, 5);
}
