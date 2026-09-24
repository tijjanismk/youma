//! Permissions fines et rôles par défaut (matrice : docs/conception/phase-3-acces-ecrans-api.md).

pub const COMMANDE_CREER: &str = "commande.creer";
pub const COMMANDE_MODIFIER_AUTRES: &str = "commande.modifier_autres";
pub const COMMANDE_ANNULER_ENVOYE: &str = "commande.annuler_envoye";
pub const COMMANDE_REMISE: &str = "commande.remise";
pub const COMMANDE_OFFRIR: &str = "commande.offrir";
pub const COMMANDE_TRANSFERER: &str = "commande.transferer";
pub const COMMANDE_CONSO_EMPLOYE: &str = "commande.conso_employe";
pub const CUISINE_VOIR: &str = "cuisine.voir";
pub const CAISSE_ENCAISSER: &str = "caisse.encaisser";
pub const CAISSE_SESSION: &str = "caisse.session";
pub const CAISSE_MOUVEMENT: &str = "caisse.mouvement";
pub const CAISSE_RETRAIT_PROPRIETAIRE: &str = "caisse.retrait_proprietaire";
pub const CAISSE_ANNULER_PAIEMENT: &str = "caisse.annuler_paiement";
pub const CAISSE_VERIFIER_MM: &str = "caisse.verifier_mm";
pub const CAISSE_ECART: &str = "caisse.ecart";
pub const DEPENSE_CREER: &str = "depense.creer";
pub const STOCK_VOIR: &str = "stock.voir";
pub const STOCK_MOUVEMENT: &str = "stock.mouvement";
pub const STOCK_INVENTAIRE: &str = "stock.inventaire";
pub const STOCK_VALIDER_INVENTAIRE: &str = "stock.valider_inventaire";
pub const ACHAT_GERER: &str = "achat.gerer";
pub const CLIENT_GERER: &str = "client.gerer";
pub const CLIENT_CREDIT: &str = "client.credit";
pub const CLIENT_DEPASSER_LIMITE: &str = "client.depasser_limite";
pub const EMPLOYE_VOIR: &str = "employe.voir";
pub const EMPLOYE_GERER: &str = "employe.gerer";
pub const EMPLOYE_PRESENCE: &str = "employe.presence";
pub const EMPLOYE_AVANCE: &str = "employe.avance";
pub const EMPLOYE_DEPASSER_PLAFOND: &str = "employe.depasser_plafond";
pub const PAIE_GERER: &str = "paie.gerer";
pub const LIVRAISON_GERER: &str = "livraison.gerer";
pub const RAPPORT_VOIR: &str = "rapport.voir";
pub const CATALOGUE_GERER: &str = "catalogue.gerer";
pub const SALLE_GERER: &str = "salle.gerer";
pub const UTILISATEUR_GERER: &str = "utilisateur.gerer";
pub const PARAMETRE_GERER: &str = "parametre.gerer";
pub const JOURNEE_GERER: &str = "journee.gerer";
pub const SAUVEGARDE_GERER: &str = "sauvegarde.gerer";
pub const AUDIT_VOIR: &str = "audit.voir";
pub const HORLOGE_FORCER: &str = "horloge.forcer";
pub const LICENCE_GERER: &str = "licence.gerer";
pub const APPAREIL_GERER: &str = "appareil.gerer";

pub const TOUTES: &[&str] = &[
    COMMANDE_CREER, COMMANDE_MODIFIER_AUTRES, COMMANDE_ANNULER_ENVOYE, COMMANDE_REMISE, COMMANDE_OFFRIR,
    COMMANDE_TRANSFERER, COMMANDE_CONSO_EMPLOYE, CUISINE_VOIR, CAISSE_ENCAISSER, CAISSE_SESSION,
    CAISSE_MOUVEMENT, CAISSE_RETRAIT_PROPRIETAIRE, CAISSE_ANNULER_PAIEMENT, CAISSE_VERIFIER_MM, CAISSE_ECART,
    DEPENSE_CREER, STOCK_VOIR, STOCK_MOUVEMENT, STOCK_INVENTAIRE, STOCK_VALIDER_INVENTAIRE, ACHAT_GERER,
    CLIENT_GERER, CLIENT_CREDIT, CLIENT_DEPASSER_LIMITE, EMPLOYE_VOIR, EMPLOYE_GERER, EMPLOYE_PRESENCE,
    EMPLOYE_AVANCE, EMPLOYE_DEPASSER_PLAFOND, PAIE_GERER, LIVRAISON_GERER, RAPPORT_VOIR, CATALOGUE_GERER,
    SALLE_GERER, UTILISATEUR_GERER, PARAMETRE_GERER, JOURNEE_GERER, SAUVEGARDE_GERER, AUDIT_VOIR,
    HORLOGE_FORCER, LICENCE_GERER, APPAREIL_GERER,
];

/// (code, nom, plafond de remise %, permissions)
pub fn roles_par_defaut() -> Vec<(&'static str, &'static str, i64, Vec<&'static str>)> {
    let serveur = vec![COMMANDE_CREER, CUISINE_VOIR, CLIENT_GERER];
    let caissier = vec![
        COMMANDE_CREER, COMMANDE_TRANSFERER, CUISINE_VOIR, CAISSE_ENCAISSER, CAISSE_SESSION, CAISSE_MOUVEMENT,
        DEPENSE_CREER, CLIENT_GERER, CLIENT_CREDIT, STOCK_VOIR, LIVRAISON_GERER, JOURNEE_GERER,
    ];
    let mut gerant: Vec<&str> = TOUTES
        .iter()
        .copied()
        .filter(|p| {
            ![UTILISATEUR_GERER, LICENCE_GERER, HORLOGE_FORCER, CAISSE_RETRAIT_PROPRIETAIRE, PARAMETRE_GERER, EMPLOYE_DEPASSER_PLAFOND]
                .contains(p)
        })
        .collect();
    gerant.sort();
    let rh = vec![EMPLOYE_VOIR, EMPLOYE_GERER, EMPLOYE_PRESENCE, EMPLOYE_AVANCE, PAIE_GERER];
    let stock = vec![STOCK_VOIR, STOCK_MOUVEMENT, STOCK_INVENTAIRE, ACHAT_GERER];
    vec![
        ("proprietaire", "Propriétaire", 100, TOUTES.to_vec()),
        ("administrateur", "Administrateur", 100, TOUTES.to_vec()),
        ("gerant", "Gérant", 50, gerant),
        ("caissier", "Caissier", 10, caissier),
        ("serveur", "Serveur", 0, serveur),
        ("cuisinier", "Cuisinier", 0, vec![CUISINE_VOIR]),
        ("livreur", "Livreur", 0, vec![CUISINE_VOIR]),
        ("stock", "Magasinier", 0, stock),
        ("rh", "RH", 0, rh),
    ]
}
