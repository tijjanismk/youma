# Coquille Windows (Tauri 2)

Fenêtre du poste central. Elle démarre `youma-server` dans le même processus puis affiche
`http://127.0.0.1:7878/` : c'est la même interface que celle servie aux téléphones.

## Construire l'installateur (sur Windows)

```powershell
cd ui; npm ci; npm run build; cd ..
cd apps/desktop/src-tauri
$env:YOUMA_CLE_PUBLIQUE = "<clé publique de production>"
cargo install tauri-cli --version "^2" --locked
cargo tauri build
```

L'installateur NSIS (`target/release/bundle/nsis/`) :

* embarque WebView2 (installation **sans Internet**) ;
* crée l'exception pare-feu « Youma » (réseau privé) ;
* active le démarrage automatique avec Windows ;
* ne supprime jamais `%PROGRAMDATA%\Youma` (base et sauvegardes) à la désinstallation.

## Configuration du poste

`%PROGRAMDATA%\Youma\config.json` :

```json
{ "port": 7878, "reseau": true, "demo": false }
```

`reseau: true` = mode B (téléphones des serveurs). Nécessite le module « reseau » de la licence.
