/**
 * Villes et quartiers proposés dans les listes de choix. Ce ne sont que des suggestions : « Autre… » permet
 * toujours une saisie libre (autre ville, quartier récent). Pour ajouter une ville, compléter QUARTIERS_PAR_VILLE.
 * [HYPOTHÈSE] Découpage de Bamako par commune à faire valider sur le terrain (fiche 0021).
 */

export type GroupeQuartiers = { nom: string; quartiers: string[] };

export const VILLES = [
  "Bamako",
  "Kati",
  "Koulikoro",
  "Kayes",
  "Kita",
  "Sikasso",
  "Koutiala",
  "Bougouni",
  "Ségou",
  "San",
  "Mopti",
  "Sévaré",
  "Djenné",
  "Tombouctou",
  "Gao",
  "Kidal",
  "Nioro du Sahel",
];

export const QUARTIERS_PAR_VILLE: Record<string, GroupeQuartiers[]> = {
  Bamako: [
    {
      nom: "Commune I",
      quartiers: ["Banconi", "Boulkassoumbougou", "Djélibougou", "Doumanzana", "Fadjiguila", "Korofina Nord", "Korofina Sud", "Sikoroni", "Sotuba"],
    },
    {
      nom: "Commune II",
      quartiers: [
        "Bagadadji",
        "Bakaribougou",
        "Bougouba",
        "Bozola",
        "Hippodrome",
        "Médina Coura",
        "Missira",
        "Niarela",
        "Quinzambougou",
        "TSF",
        "Zone industrielle",
      ],
    },
    {
      nom: "Commune III",
      quartiers: [
        "Badialan",
        "Bamako-Coura",
        "Bolibana",
        "Centre commercial",
        "Dar Salam",
        "Dravéla",
        "Kodabougou",
        "Koulouba",
        "Minkoungo",
        "N'Tomikorobougou",
        "Niomirambougou",
        "Ouolofobougou",
        "Point G",
        "Samé",
        "Sirakoro Dounfing",
      ],
    },
    {
      nom: "Commune IV",
      quartiers: ["ACI 2000", "Djicoroni Para", "Dogodouman", "Hamdallaye", "Kalabambougou", "Lafiabougou", "Lassa", "Sébénikoro", "Sibiribougou", "Taliko"],
    },
    {
      nom: "Commune V",
      quartiers: ["Baco-Djicoroni", "Badalabougou", "Daoudabougou", "Garantiguibougou", "Kalaban Coura", "Quartier Mali", "Sabalibougou", "Torokorobougou"],
    },
    {
      nom: "Commune VI",
      quartiers: [
        "Banankabougou",
        "Dianéguéla",
        "Faladié",
        "Magnambougou",
        "Missabougou",
        "Niamakoro",
        "Sénou",
        "Sirakoro Méguétana",
        "Sogoniko",
        "Sokorodji",
        "Yirimadio",
      ],
    },
  ],
};

export function quartiersDeLaVille(ville: string): GroupeQuartiers[] {
  const cle = Object.keys(QUARTIERS_PAR_VILLE).find((v) => v.toLowerCase() === ville.trim().toLowerCase());
  return cle ? QUARTIERS_PAR_VILLE[cle] : [];
}
