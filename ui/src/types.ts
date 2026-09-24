// Types des réponses de l'API (miroir des structures de youma-core).

export type Session = {
  jeton: string;
  utilisateur: { id: string; nom: string; role_code: string; role_nom: string; employe_id: string | null; a_mot_de_passe: boolean };
  permissions: string[];
  plafond_remise_pct: number;
  /** Session confirmée par mot de passe (RG-AUT-06). */
  eleve: boolean;
};

export type Journee = { id: string; date_exploitation: string; statut: string; ouverte_le: number; cloturee_le: number | null };

export type EtatGeneral = {
  installe: boolean;
  /** Appareil du réseau pas encore autorisé : seul l'appairage est possible. */
  appairage_requis?: boolean;
  message?: string;
  demo: boolean;
  restaurant: string;
  horloge: { maintenant: number; dernier_evenement: number; coherente: boolean };
  journee: Journee | null;
  version: string;
  reseau: boolean;
  parametres: Parametres;
};

export type Parametres = {
  heure_bascule: number;
  fuseau_minutes: number;
  arrondi: number;
  seuil_ecart_caisse: number;
  reference_mm_obligatoire: boolean;
  paiement_avant_comptoir: boolean;
  paiement_avant_emporter: boolean;
  verrouillage_minutes: number;
  paie: { deduire_absences: boolean; jours_ouvrables_mois: number; plafond_avance_pct: number };
  cotisations: {
    inps_active: boolean;
    inps_salarie_bp: number;
    inps_employeur_bp: number;
    amo_active: boolean;
    amo_salarie_bp: number;
    amo_employeur_bp: number;
  };
  coupures: number[];
  quartiers: { nom: string; frais: number }[];
  largeur_ticket: number;
  imprimante_caisse: string;
  ouvrir_tiroir: boolean;
  dossier_sauvegarde_externe: string;
  alerte_sauvegarde_jours: number;
  intervalle_sauvegarde_minutes: number;
  canaux: Canaux;
};

/** Canaux de commande (fiche 0013) : le menu papier est toujours actif. */
export type Canaux = {
  telephone: boolean;
  qr_table: boolean;
  en_ligne: boolean;
  paiement_avance: boolean;
  paiement_a_la_livraison: boolean;
  plafond_paiement_livraison: number;
  avance_nouveau_client: boolean;
  /** rappel | sms */
  verification_numero: string;
  relais_url: string;
  relais_cle: string;
};

export type Entrante = {
  commande: Commande;
  canal: "qr_table" | "en_ligne";
  table_demandee: string | null;
  client_nom: string;
  client_telephone: string | null;
  paiement_mode: string | null;
  paiement_operateur: string | null;
  paiement_reference: string | null;
  validation_responsable: boolean;
  motif: string | null;
  commandes_precedentes: number;
  verification_numero: string;
};

export type ZoneRisque = {
  id: string;
  nom: string;
  quartier: string | null;
  lat: number | null;
  lon: number | null;
  rayon_m: number | null;
  debut_min: number;
  fin_min: number;
  jours: number;
  action: "bloquer" | "paiement_avance" | "validation_manuelle";
  message: string;
  actif: boolean;
};

export type MenuPublic = {
  restaurant: string;
  telephone: string;
  ouvert: boolean;
  table: string | null;
  qr_table: boolean;
  en_ligne: boolean;
  paiement_avance: boolean;
  paiement_a_la_livraison: boolean;
  verification_numero: string;
  operateurs: string[];
  quartiers: { nom: string; frais: number }[];
  categories: { id: string; nom: string; icone: string; couleur: string }[];
  produits: { id: string; categorie_id: string; nom: string; description: string; photo: string; prix: number; groupes_options: GroupeOptions[] }[];
};

export type ReponseEntrante = { statut: "en_attente" | "refusee"; message: string; numero: number | null; code_suivi: string | null; total: number };

export type Suivi = {
  numero: number;
  restaurant: string;
  etape: string;
  motif: string | null;
  type: string;
  total: number;
  reste: number;
  paiement_mode: string | null;
  lignes: [number, string][];
  livreur: [number, number, number] | null;
  destination: [number, number] | null;
  mis_a_jour: number;
};

export type Categorie = { id: string; nom: string; couleur: string; icone: string; ordre: number; actif: boolean };
export type OptionProduit = { id: string; nom: string; supplement: number };
export type GroupeOptions = { id: string; nom: string; min_choix: number; max_choix: number; options: OptionProduit[] };
export type Produit = {
  id: string;
  categorie_id: string;
  nom: string;
  nom_court: string;
  description: string;
  photo: string;
  prix: number;
  poste_id: string | null;
  disponible: boolean;
  actif: boolean;
  code: string;
  code_barres: string;
  taux_tva_bp: number;
  suivi_stock: string;
  article_stock_id: string | null;
  prix_achat_estime: number;
  ordre: number;
  prix_zones: { zone_id: string; prix: number }[];
  groupes_options: GroupeOptions[];
};
export type Poste = { id: string; nom: string; imprimante: string; ecran: boolean; actif: boolean };
export type Catalogue = { categories: Categorie[]; produits: Produit[]; postes: Poste[] };

export type Zone = { id: string; nom: string; ordre: number; actif: boolean };
export type TablePlan = {
  id: string;
  zone_id: string;
  nom: string;
  capacite: number;
  statut: "libre" | "occupee" | "reservee" | "a_nettoyer";
  commande_id: string | null;
  commande_numero: number | null;
  serveur: string | null;
  ouverte_le: number | null;
  a_envoyer: number;
  pretes: number;
};

export type Ligne = {
  id: string;
  produit_id: string;
  libelle: string;
  quantite: number;
  quantite_annulee: number;
  prix_unitaire: number;
  options: { id: string; nom: string; supplement: number }[];
  montant_options: number;
  commentaire: string;
  poste_id: string | null;
  envoi_id: string | null;
  statut: string;
  offert: boolean;
  offert_motif: string | null;
  montant: number;
};

export type Totaux = { brut: number; remises: number; offerts: number; frais_livraison: number; total: number; paye: number; reste: number };

export type Commande = {
  id: string;
  numero: number;
  journee_id: string;
  type: string;
  ordre_paiement: "avant" | "apres";
  table_id: string | null;
  table_nom: string | null;
  zone_id: string | null;
  serveur_nom: string | null;
  client_id: string | null;
  client_nom: string | null;
  employe_id: string | null;
  statut: string;
  livraison_statut: string | null;
  livraison_quartier: string | null;
  livraison_repere: string | null;
  livraison_telephone: string | null;
  livraison_frais: number;
  livreur_id: string | null;
  note: string;
  cree_le: number;
  lignes: Ligne[];
  envois: { id: string; numero: number; poste_nom: string | null; statut: string; message: string; cree_le: number }[];
  remises: { id: string; ligne_id: string | null; montant: number; motif: string }[];
  totaux: Totaux;
};

export type CommandeResume = {
  id: string;
  numero: number;
  type: string;
  table_nom: string | null;
  serveur_nom: string | null;
  client_nom: string | null;
  statut: string;
  livraison_statut: string | null;
  cree_le: number;
  total: number;
  reste: number;
};

export type Compte = { id: string; nom: string; type: string; operateur: string; employe_id: string | null; actif: boolean; solde: number };

export type SessionCaisse = {
  id: string;
  compte_id: string;
  compte_nom: string;
  caissier_id: string;
  caissier_nom: string;
  statut: string;
  ouverte_le: number;
  fond_compte: number;
  theorique_ouverture: number;
  solde_actuel: number;
  ecart: number | null;
};

export type Client = {
  id: string;
  nom: string;
  telephone: string | null;
  adresse: string;
  reperes: string;
  credit_autorise: boolean;
  limite_credit: number;
  actif: boolean;
  dette: number;
};

export type Employe = {
  id: string;
  nom: string;
  surnom: string;
  telephone: string;
  fonction: string;
  date_embauche: string | null;
  type_remuneration: string;
  montant_base: number;
  type_contrat: string;
  date_fin_contrat: string | null;
  piece_identite: string;
  contact_urgence: string;
  quartier: string;
  declare_inps: boolean;
  numero_inps: string;
  affilie_amo: boolean;
  numero_amo: string;
  avantages_nature: string;
  plafond_avance: number | null;
  horaires: string;
  statut: string;
  date_depart: string | null;
  notes: string;
  solde: number;
};

export type Bulletin = {
  id: string | null;
  numero: number | null;
  employe_id: string;
  employe_nom: string;
  fonction: string;
  debut: string;
  fin: string;
  type_remuneration: string;
  type_contrat: string;
  base: number;
  jours_presents: number;
  jours_absence_nj: number;
  report_precedent: number;
  total_gains: number;
  total_retenues: number;
  cotisations_salarie: number;
  charges_employeur: number;
  deja_paye: number;
  net_a_payer: number;
  lignes: { type: string; libelle: string; montant: number; quantite: number | null }[];
  paye_depuis: number;
  reste_a_payer: number;
};

export type Indicateur = { cle: string; libelle: string; valeur: number; formule: string };
export type Tableau = { titre: string; colonnes: string[]; lignes: (string | number | null)[][]; formule: string | null };
export type Rapport = { titre: string; debut: string; fin: string; indicateurs: Indicateur[]; tableaux: Tableau[] };

export type NiveauStock = {
  article_id: string;
  nom: string;
  unite: string;
  famille: string;
  quantite: number;
  seuil_alerte: number;
  cout_unitaire: number;
  valeur: number;
  alerte: boolean;
  conditionnements: { id: string; nom: string; contenance: number }[];
};

/** Recette (fiche 0014) : quantités entières dans l'unité de base de l'article (g, ml, pièce). */
export type LigneRecette = { article_id: string; quantite: number; article_nom?: string; unite?: string; cout_unitaire?: number };
export type Recette = {
  produit_id: string;
  lignes: LigneRecette[];
  options: { option_id: string; option_nom?: string; lignes: LigneRecette[] }[];
  cout: number;
};
export type CoutMatiere = { produit_id: string; nom: string; prix: number; cout: number; part_bp: number };
