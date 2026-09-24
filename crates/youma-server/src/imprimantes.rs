//! Envoi des tickets aux imprimantes (fiche 0006).
//! Destinations : `tcp:IP:PORT` (imprimante réseau ESC/POS, port 9100),
//! `fichier:CHEMIN` (tests, démonstration), `windows:NOM` (imprimante USB installée sous Windows).

use std::io::Write;
use std::time::Duration;

use youma_core::impression::{en_escpos, texte_brut};

pub fn envoyer(destination: &str, contenu: &str, ouvrir_tiroir: bool) -> Result<(), String> {
    let (genre, cible) = destination.split_once(':').ok_or("Destination d'imprimante invalide")?;
    match genre {
        "tcp" => {
            let adresse: std::net::SocketAddr = cible.parse().map_err(|_| format!("Adresse invalide : {cible}"))?;
            let mut flux = std::net::TcpStream::connect_timeout(&adresse, Duration::from_secs(3)).map_err(|e| format!("Imprimante injoignable ({e})"))?;
            flux.set_write_timeout(Some(Duration::from_secs(5))).ok();
            flux.write_all(&en_escpos(contenu, ouvrir_tiroir)).map_err(|e| e.to_string())
        }
        "fichier" => {
            let mut f = std::fs::OpenOptions::new().create(true).append(true).open(cible).map_err(|e| e.to_string())?;
            writeln!(f, "{}\n========", texte_brut(contenu)).map_err(|e| e.to_string())
        }
        "windows" => windows::imprimer_brut(cible, &en_escpos(contenu, ouvrir_tiroir)),
        _ => Err(format!("Type d'imprimante inconnu : {genre}")),
    }
}

#[cfg(windows)]
mod windows {
    //! Impression brute par le spouleur Windows (pilote « Generic / Text Only » ou pilote du fabricant).
    use std::ffi::c_void;
    use std::ptr::null_mut;

    #[repr(C)]
    struct DocInfo1W {
        doc_name: *const u16,
        output_file: *const u16,
        datatype: *const u16,
    }

    #[link(name = "winspool")]
    extern "system" {
        fn OpenPrinterW(name: *const u16, handle: *mut *mut c_void, defaults: *const c_void) -> i32;
        fn StartDocPrinterW(handle: *mut c_void, level: u32, info: *const DocInfo1W) -> u32;
        fn StartPagePrinter(handle: *mut c_void) -> i32;
        fn WritePrinter(handle: *mut c_void, buf: *const c_void, len: u32, written: *mut u32) -> i32;
        fn EndPagePrinter(handle: *mut c_void) -> i32;
        fn EndDocPrinter(handle: *mut c_void) -> i32;
        fn ClosePrinter(handle: *mut c_void) -> i32;
    }

    fn large(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    pub fn imprimer_brut(nom: &str, octets: &[u8]) -> Result<(), String> {
        let nom_w = large(nom);
        let doc = large("Youma");
        let raw = large("RAW");
        unsafe {
            let mut h: *mut c_void = null_mut();
            if OpenPrinterW(nom_w.as_ptr(), &mut h, std::ptr::null()) == 0 {
                return Err(format!("Imprimante Windows introuvable : {nom}"));
            }
            let info = DocInfo1W { doc_name: doc.as_ptr(), output_file: std::ptr::null(), datatype: raw.as_ptr() };
            let mut ok = StartDocPrinterW(h, 1, &info) != 0 && StartPagePrinter(h) != 0;
            if ok {
                let mut ecrits = 0u32;
                ok = WritePrinter(h, octets.as_ptr() as *const c_void, octets.len() as u32, &mut ecrits) != 0
                    && ecrits as usize == octets.len();
                EndPagePrinter(h);
                EndDocPrinter(h);
            }
            ClosePrinter(h);
            if ok {
                Ok(())
            } else {
                Err(format!("Échec d'impression sur {nom}"))
            }
        }
    }
}

#[cfg(not(windows))]
mod windows {
    pub fn imprimer_brut(nom: &str, _octets: &[u8]) -> Result<(), String> {
        Err(format!("Imprimante Windows « {nom} » : disponible uniquement sous Windows"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn impression_fichier_et_erreurs() {
        let d = tempfile::tempdir().unwrap();
        let f = d.path().join("ticket.txt");
        envoyer(&format!("fichier:{}", f.display()), "##TABLE 4\n2 × Bière", false).unwrap();
        let t = std::fs::read_to_string(&f).unwrap();
        assert!(t.contains("TABLE 4") && !t.contains("##"));
        assert!(envoyer("tcp:127.0.0.1:1", "x", false).is_err());
        assert!(envoyer("bidon", "x", false).is_err());
    }

    #[test]
    fn impression_tcp_escpos() {
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let adr = l.local_addr().unwrap();
        let h = std::thread::spawn(move || {
            let (mut s, _) = l.accept().unwrap();
            let mut v = Vec::new();
            std::io::Read::read_to_end(&mut s, &mut v).unwrap();
            v
        });
        envoyer(&format!("tcp:{adr}"), "**Bonjour", true).unwrap();
        let octets = h.join().unwrap();
        assert_eq!(&octets[..2], &[0x1b, b'@']);
        assert!(octets.windows(3).any(|w| w == [0x1b, b'p', 0]), "ouverture du tiroir");
    }
}
