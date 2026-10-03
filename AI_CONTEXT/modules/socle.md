# Module : socle (base, droits, horloge, système)

Rôle : transaction métier, migrations, audit, permissions, sessions, appareils, licence, paramètres, sauvegardes, journée.

## Fichiers (`crates/youma-core/src/`)
- `db.rs` (494 l.) — `Db`, `Op`, `Acteur` ; `Db::executer` (l. 205) = `BEGIN IMMEDIATE` + garde d'horloge + audit ; `op.exiger` (324), `op.audit` (369), `op.outbox` (401), `op.evenement` (410) ; migrations `MIGRATIONS` (15), marqueur clés coupées (30) ; `valeur_systeme`/`definir_valeur_systeme`, `trouver`.
- `erreur.rs` (113 l.) — `Erreur` (Regle, Validation, NonTrouve, PinIncorrect, HorlogeIncoherente…), `ErreurApi` (codes `AUTORISATION_REQUISE`, `MOT_DE_PASSE_REQUIS`…).
- `horloge.rs` (120 l.) — `Horloge`, `HorlogeFixe` (tests), `date_exploitation`, `date_locale`, `plage_active`.
- `permissions.rs` (94 l.) — constantes `perm::*` (CAISSE_ENCAISSER, CLIENT_CREDIT, COMMANDE_OFFRIR…).
- `auth.rs` (579 l.) — Argon2, `connexion_pin` (verrouillage l. 323–346), `verifier_pin_autorisation` (107), `elever_session` (416–437), rôles, utilisateurs.
- `appareils.rs` (98 l.) — appairage par code, jeton haché, révocation.
- `secours.rs` (169 l.) — réinitialisation par code signé du fournisseur.
- `licence.rs` (181 l.) — Ed25519 ; `cle_publique` = `YOUMA_CLE_PUBLIQUE` sinon `CLE_DEV` (l. 18–21).
- `parametres.rs` (383 l.) — `Parametres` (Fidelite, Canaux, ParametresPaie, Cloud…), `lire`/`modifier`, `journal_audit`.
- `sauvegarde.rs` (249 l.) — sauvegarde, export, restauration, intégrité, rotation.
- `journee.rs` (152 l.) — ouverture, `blocages_cloture` (RG-JOU-04), clôture.
- `demo.rs` (371 l.) — base de démonstration (PIN 1234/2222/3333/4444/5555).

## Entrant
`db.rs` importé par 31 fichiers, `erreur.rs` par 34 ; `youma-server/src/api.rs` et `lib.rs` (INFERE via `use youma_core::…`).

## Base de données
Écrit : journal_audit, outbox, sequences, systeme, restaurant, roles, role_permissions, utilisateurs, sessions, appareils, parametres, sauvegardes, journees.

## Règles métier
- [CONFIRMÉ] RG-SYS-01/02 : `db.rs:204`. RG-SYS-04 : `db.rs:356`. RG-SYS-06 : `licence.rs:2`.
- [CONFIRMÉ] RG-AUT-03 : l'autorisation ponctuelle par PIN ne lit ni n'incrémente `echecs_pin` — `auth.rs:107` (constat C1, voir `ETUDE-MOBILE.md`).
