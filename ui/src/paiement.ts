/** Calculs de l'écran d'encaissement (le serveur revalide tout : RG-CAI-02). */

export type Part = {
  moyen: "especes" | "mobile_money" | "virement" | "carte" | "credit" | "carte_cadeau";
  montant: number;
  compte_id?: string;
  reference?: string;
  numero_payeur?: string;
  client_id?: string;
  par_livreur?: boolean;
  /** RG-SOC-02 : part payée par une société sous contrat (employé dans `reference`). */
  contrat_id?: string;
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
    if (p.moyen === "carte" && !p.compte_id) return "Choisissez le compte bancaire du TPE";
    if (p.moyen === "carte" && !p.reference?.trim()) return "Saisissez le numéro d'autorisation imprimé par le TPE";
    if (p.moyen === "credit" && p.contrat_id && !p.reference?.trim()) return "Indiquez le nom de l'employé de la société";
    if (p.moyen === "credit" && !p.contrat_id && !p.client_id) return "Choisissez le client pour le crédit";
    if (p.moyen === "carte_cadeau" && !p.reference?.trim()) return "Saisissez le code de la carte cadeau";
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

/** Comptes hors caisse pour la paie (fiche 0027) : le coffre d'abord, puis la banque, puis le Mobile Money. */
export function comptesHorsCaisse<C extends { type: string; actif: boolean }>(comptes: C[]): C[] {
  const ordre = ["coffre", "banque", "mobile_money"];
  return comptes.filter((c) => c.actif && ordre.includes(c.type)).sort((a, b) => ordre.indexOf(a.type) - ordre.indexOf(b.type));
}
