# Ace Blocker — security audit statico (26 ottobre 2026)

> **Esito del run: INCOMPLETO (security-audit skill, profilo `standard`).** Il documento è un audit **statico, orientato al rischio, dell'intero repository unificato**; non è una certificazione di sicurezza né una validazione end-to-end. Sono state ispezionate le superfici più sensibili e inventariati tutti i 714 file tracciati, **non** lette integralmente tutte le righe di ogni file. Le quattro deleghe di ricognizione indipendente richieste dalla skill sono fallite per indisponibilità del backend NovaGPT; mancano quindi hunter, verifier e critic indipendenti. Non sono stati eseguiti target, test, installer, driver, proxy o servizi: le garanzie richieste dalla skill per un sandbox OS-enforced non sono state dimostrate. Nessuna voce sottostante è classificata `confirmed` o riceve una severity formale finché non viene verificata indipendentemente.

**Repository:** `C:\Projects\ace-project\ace-blocker`  
**Commit esaminato:** `17c7e36a53374b3d07fb9eaef4f6120cdbbbe9e6` (`main`, working tree pulita prima dell'audit)  
**Perimetro:** `apps/ace-app`, `crates/ace-engine`, `crates/ace-watchdog`, `crates/ace-watchdog-protocol`, `crates/ace-tun`, `crates/mitm-proxy`, `crates/ace-domain-blocklist`, `installer`, `.github/workflows` e configurazioni/documentazione del repository.  
**Esclusi:** sito web/account e API cloud esterni (`ace-blocker-site`), app Android, pipeline `ace-hosts`, sorgenti esterne degli script `boredom-engine`, configurazioni effettive delle macchine di produzione e dei provider.  
**Skill:** `ace-project/.agents/skills/security-audit/SKILL.md`, con companion desktop/IPC, RPC, memory safety, client/browser, web, release/supply-chain e availability.  
**Esecuzione:** sola lettura del sorgente e comandi di inventario/ricerca; nessun contatto con endpoint o dati reali, nessuna riproduzione offensiva.

## Sintesi esecutiva

La maggiore area di rischio osservata è il **confine fra processi normali Windows e componenti che girano come LocalSystem**. La configurazione del MSI rende **potenzialmente leggibile agli utenti standard** la cartella che contiene il token dell'IPC, la chiave API del servizio cloud conservata nel database e la chiave privata della CA di intercettazione TLS. Il token, una volta acquisito, autorizzerebbe richieste remoc normalmente riservate all'app, benché le regole di lock dell'engine costituiscano un controllo distinto che non va presunto aggirabile. In parallelo, un **RPC di esportazione del backup è raggiungibile senza presentare il token**, in base al codice.

Questi sono **lead prioritari da verificare prima della release** con installazione pulita e identità Windows distinte. Non è corretto concludere che l'installer *effettivamente* conceda l'accesso ai singoli file finché non si osservano gli ACL risultanti, o che una specifica escalation sia stata riprodotta.

### Registro delle verifiche prioritarie

| ID | Priorità di verifica | Stato | Superficie | Questione da stabilire |
| --- | --- | --- | --- | --- |
| AB-01 | Prima della release | `needs_validation` | MSI / capability / IPC | Token dell'installazione e credenziale cloud potenzialmente leggibili da altri utenti; token riutilizzabile via pipe |
| AB-02 | Prima della release | `needs_validation` | CA MITM / filesystem | Chiave privata di una root CA attendibile leggibile da utenti standard |
| AB-03 | Prima della release | `needs_validation` | remoc RPC / backup | Dati e preferenze del profilo esportabili da altra identità locale senza token |
| AB-04 | Prima della release | `needs_validation` | URI scheme / account | Callback `aceblocker://` può sostituire l'API key senza essere associato a un login avviato |
| AB-05 | Successiva, prima della release | `needs_validation` | Named Pipe / lifecycle | Connessioni locali illimitate assorbono risorse del servizio |
| AB-06 | Successiva, prima della release | `needs_validation` | Import site pack | Sorgenti URL/file elaborate da LocalSystem oltre l'autorità prevista per l'utente |

**Vulnerabilità formalmente confermate con il workflow completo della skill: 0.** Questa cifra indica **mancata conclusione della validazione**, non assenza di problemi.

## Confini di fiducia osservati

1. **Utente Windows → WebView Tauri → backend Rust → remoc:** il frontend inoltra `engine_call(op,args)` alla parte nativa, che legge `capability.token` e lo allega agli RPC. Il WebView non riceve direttamente il valore; è proibito l'op `shutdown` da WebView (`apps/ace-app/src-tauri/src/lib.rs:25-68,107-113`).
2. **Altri processi locali → pipe LocalSystem:** il pipe Windows accetta utenti autenticati locali con `accept_remote(false)`; su Linux il socket engine è `0666` (`crates/ace-engine/src/service/api/transport.rs:17-36,66-80`). La capacità è verificata a livello di singolo metodo, non al momento della connessione (`crates/ace-engine/src/service/api/rpc.rs:359-371`).
3. **Engine LocalSystem → database e filesystem:** backup, impostazioni, script e certificati risiedono nel perimetro dati dell'app. Sono sensibili anche quando il desktop non è aperto.
4. **Browser → TUN → proxy HTTPS:** solo il traffico browser deve essere decifrato, la verifica TLS upstream è abilitata nelle configurazioni ordinarie; il proxy genera la propria CA e installa la root nello store di fiducia (`crates/ace-engine/src/service/proxy/mod.rs:578-590,676-692,897-919`).
5. **MSI → SCM/watchdog:** i due servizi LocalSystem hanno policy di recovery e service ACL; la disinstallazione richiede controllo lock e shutdown con lease di manutenzione (`installer/installer.wxs:86-153,373-379,419-436`, `crates/ace-engine/src/service/api/operations_system.rs:105-149`).

## AB-01 — File del token IPC e ACL ereditabili

**Traccia statica.** L'MSI imposta sulla cartella `AceProgramDataDir` l'SDDL `D:PAI(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)(A;OICI;GRGX;;;BU)`, dove `BU` è il gruppo Users, `GRGX` include lettura e `OICI` abilita ereditarietà (`installer/installer.wxs:209-215`). `Settings::get_capability_token_path()` localizza il token sotto la directory `SharedData` (`crates/ace-engine/src/settings.rs:144-165`); `load_or_create_capability_token` genera 32 byte casuali e li persiste usando `std::fs::write`, senza un override ACL visibile nel percorso esaminato (`settings.rs:168-190`). In release `AppState` utilizza quel token con enforcement attivo (`crates/ace-engine/src/lib.rs:353-359`). Il client Tauri lo legge dal medesimo storage (`apps/ace-app/src-tauri/src/lib.rs:25-38`) e il server lo confronta in modo constant-time sulle chiamate protette (`crates/ace-engine/src/service/api/rpc.rs:359-371`, `security.rs:3-14`).

**Confine ipotizzato.** Un account locale diverso da quello che usa Ace, membro di `BU`, legge il file ed effettua RPC di modifica come client provvisto del segreto. **Non** ne consegue automaticamente un bypass dei lock applicativi: gli handler dell'engine rimangono titolari delle loro policy.

**Ulteriore segreto nella stessa directory.** Il database principale è collocato sotto `SharedData` (`crates/ace-engine/src/settings.rs:93-95`). Il campo SQLite `accounts.api_key` viene scritto in forma di stringa da `crates/ace-engine/src/db/account_repo.rs:19-32`; il client server riutilizza tale valore come credenziale API (`crates/ace-engine/src/service/clients/ace_server_client.rs:9-20`). Se `BU` ha accesso in lettura anche al database, il problema comprenderebbe una credenziale cloud, distinta dal token IPC. Questo è un rischio da verificare con **ACL e dati sintetici**, senza estrarre credenziali reali.

**Blocco di validazione:** DACL effettiva del file dopo creazione da parte del servizio; eventuali ACE esplicite/protezioni ereditate per token e database, owner e profilo di installazione; possibilità dell'account secondario di aprire il pipe e autorizzare una *sola operazione innocua* su dati fittizi.

**Verifica sicura:** in una **VM Windows monouso**, eseguire installazione con account test, osservare **soltanto le ACL** di cartella/token/database mediante `icacls` o `Get-Acl` e confrontarle da un secondo account standard. Usare soltanto un token e una risorsa fittizi isolati per un singolo RPC non distruttivo; non copiare, registrare o esporre credenziali reali.

**Correzione minima suggerita:** definire ACL esplicite sul **file** e verificare gli ACE post-creazione, con isolamento per SID dell'utente legittimo. Poiché la Tauri app gira come utente normale e deve leggere il token, **non** restringerlo semplicemente a SYSTEM rompendo l'IPC: progettare emissione/rotazione di capability per identità autorizzata oppure associazione sicura dell'identità peer lato engine, senza credenziale globale condivisa fra tutti gli utenti. Aggiungere test multi-account con ACL reali.

## AB-02 — Chiave privata della CA MITM nello stesso albero dati

**Traccia statica.** I certificati vengono collocati in `Settings::get_app_dir()/certs` (`crates/ace-engine/src/settings.rs:97-99`), con `mitm_ca.pem` e `mitm_ca.key` generati/gestiti dal servizio (`crates/ace-engine/src/service/proxy/mod.rs:897-915`). `CertAuthority::load_or_generate` scrive la **chiave privata** con `fs::write(key_path, &key_pem)` senza DACL per-file in quel percorso (`crates/mitm-proxy/src/ca.rs:199-223`). La directory madre è soggetta alla policy MSI di AB-01.

**Confine ipotizzato.** Se `BU` eredita lettura effettiva anche su `mitm_ca.key` e la root pubblica è considerata attendibile sul dispositivo, un utente locale non autorizzato può leggere la chiave che consente di firmare certificati associati a quella CA. L'effettiva possibilità di intercettare connessioni resta condizionata al controllo del traffico e alla trust configuration del client.

**Blocco di validazione:** DACL reali su `certs` e `mitm_ca.key`, effettiva installazione nello store Root usato dai browser, eventuale gestione dell'ACL altrove o trust store differenziati.

**Verifica sicura:** controllare con `Get-Acl`/`icacls` solo metadati di permessi in una VM nuova, con CA **di test**; esaminare il trust store senza esportare chiavi. Nessuna intercettazione di traffico reale.

**Correzione minima suggerita:** creazione atomica del file della CA con permessi espliciti **SYSTEM + Administrators** (ed eventuali SID di servizio strettamente necessari), verifica dell'accesso prima dell'installazione della root, migrazione/rigenerazione sicura se una vecchia chiave risultasse leggibile. Il client Tauri **non deve** avere accesso alla chiave privata. Test di ACL e trust store automatizzato su VM Windows.

## AB-03 — RPC di lettura sensibile privo di capability

**Traccia statica.** Su Windows il pipe consente R/W a `AU` (`crates/ace-engine/src/service/api/transport.rs:17-36`); su Linux il socket è impostato a `0666` (`:66-80`). Il metodo `check_capability(presented, required)` ritorna immediatamente successo per `required == false` (`crates/ace-engine/src/service/api/rpc.rs:359-371`). `export_backup` usa esplicitamente `false` e invoca l'export senza ulteriori controlli di identità nel percorso visto (`rpc.rs:438-447`; `operations_backup.rs:6-12`). L'oggetto `EngineBackup` include `network_blocks`, `custom_site_packs` e `deactivated_resistance_scripts`; ogni network block contiene nome, schedule, user rules e site pack connessi (`crates/ace-engine/src/manager/network/backup.rs:48-67,71-165`). Anche `get_all_network_blocks`, `get_user_rules`, `get_account_info` sono invocati con `false` (`rpc.rs:376-386,822-830,1360-1371`).

**Confine ipotizzato.** Un altro account locale, ammesso al pipe dal sistema operativo, ottiene preferenze e regole che appartengono al profilo di Ace senza il token. Non è provato dalla sola sorgente se il prodotto preveda espressamente condivisione di queste informazioni fra tutti gli utenti della macchina.

**Blocco di validazione:** modello d'uso multiutente, ACL Windows effettive del pipe, semantica del broker `remoc` su tutte le piattaforme e osservazione del risultato con dati dummy.

**Verifica sicura:** su macchina virtuale isolata con due account fittizi e backup sintetico, chiedere `export_backup(None)` con client remoc locale e verificare unicamente se il risultato contiene un marker fittizio. Non consultare backup di utenti reali.

**Correzione minima suggerita:** trattare tutte le letture con contenuto personale (backup, network blocks, user rules, account info, dashboard) come operazioni da autorizzare; mantenere eventualmente anonimi solo `ping` e versione. Distinguere autenticazione al pipe da autorizzazione ai dati, introducendo test per client `None`, token errato, token valido e SID alternativo.

## AB-04 — Account-key deep link non associato a una richiesta di login

**Traccia statica.** Il protocollo `aceblocker://` permette solo `/account/api-key` (`crates/ace-engine/src/protocol.rs:23-33,91-176`). Il receiver carica un `apiKey` direttamente dal frammento/query ricevuti e chiama l'RPC autenticato `setApiKey` (`crates/ace-engine/src/bin/ace_redirect.rs:23-72`). L'operazione aggiorna il credential store e poi verifica lo stato Pro (`crates/ace-engine/src/service/api/operations_account.rs:44-63`). Non è visibile in questi percorsi una correlazione con un nonce/stato di login avviato dall'app.

**Confine ipotizzato.** Un sito non attendibile induce l'utente ad aprire uno schema URI che lega Ace a una credenziale di un account diverso, con confusione sull'identità e sugli eventuali backup/sincronizzazioni. La conferma del browser, la provenienza del link e il comportamento del server dei token sono esterni al repository.

**Blocco di validazione:** procedura login del sito, UX dei browser, proprietà delle API key e capacità concreta di sostituire la sessione senza una login challenge legittima.

**Verifica sicura:** usare due identità sintetiche in VM con browser di test; osservare soltanto se una callback inattesa viene accettata dall'engine. Nessun account cloud reale.

**Correzione proposta se confermato:** associare callback a uno state/nonce one-time generato dall'app, con expiry e binding dell'operazione, più conferma esplicita quando si cambia identità. La allowlist di route attuale è una difesa positiva ma non un binding di sessione.

## AB-05 — Accettazione concorrente di client remoc senza quota evidente

**Traccia statica.** Ogni connessione accettata dalla listener viene servita in un `tokio::spawn`, usando `remoc::Cfg::default()` senza limite globale di connessioni visibile in questo file (`crates/ace-engine/src/service/api/transport.rs:29-63`). I client locali possono aprire il pipe prima della verifica della capability.

**Confine ipotizzato.** Un processo non autorizzato consuma risorse del servizio privilegiato con connessioni inattive. Mancano la quantificazione delle risorse, le eventuali mitigazioni interne di remoc e la misura di un impatto reale.

**Verifica sicura:** test sintetico con numero **piccolo, limitato e monitorato** di client in VM disposable e limiti OS di CPU/memoria/processi/tempo, interrompendo al primo effetto; mai contro il servizio condiviso. Possibili rimedi: semaforo concorrente, timeout handshake/inattività, limiti di payload e metriche.

## AB-06 — Sorgenti URL e file usate da un engine privilegiato

**Traccia statica.** Il comando `create_site_pack` prende sorgenti dall'utente e applica gating Pro (`crates/ace-engine/src/service/api/operations_site_pack.rs:43-80`); la creazione RPC richiede capability (`rpc.rs:1124-1132`). L'importer usa l'URL come target di `download_file_with_limit` oppure apre un percorso `File` così come configurato (`crates/ace-engine/src/core/network_block/site_pack/importer/importer.rs:58-105,124-135`); il downloader usa reqwest con timeout 10/60 secondi e tetto di 2 GiB (`crates/ace-engine/src/core/utils/file_download.rs:27-49,59-124`). L'engine è un servizio privilegiato in produzione.

**Confine ipotizzato:** la funzione di import legittima potrebbe estendersi a risorse locali o indirizzi di rete che il processo chiamante non può accedere direttamente. **Non** è stato dimostrato un esito di lettura dati, un SSRF con conseguenze verso un'altra identità né un aggiramento dei controlli Pro/capability. Non trattare la scelta intenzionale dell'utente di importare file come vulnerabilità di per sé.

**Blocco di validazione:** destinazioni permesse dal prodotto, filtro dei protocolli/redirect, natura dei permessi del processo, modalità con cui l'eventuale contenuto diviene osservabile, e limiti infrastrutturali non presenti nel repository. Test soltanto con file dummy e server loopback isolato, mai servizi interni o di produzione. Se il confine è reale, limitare `File` a origini autorizzate e le URL ai protocolli/destinazioni previsti, ricontrollando redirect.

## Hardening senza vulnerabilità dimostrata

- **WebView:** `apps/ace-app/src-tauri/tauri.conf.json` ha `app.security.csp: null` e `withGlobalTauri: true`. Il frontend può fare chiamate native con capacità dell'app (eccetto shutdown). Una CSP restrittiva, una allowlist di comandi/operazioni e una revisione degli ingressi HTML sono consigliate, ma **non è stato dimostrato un percorso XSS**; non attribuire a questo solo fatto una vulnerabilità.
- **Segreti durante la generazione:** il token viene scritto con `std::fs::write`; valutare persistenza fail-closed e creazione esclusiva/atomica per evitare permessi iniziali involontari, soprattutto dopo reinstallazioni e in presenza di preesistenze. Non è dimostrata una race sfruttabile.
- **Delivery di contenuti:** la verifica SHA-256 degli script è utile, ma hash e URL arrivano dal metadata del server. Eventuale verifica firma indipendente, pinning di chiavi o manifest firmato è decisione di threat model, non un incidente dimostrato (`crates/ace-engine/src/service/content_updater.rs:320-409`).
- **Dipendenze e supply chain:** `Cargo.lock` e `apps/ace-app/bun.lock` sono presenti, ma **nessuna verifica CVE aggiornata** o scansione con advisory database è stata eseguita. La pipeline release usa alcuni GitHub Actions taggati (`.github/workflows/release.yml`) e genera manifesto SHA-256: valutare action pin per commit, attestazioni e firma autentica MSI, distinta dal semplice hash.
- **Unsafe/FFI:** WinTun, Windows networking, Linux/macOS TUN e funzioni packet/process utilizzano blocchi `unsafe`. L'inventario li ha identificati, ma **nessuna prova di memory corruption** e nessun fuzzing sono stati svolti. Eseguire fuzzing/strumentazione dei parser e test ABI in sandbox dedicato.
- **Policy di installazione:** su `installer/installer.wxs:419-436` il controllo lock precede la preparazione manutenzione, non il contrario come afferma un commento adiacente; allineare commenti, design e risultati di un reale test MSI, senza supporre un bypass attivo.

## Controlli positivi osservati nel sorgente

- Nessun listener HTTP alternativo per il control plane nell'architettura dichiarata; remoc IPC nativo e `accept_remote(false)` Windows (`docs/IPC_ARCHITECTURE.md`, `transport.rs:29-36`).
- Metodi RPC mutativi controllano la capability per metodo; `tokens_match` usa digest SHA-256 e `subtle::ConstantTimeEq` (`rpc.rs:359-371`, `security.rs:3-14`).
- La UI non può chiedere `shutdown` tramite `engine_call`; l'engine ricontrolla i lock e richiede lease del watchdog prima dello shutdown (`apps/ace-app/src-tauri/src/lib.rs:44-58`, `operations_system.rs:105-149`).
- Lock e autorizzazione Pro sono verificati anche negli handler sensibili, non solo nel frontend (`operations_site_pack.rs:43-80,83-128`).
- Downloader con timeout/size cap e ZIP extraction con controllo dei segmenti di percorso e dimensione totale (`crates/ace-engine/src/core/utils/file_download.rs:27-49,219-297`).
- Le operazioni `debug` più pericolose sono compile-gated e l'enforcement della capability è abilitato in release (`crates/ace-engine/src/service/api/rpc.rs:510-560`, `crates/ace-engine/src/lib.rs:353-359`).
- Permessi restrittivi per il socket separato del watchdog Linux (`crates/ace-watchdog/src/linux.rs:350-369`).

## Copertura e lacune

| Componente | Analisi statica svolta | Lacuna decisiva |
| --- | --- | --- |
| `ace-engine` API/IPC/autorizzazione | Percorsi Tauri → remoc → handler, token, backup e operazioni di sistema | Client multiutente reale, verifica delle 63 chiamate con matrice di permessi |
| `ace-engine` lock/schedule/backup | Contratti e flussi lock, shutdown e import; test esistenti inventariati | State-machine test/fuzz/restore differenziale e race |
| `ace-watchdog` e protocollo | Architettura, permessi e integrazione installer esaminati | Verifica heartbeat/recovery/SCM con perdita servizio reale in VM |
| `mitm-proxy` e CA | Generazione/salvataggio chiavi, TLS interception e trust policy | ACL root key, TLS/cert chain e browser con client cert |
| `ace-tun`, networking, `ace-domain-blocklist` | Inventario `unsafe`/FFI, indirizzamento e input trust boundaries | Fuzz packet parsing, crash containment, VPN enforcement e driver |
| `ace-app` Svelte/Tauri | Bridge IPC, capability, CSP, capabilities e comandi nativi | Revisione esaustiva XSS su tutte le route e test WebView nativi |
| `installer` WiX | DACL, SCM, recovery, root cert, gate uninstall, custom actions | MSI effettivo, repair/upgrade/uninstall e permessi file su clean install |
| `.github/workflows`, dipendenze | Pipeline release, permessi, lockfile e manifesti | CVE database, provenance attestation e verifica supply chain |
| Sito/account cloud esterno | Solo call-site e contratto osservabili nel desktop | **Fuori repository**: audit separato necessario |

**Metriche del run:** 714 file tracciati inventariati; 4 deleghe di reconnaissance tentate e fallite; 0 verificatori/critic/hunter operativi; 0 test di codice bersaglio; 0 verifiche runtime Windows/MSI; 0 validazioni advisory online; 0 vulnerabilità formalmente confermate. Nessun precedente `coverage-ledger.json` compatibile individuato per questo run. Il profilo `standard` richiesto dalla skill **non è stato portato a termine**; la copertura è parziale.

### Piano di chiusura suggerito

1. **Bloccare una release pubblica** finché AB-01/AB-02/AB-03 non sono stati risolti o confutati con controlli ACL su VM Windows pulita e due identità locali distinte.
2. Applicare test di regressione per ACL, letture RPC senza credenziale, stato della CA, riavvio servizi e uninstall gate; eseguire in VM offline usa-e-getta con dataset sintetici e output non sensibili.
3. Rieseguire la skill quando i revisori indipendenti NovaGPT e un sandbox OS-enforced siano disponibili, completando reconnaissance, hunter waves, verifier, critic, `findings.json` e i due validator della skill.
4. Pianificare separatamente review completa di frontend e moduli `unsafe`, e valutazione del sito/cloud; non trasferire automaticamente queste lacune alla categoria di bug confermato.

### Integrità del report

Questo documento è stato scritto come **artefatto del repository**, senza modificare codice applicativo né simulare una verifica riuscita. I numeri `file:line` fanno riferimento al commit indicato. I dati sconosciuti sono riportati esplicitamente come `needs_validation`; non sono state effettuate prove su utenti, servizi o host reali.
