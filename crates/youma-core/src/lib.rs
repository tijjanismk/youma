//! Youma — cœur métier : règles (RG-*), persistance SQLite, rapports, impression, licence.
//! L'interface ne touche jamais la base : elle passe par l'API du serveur, qui appelle ce crate.

// Lignes SQL lues en tuples, à usage local : plus lisible qu'un type par requête.
#![allow(clippy::type_complexity)]

pub mod achats;
pub mod appareils;
pub mod auth;
pub mod caisse;
pub mod catalogue;
pub mod clients;
pub mod commandes;
pub mod consignes;
pub mod db;
pub mod demo;
pub mod entrantes;
pub mod employes;
pub mod erreur;
pub mod horloge;
pub mod impression;
pub mod journee;
pub mod licence;
pub mod livraison;
pub mod paie;
pub mod parametres;
pub mod permissions;
pub mod promotions;
pub mod rapports;
pub mod recettes;
pub mod releves_mm;
pub mod salle;
pub mod sauvegarde;
pub mod sortie;
pub mod stock;
pub mod zones_risque;

pub use db::{Acteur, Db};
pub use erreur::{Erreur, Resultat};
