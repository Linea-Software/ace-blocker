# Ace Blocker — Security audit di follow-up
**Data:** 26 ottobre 2026 · **Repository:** `C:\Projects\ace-project\ace-blocker` · **Ref:** `ad1963ebaa8a63cb41fc07cea436e6a7d67ea53f` (`main`, pulito all'inizio)

**Profilo:** standard, ambito repository completo · **Esito:** **INCOMPLETO — revisione statica / non attestazione di sicurezza**.

## Executive summary

Il repository unificato è stato ricognito per superfici di rischio che attraversano `apps/ace-app`, `crates/ace-engine`, `ace-watchdog`, `ace-watchdog-protocol`, `ace-tun`, `mitm-proxy`, `ace-domain-blocklist`, `installer` e CI. Non sono stati letti integralmente tutti i file né verificati tutti i percorsi di esecuzione; **"ambito completo" non equivale a "copertura completa"**. Sono stati riesaminati i tre lead del precedente audit e identificati due nuovi percorsi da approfondire. **Nessuna vulnerabilità ha superato la validazione indipendente richiesta dalla skill**; non assegnare severità formale o trattare questi lead come exploit dimostrati.

Il confine prioritario è la **coesistenza di componenti LocalSystem e processi/utenti non privilegiati**. L'installer predispone lettura ereditabile in ProgramData; il servizio salva nello stesso ambito la capability IPC, la credenziale API cloud e la private key della CA locale. L'effettiva autorizzazione di un secondo utente Windows va verificata sul pacchetto installato. Il secondo asse è il controllo delle **operazioni IPC senza capability** e dei **callback browser → account**; il terzo è la disponibilità del processo privilegiato su input di lista non fidati.

## Registro dei lead statici, non indipendentemente validati

| ID | Priorità di verifica | Confine / componente | Ipotesi specifica | Stato |
| --- | --- | --- | --- | --- |
| FUP-01 | Immediata, prima della release | MSI → SharedData → segreti / RPC | DACL effettive possono rendere accessibile la capability IPC, la chiave cloud e la CA a un altro account locale | Non validato |
| FUP-02 | Immediata, prima della release | Pipe multiutente → RPC lettura → backup | Client locale privo di capability può ottenere backup/configurazione, se tali dati sono intesi come privati | Non validato |
| FUP-03 | Immediata, prima della release | Browser → deep link → associazione account | Callback `aceblocker://account/api-key` senza state app-bound può sostituire una chiave senza richiesta pendente | Non validato |
| FUP-04 | Alta, prima della release | Lista URL/file → parser → memoria LocalSystem | Righe fino alla dimensione totale del download possono gonfiare `line_buffer` senza limite per riga | Non validato |
| FUP-05 | Da misurare prima della release | Utente locale → pipe → sessioni daemon | La creazione di un task per connessione senza cap esplicito potrebbe esaurire risorse condivise | Non validato |

I lead FUP-01–03 riprendono e ricontrollano il report precedente, che li aveva marcati `needs_validation`. I lead FUP-04 e FUP-05 estendono la ricognizione. Non è stata trovata prova di compromissione, di bypass delle regole di lock o di sfruttamento di servizi in esercizio.

### FUP-01 — Isolamento dei segreti (ACL e identità IPC)

**Percorso:** `installer/installer.wxs:209-215` imposta `(A;OICI;GRGX;;;BU)` sulla cartella dati; `crates/ace-engine/src/settings.rs:93-99,144-190` posiziona database, certificati e `capability.token` nel percorso SharedData e salva il token con `std::fs::write`; `crates/mitm-proxy/src/ca.rs:199-216` persiste il PEM privato; `crates/ace-engine/src/db/account_repo.rs:19-32` salva la chiave API nel DB; `crates/ace-engine/src/service/clients/ace_server_client.rs:9-20` la usa come credenziale cloud; `crates/ace-engine/src/service/api/rpc.rs:359-371` controlla la capability sui metodi che la richiedono.

**Condizione decisiva non osservata:** DACL effettive di `capability.token`, database e `mitm_ca.key` su installazione Windows reale, tenendo conto di ereditarietà, ownership e creazione dei file da LocalSystem. La lettura di un file su macchina installata non è stata dimostrata.

**Correzione se confermato:** separare contenitori pubblici da segreti con ACL file-specifiche e test multiutente. La Tauri app legittima deve poter richiedere operazioni; non rendere il token semplicemente inaccessibile a tutti gli utenti senza definire identità o broker. Proteggere la chiave CA per SYSTEM/Administrators, ruotare materiali già esposti, introdurre creazione esclusiva e fail-closed sulle scritture sensibili.

### FUP-02 — Esportazione del backup senza capability

**Percorso:** `crates/ace-engine/src/service/api/transport.rs:17-36` configura la pipe Windows con accesso `AU` e `accept_remote(false)`; Linux assegna al socket `0666` (`:66-80`). `crates/ace-engine/src/service/api/rpc.rs:359-371,438-447` tratta `required=false` come successo ed esporta il backup. `crates/ace-engine/src/manager/network/backup.rs:48-67,111-163` include nomi, regole, schedule e liste nel risultato.

**Condizione decisiva:** il modello privacy della macchina è multiutente con configurazioni private oppure stato di blocco deliberatamente condiviso? Manca una chiamata fittizia da altro account in VM isolata. Non è automatico un bypass di autenticazione: è un controllo esplicitamente differenziato per letture e scritture.

**Correzione condizionale:** se le configurazioni non devono essere visibili ai peer, richiedere capability/identità per `export_backup` e letture sensibili; lasciare solo stato pubblico minimizzato alle RPC pubbliche.

### FUP-03 — Deep link non associato a richiesta pendente

**Percorso:** `crates/ace-engine/src/protocol.rs:13-29,76-166` ammette solo il route `/account/api-key`; `crates/ace-engine/src/bin/ace_redirect.rs:38-73` estrae `apiKey` da fragment/query e chiama `setApiKey` con la capability locale; `crates/ace-engine/src/service/api/operations_account.rs:44-62` aggiorna la credenziale.

**Controllo positivo:** l'allowlist riduce l'invocazione di altre azioni privilegiate da un URL web. **Condizioni mancanti:** possibile consenso del browser/OS, stato di login lato sito esterno, intento dell'utente e impatto effettivo della sostituzione di account. Non attribuire al callback la capacità di spegnere il servizio, che è esclusa dall'allowlist.

**Correzione condizionale:** state/nonce monouso generato dall'app, scadenza, binding all'account e all'operazione iniziata, rifiuto callback non richiesti e test con due identità sintetiche.

### FUP-04 — Righe smisurate nell'import delle liste

**Percorso nuovo:** `crates/ace-engine/src/service/api/operations_site_pack.rs:42-79` prende sorgenti configurate dall'utente, previa verifica entitlement Pro; `crates/ace-engine/src/core/network_block/site_pack/importer/importer.rs:63-105` scarica la sorgente URL oppure usa un file; `crates/ace-engine/src/core/utils/file_download.rs:27-36,59-124` limita il download, ma il limite standard arriva a **2 GiB**. `importer.rs:140-161` usa un `Vec<u8>` e `reader.read_until(b'\n', &mut line_buffer)`, senza limite visibile alla lunghezza della singola riga; `read_until` accumula prima di classificare la riga.

**Possibile risultato:** una sorgente controllata da terzi contenente una riga molto lunga può causare grande allocazione durante il parsing, impegnando memoria del servizio anziché solo il disco del download. Non è stato eseguito alcun test OOM né osservata una caduta del servizio. L'entitlement, la capability e il limite download riducono l'accessibilità; non eliminano da soli la pressione memoria su un contenuto remoto legittimamente importato.

**Correzione raccomandata:** limite applicativo esplicito (es. 8–64 KiB per riga, motivato dal formato), lettura a chunk con scarto delle righe troppo lunghe senza accumulo, limite contenuti file locali e regression test con input sintetico piccolo/bounded. Evitare di impostare un tetto globale RAM osservato solo a valle del parsing.

### FUP-05 — Limiti di sessione IPC non espliciti nel wrapper

**Percorso:** `crates/ace-engine/src/service/api/transport.rs:29-62` esegue `accept()` e crea un `tokio::spawn` per connessione, poi handshaking remoc, senza limite/timeout applicativo esplicito nel codice mostrato. Il socket Linux ha permessi `0666` (`:66-90`). Un peer locale non necessita capability per avviare una sessione, mentre la verifica della capability avviene in seguito per metodo.

**Condizioni mancanti:** limiti effettivi di remoc, named pipe e del sistema operativo, cleanup in caso di connessione inattiva, costo per sessione, impatto sul percorso di enforcement. Nessuna saturazione o stress test su host condiviso.

**Hardening/test:** misurare in VM offline con pochi client dummy e limiti rigidi; applicare cap di sessioni, handshake/idle timeout e cleanup tempestivo se la misura dimostra crescita persistente non voluta.

## Controlli difensivi osservati

- Il wrapper Tauri nega `shutdown` prima di leggere la capability (`apps/ace-app/src-tauri/src/lib.rs:41-69`); l'engine conserva i gate del lock e il watchdog indipendente.
- L'RPC confronta digest con confronto constant-time (`crates/ace-engine/src/service/api/security.rs:3-14`); in produzione lo stato iniziale passa `Some(Settings::load_or_create_capability_token())` con enforcement abilitato (`crates/ace-engine/src/lib.rs:350-359`). Il caso `expected=None` di `rpc.rs:364-366` è fail-open nel codice ma non è dimostrato raggiungibile nel costruttore di release visibile: **hardening, non finding confermato**.
- L'import ZIP del database rifiuta traversal e impone un cap (`crates/ace-engine/src/core/network_block/site_pack/mod.rs:342-409`); gli update degli script controllano SHA-256 e limiti di estrazione (`crates/ace-engine/src/service/content_updater.rs:364-411`).
- La named pipe Windows imposta `accept_remote(false)` (`transport.rs:29-36`), e l'installer contiene gate di manutenzione/uninstall (`installer/installer.wxs:353-379,419-437`).

## Hardening non classificato come vulnerabilità

La configurazione Tauri espone `csp: null` e `withGlobalTauri: true` (`apps/ace-app/src-tauri/tauri.conf.json:3-17`), con scope FS documentato in `apps/ace-app/src-tauri/capabilities/default.json`: servono review XSS/origini prima della release, ma non è provata una XSS raggiungibile. Valutare restrizioni per operazione nel bridge generico, pinning di azioni CI, verifica di autenticità indipendente dei bundle e analisi CVE delle lockfile. Analizzare `std::env::temp_dir()/AceBlocker/temp_sites` (`settings.rs:116-127`) per ownership e reparse point su Windows: la raggiungibilità di un path-squatting non è dimostrata.

## Coverage, esecuzione e limiti

- **Source ref confrontati:** audit precedente `17c7e36a53374b3d07fb9eaef4f6120cdbbbe9e6` e ref corrente `ad1963ebaa8a63cb41fc07cea436e6a7d67ea53f`; fra i due, le modifiche Git osservate sono soltanto documentali. Le tracce applicative precedenti non risultano modificate.
- **Coverage ledger:** 21 unità per classi/superfici principali; **0 covered, 0 candidate indipendentemente verificate, 0 blocked, 21 deferred**. Priorità e verifica statica manuale non trasformano tali unità in `covered`.
- **Deleghe:** 4 ricognizioni indipendenti tentate via NovaGPT, tutte bloccate da richiesta di ispezione manuale del browser; nessun risultato accettabile ricevuto, 0 hunter, 0 critic, 0 verifier. Il workflow completo standard non è quindi completato.
- **Esecuzione:** nessun target process/build/test, nessun servizio Windows avviato, nessun endpoint reale, nessun input di attacco su rete, nessuna verifica MSI/ACL runtime, nessuna ricerca CVE con advisory aggiornata. Non è disponibile una VM Windows dimostrabilmente isolata secondo i requisiti della skill.
- **Validazione strutturale:** il validator ufficiale `validate-coverage-ledger.cjs` fallisce sul runtime Windows disponibile perché mancano primitive no-follow/nonblocking per aprire il JSON in modo sicuro; un eventuale controllo `validateDocument` in memoria è solo semantico, non sostituisce l'esecuzione ufficiale.
- **Out of scope:** il sito/account cloud `ace-blocker-site`, l'app Android, la pipeline esterna `ace-hosts`, le fonti remote dei contenuti, l'infrastruttura di produzione e tutti gli altri repository fratelli.

**Decisione operativa:** non basare il go-live su questo run. Verificare per primi FUP-01/02/03 in una VM Windows disposable multiutente; poi FUP-04/05 in un harness confinato; completare audit indipendente di ogni entrypoint RPC e dei moduli FFI/TUN, TLS, installer e supply chain. Le prove devono usare dati sintetici e non devono contattare produzione.

**File di run:** `C:\Projects\temp\ace-blocker-security-audit-run2\` (artefatti tecnici fuori dalla repo, conformemente all'isolamento previsto dalla skill). Questo documento è la copia destinata a `ace-blocker/docs`.
