//! HTTPS du réseau local (fiche 0020) : les téléphones n'installent l'application (PWA) qu'en HTTPS.
//!
//! Une autorité de certification propre au restaurant est créée une fois (conservée dans la base, donc
//! dans les sauvegardes) ; chaque téléphone l'installe une fois. Elle ne peut signer que des adresses
//! du réseau local : même volée, sa clé ne permet pas d'usurper un site Internet. Le certificat du poste central est refait à chaque démarrage pour ses
//! adresses du moment (le Wi-Fi peut changer l'IP du PC). Aucun service Internet n'intervient.

use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;

use axum::extract::ConnectInfo;
use axum::{Extension, Router};
use chrono::Datelike;
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto;
use hyper_util::service::TowerToHyperService;
use rcgen::{
    BasicConstraints, Certificate, CertificateParams, CidrSubnet, DnType, ExtendedKeyUsagePurpose, GeneralSubtree, IsCa, KeyPair, KeyUsagePurpose,
    NameConstraints, SanType,
};
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use tokio_rustls::TlsAcceptor;
use youma_core::db::{definir_valeur_systeme, valeur_systeme};
use youma_core::{Db, Erreur, Resultat};

const CLE_SYSTEME: &str = "tls_autorite";

fn err(e: impl std::fmt::Display) -> Erreur {
    Erreur::Fichier(std::io::Error::other(format!("HTTPS local : {e}")))
}

/// Dates de validité : d'hier (horloge du téléphone un peu en retard) à `annees` ans.
fn validite(p: &mut CertificateParams, annees: i32) {
    let h = chrono::Utc::now().date_naive() - chrono::Duration::days(1);
    p.not_before = rcgen::date_time_ymd(h.year(), h.month() as u8, h.day() as u8);
    // Le 28 existe tous les mois : pas de 29 février introuvable.
    p.not_after = rcgen::date_time_ymd(h.year() + annees, h.month() as u8, h.day().min(28) as u8);
}

/// Adresses que l'autorité a le droit de certifier (voir `params_autorite`).
pub fn adresse_locale_permise(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let o = v4.octets();
            v4.is_loopback() || v4.is_private() || (o[0] == 100 && (o[1] & 0xc0) == 64)
        }
        IpAddr::V6(_) => false,
    }
}

/// Paramètres de l'autorité : toujours les mêmes, pour re-signer avec la clé conservée.
fn params_autorite() -> CertificateParams {
    let mut p = CertificateParams::default();
    p.distinguished_name.push(DnType::CommonName, "Youma - autorite locale du restaurant");
    p.distinguished_name.push(DnType::OrganizationName, "Youma");
    p.is_ca = IsCa::Ca(BasicConstraints::Constrained(0));
    p.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign, KeyUsagePurpose::DigitalSignature];
    // Réseaux privés et le PC lui-même seulement.
    p.name_constraints = Some(NameConstraints {
        permitted_subtrees: vec![
            GeneralSubtree::DnsName("localhost".into()),
            GeneralSubtree::IpAddress(CidrSubnet::from_v4_prefix([127, 0, 0, 0], 8)),
            GeneralSubtree::IpAddress(CidrSubnet::from_v4_prefix([10, 0, 0, 0], 8)),
            GeneralSubtree::IpAddress(CidrSubnet::from_v4_prefix([172, 16, 0, 0], 12)),
            GeneralSubtree::IpAddress(CidrSubnet::from_v4_prefix([192, 168, 0, 0], 16)),
            // Box 4G et certains routeurs d'opérateur.
            GeneralSubtree::IpAddress(CidrSubnet::from_v4_prefix([100, 64, 0, 0], 10)),
        ],
        excluded_subtrees: vec![],
    });
    p
}

/// Autorité du restaurant, et le certificat tel qu'il a été installé sur les téléphones (PEM).
pub struct Autorite {
    cle: KeyPair,
    certificat: Certificate,
    pub pem: String,
}

impl Autorite {
    /// Charge l'autorité du restaurant ou la crée (clé + certificat valables 10 ans).
    pub fn charger_ou_creer(db: &Db) -> Resultat<Self> {
        if let Some(texte) = valeur_systeme(db.conn(), CLE_SYSTEME)? {
            let debut = texte.find("-----BEGIN CERTIFICATE-----").ok_or_else(|| err("autorité incomplète"))?;
            let cle = KeyPair::from_pem(&texte[..debut]).map_err(err)?;
            let certificat = params_autorite().self_signed(&cle).map_err(err)?;
            return Ok(Autorite { cle, certificat, pem: texte[debut..].to_string() });
        }
        let cle = KeyPair::generate().map_err(err)?;
        let mut p = params_autorite();
        validite(&mut p, 10);
        let certificat = p.self_signed(&cle).map_err(err)?;
        let pem = certificat.pem();
        definir_valeur_systeme(db.conn(), CLE_SYSTEME, &format!("{}{}", cle.serialize_pem(), pem))?;
        Ok(Autorite { cle, certificat, pem })
    }

    /// Certificat de l'autorité en DER (fichier `.crt` que le téléphone installe).
    pub fn der(&self) -> Resultat<Vec<u8>> {
        Ok(CertificateDer::from_pem_slice(self.pem.as_bytes()).map_err(err)?.to_vec())
    }

    /// Certificat du poste central pour ses adresses (1 an, refait à chaque démarrage).
    pub fn certificat_serveur(&self, ips: &[IpAddr]) -> Resultat<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>)> {
        let cle = KeyPair::generate().map_err(err)?;
        let mut p = CertificateParams::new(vec!["localhost".to_string()]).map_err(err)?;
        p.distinguished_name.push(DnType::CommonName, "Youma - poste central");
        p.subject_alt_names.extend(ips.iter().filter(|ip| adresse_locale_permise(ip)).map(|ip| SanType::IpAddress(*ip)));
        p.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
        p.key_usages = vec![KeyUsagePurpose::DigitalSignature, KeyUsagePurpose::KeyEncipherment];
        validite(&mut p, 1);
        let c = p.signed_by(&cle, &self.certificat, &self.cle).map_err(err)?;
        let chaine = vec![c.der().clone()];
        Ok((chaine, PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(cle.serialize_der()))))
    }
}

/// Configuration TLS du serveur (fournisseur cryptographique `ring`, déjà utilisé par reqwest).
pub fn config_serveur(autorite: &Autorite, ips: &[IpAddr]) -> Resultat<Arc<rustls::ServerConfig>> {
    let (chaine, cle) = autorite.certificat_serveur(ips)?;
    let mut c = rustls::ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
        .with_safe_default_protocol_versions()
        .map_err(err)?
        .with_no_client_auth()
        .with_single_cert(chaine, cle)
        .map_err(err)?;
    c.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
    Ok(Arc::new(c))
}

/// Sert l'application en HTTPS (mêmes routes que l'HTTP, WebSocket compris).
pub async fn servir(app: Router, ecoute: tokio::net::TcpListener, config: Arc<rustls::ServerConfig>) {
    let accepteur = TlsAcceptor::from(config);
    loop {
        let Ok((tcp, adresse)) = ecoute.accept().await else { continue };
        let accepteur = accepteur.clone();
        let app = app.clone().layer(Extension(ConnectInfo::<SocketAddr>(adresse)));
        tokio::spawn(async move {
            // Un téléphone qui refuse le certificat coupe la poignée de main : rien à signaler.
            let Ok(flux) = accepteur.accept(tcp).await else { return };
            let service = TowerToHyperService::new(app);
            let _ = auto::Builder::new(TokioExecutor::new()).serve_connection_with_upgrades(TokioIo::new(flux), service).await;
        });
    }
}

#[cfg(test)]
mod tests {
    use super::adresse_locale_permise;

    #[test]
    fn autorite_limitee_au_reseau_local() {
        for ip in ["127.0.0.1", "192.168.1.20", "10.0.0.5", "172.20.3.4", "100.72.1.9"] {
            assert!(adresse_locale_permise(&ip.parse().unwrap()), "{ip}");
        }
        for ip in ["8.8.8.8", "41.73.120.5", "172.32.0.1", "100.128.0.1", "::1"] {
            assert!(!adresse_locale_permise(&ip.parse().unwrap()), "{ip}");
        }
    }
}
