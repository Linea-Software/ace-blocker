# Architettura e confini di fiducia — Ace Blocker

Revisione statica locale, commit `17c7e36a`, 26 ottobre 2026. Nessun collegamento a endpoint pubblici o installazioni attive.

## Principali componenti

- **Engine** (`crates/ace-engine`): servizio privilegiato LocalSystem su Windows e servizio systemd su Linux; SQLite, regole di blocco, lock/schedule, RPC remoc, aggiornatore di contenuti, MITM TLS.
- **Watchdog** (`crates/ace-watchdog`, `ace-watchdog-protocol`): servizio indipendente che sorveglia engine, verifica PID/SCM o systemd e gestisce manutenzione.
- **App** (`apps/ace-app`): SvelteKit 5 in WebView Tauri 2; bridge Rust verso l'IPC; componenti JavaScript non devono ricevere il token.
- **Proxy/TUN** (`crates/mitm-proxy`, `ace-tun`, `ace-domain-blocklist`): inoltro TCP, intercept selettivo solo dei browser, CA generata localmente, policy di rete, FFI Windows/Linux.
- **Installer/CI** (`installer`, `.github/workflows`): WiX/MSI, avvio e ACL dei servizi, cartella dati ProgramData, certificati attendibili, pacchetti offline, rilascio.
- **Fuori repository**: sito/account Ace, Android, source scripts boredom-engine, pipeline ace-hosts; non inferire da qui la loro sicurezza.

## Principali confini

1. **Altro processo locale → engine LocalSystem**: pipe Windows con DACL o socket Unix, remoc, capability richiesta sulle mutazioni; alcune letture intenzionalmente prive di capability. Fonti: `crates/ace-engine/src/service/api/{transport,rpc}.rs`.
2. **WebView → backend Tauri → engine**: il token è letto dal Rust Tauri, `shutdown` è negato al WebView; gli altri nomi di operazione sono inoltrabili verso il bridge. `apps/ace-app/src-tauri/src/lib.rs`.
3. **Utente standard → dati privilegiati**: directory comune creata da MSI con ACE ereditabili di lettura per BUILTIN Users; credenziale, database e chiave CA risiedono sotto SharedData. `installer/installer.wxs`, `crates/ace-engine/src/settings.rs`.
4. **Pagina web esterna → protocol handler**: `aceblocker://account/api-key` è consentito e può invocare `setApiKey` con il token locale; verifica di intento/linking da validare. `crates/ace-engine/src/{protocol.rs,bin/ace_redirect.rs}`.
5. **Rete Internet → proxy**: connessioni inoltrate e TLS condizionale; verifica upstream disabilitata solo in debug; contenuti scaricati con SHA-256 e ZIP bounded dove indicato.
6. **Installer/privilegi → servizi**: custom actions con `Impersonate=no`, gate lock e manutenzione; controlli effettivi richiedono VM Windows.

## Metodologia e limiti

Inventario Git completo (714 file tracciati) e ricerche statiche incrociate sui percorsi `apps/`, `crates/`, `installer/`, `.github/` e documentazione. Lettura diretta di percorsi ad alto rischio. Non equivale a ispezione semantica di ogni riga. Nessun prior-run ledger compatibile individuato in `docs/`; gli audit di refactoring esistenti non sono trattati come precedenti security audit.

La skill `security-audit` è stata caricata. Su richiesta esplicita non sono stati usati subagent. Non è disponibile nel flusso un sandbox OS-enforced verificato e nessun test target-controlled è stato eseguito. Di conseguenza **nessun record è `confirmed` secondo il contratto della skill**; tutte le piste restano `needs_validation`.
