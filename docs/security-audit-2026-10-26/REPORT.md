# Security Audit — Ace Blocker

**Data:** 26 ottobre 2026 · **Commit:** `17c7e36a53374b3d07fb9eaef4f6120cdbbbe9e6` (`main` pulito prima delle modifiche)  
**Ambito:** intera codebase del repository unificato (`apps`, `crates`, `installer`, CI); esclusi i repository esterni.  
**Profilo richiesto:** `standard` · **Stato:** **INCOMPLETO, prima passata statica** · **Subagent:** 0 (richiesta esplicita).  
**Esecuzione:** lettura di sorgenti, manifest e documenti; nessun test, build, accesso a servizi reali, attacco in rete o VM Windows.  
**Passate precedenti:** nessun ledger di security audit compatibile individuato. Nessun validator della skill eseguito.

## Risultato sintetico

Non è corretto dichiarare l'app sicura o pubblicabile. La lettura statica evidenzia **tre ipotesi source-grounded**, di cui una riguarda lo stesso punto di controllo per **credenziali cloud, token di controllo e chiave privata della CA TLS**. Il loro esito runtime, il modello di autorizzazione voluto e la verifica indipendente sono ancora assenti. Nessun finding soddisfa i criteri `confirmed` della skill. Le descrizioni qui sotto non dimostrano exploit su sistemi installati.

| ID | Stato | Confine interessato | Possibile risultato | Prossimo passo |
| --- | --- | --- | --- | --- |
| AB-SA-01 | Needs validation | File privilegiati → utenti locali | Lettura di chiave API, capability token, private key CA | Verificare ACL effettive dei tre file su MSI in VM |
| AB-SA-02 | Needs validation | Client locale senza capability → RPC di lettura | Esportazione delle configurazioni personali di blocco | Definire privacy boundary e osservare chiamata dummy |
| AB-SA-03 | Needs validation | Sito esterno → deep link → collegamento account | Account switch non intenzionale dopo apertura URL | Riprodurre con account fittizi, browser isolato e consenso noto |

## AB-SA-01 — Esposizione potenziale di segreti mediante ACL ProgramData

**Fonte primaria:** `installer/installer.wxs:210-214` crea la directory dati con `(A;OICI;GRGX;;;BU)` (read/execute ereditabile per BUILTIN Users). `crates/ace-engine/src/settings.rs:93-100,144-165` colloca database, `certs/` e capability sotto SharedData. `crates/mitm-proxy/src/ca.rs:199-216` scrive il PEM della **chiave privata della CA**; `crates/ace-engine/src/db/account_repo.rs:19-32` salva `api_key` come testo SQLite; `crates/ace-engine/src/service/clients/ace_server_client.rs:9-20` usa tale valore come `x-api-key`; `crates/ace-engine/src/settings.rs:168-189` scrive il token capability in testo.

**Ipotesi:** se le ACE BU vengono effettivamente ereditate dai file installati o generati dal servizio, un altro utente standard locale può leggere materiali che dovrebbero essere confinati. Il token autorizza le mutazioni RPC; la chiave API autentica verso il sito; la private key firma i certificati dell'intercettazione TLS. Non è una semplice informazione di debug. Il solo codice non prova ACL effettive su un host installato né che un altro processo possa usare quei materiali in un caso reale.

**Fix minimo proposto:** separare i segreti dalla cartella di dati di sola consultazione. DACL esplicita sul contenitore segreti e sui nuovi file, accesso SYSTEM/Administrators e accesso selettivo al Tauri client tramite un broker strettamente autorizzato o una capability per utente; non rendere semplicemente il token illeggibile al client legittimo. Proteggere CA key con ACL esclusive e creare file senza finestra temporale permissiva; migrare/ruotare chiavi e token precedenti dopo la correzione. Testare ACL e gestione multiutente in VM non condivisa.

## AB-SA-02 — Esportazione RPC senza autenticazione esplicita del chiamante

`crates/ace-engine/src/service/api/rpc.rs:438-447` consente `export_backup` con `check_capability(..., false)`. La risposta contiene network block, source list, schedule e script deactivated (`crates/ace-engine/src/manager/network/backup.rs:72-163`). Lo stesso criterio è applicato a `get_all_network_blocks` (`rpc.rs:822-831`); la named pipe Windows include `AU` (`transport.rs:25-42`), e su Linux il socket viene portato a `0o666` (`transport.rs:66-80`). Sono controlli di sola lettura **deliberatamente** non protetti da capability, non un bypass provato.

**Fatto mancante:** stabilire se il prodotto autorizza *qualsiasi* utente locale a vedere tutte le configurazioni della macchina oppure se nomi, regole e schedule sono dati privati di chi le ha impostate. **Fix condizionale:** se privati, applicare controllo capability/identità anche alle letture sensibili o separare lo stato innocuo da backup e regole.

## AB-SA-03 — Deep link di associazione account senza correlazione visibile

`crates/ace-engine/src/protocol.rs:11-20,27-35,76-153` dichiara che qualsiasi sito può presentare un link `aceblocker://` (il browser può chiedere conferma), consente solo il path `/account/api-key` ma non verifica un nonce/state di una richiesta di login pendente. `crates/ace-engine/src/bin/ace_redirect.rs:38-72` estrae `apiKey` e invia `setApiKey` usando la capability; `crates/ace-engine/src/service/api/operations_account.rs:44-62` sostituisce la chiave locale. Si tratta di una *possibile* forma di account-linking CSRF/account swapping.

**Fatto mancante:** comportamento del browser, necessità e qualità del consenso dell'utente, correlazione lato sito (repository esterno), e reale impatto con chiave non appartenente alla sessione locale. **Fix condizionale:** richiesta di collegamento iniziata dall'app, nonce monouso a breve scadenza e associazione al contesto d'utente; rifiutare callback non richieste.

## Controlli positivi osservati

- Capability verificata prima delle RPC di mutazione (`rpc.rs:359-371`); confronto a tempo costante dei digest (`service/api/security.rs:3-13`). Nota: ramo `None` del token atteso fail-open è da rendere fail-closed difensivamente, ma il costruttore runtime visibile passa `Some` (`lib.rs:350-359`): **non dichiarato vulnerabilità raggiungibile**.
- Tauri rifiuta `shutdown` dal WebView prima di leggere la capability (`apps/ace-app/src-tauri/src/lib.rs:41-63`).
- Lock su disabilitazione block (`manager/network/network_blocks.rs:88-118`) e sulla cancellazione (`:143-159`); ripristino backup salta blocchi locked (`manager/network/backup.rs:273-278`).
- SDDL pipe Windows con `accept_remote(false)` (`service/api/transport.rs:16-46`); watchdog separato con ACL più ristretta.
- Verifica TLS upstream disabilitata esclusivamente in debug (`service/proxy/mod.rs:686-695`).
- Import ZIP di pack limita la dimensione e controlla traversal (`core/network_block/site_pack/mod.rs:345-409`); script update con hash SHA-256 e limite download (`service/content_updater.rs:364-401`).
- Installer contiene gate di rimozione lock e richiesta di manutenzione autenticata (`installer/installer.wxs:353-379,419-437`).

## Lacune e piano di validazione

**Non eseguiti:** build/test Rust, Bun e WiX; audit CVE con advisory aggiornati; test Windows/Linux di IPC/ACL/SCM; verifica reale certificati/browser; fuzzing TUN, parser, proxy; critic/verifier indipendenti; schema validators della skill. L'intera codebase è stata considerata nell'inventario e nel piano, ma **non verificata esaustivamente**. Le parti FFI, rete, updater e interazioni simultanee restano coverage gap.

Prossima sequenza consigliata: (1) controllare ACL e segreti su installazione MSI usa-e-getta, (2) decidere visibilità IPC multiutente, (3) testare linking con due account fittizi; successivamente test isolati sulla pipeline di aggiornamento, proxy/TUN, watchdog e installer. Nessuna verifica su impianti di terzi o produzione.

### Sintesi della copertura registrata

Il ledger contiene **12 unità** trasversali a engine/RPC, segreti, lock/backup, deep link, Tauri, watchdog, proxy, TUN/FFI, aggiornamenti, rilascio, blocklist e frontend. Stato finale: **0 covered, 0 candidate verificati, 0 blocked, 12 deferred, 0 out_of_scope nel repository**, con i repository esterni esclusi dalla definizione del target. La critica di copertura indipendente **non è stata eseguita** (niente subagent). Questi numeri non rappresentano un controllo esaustivo: descrivono la parte non completata del programma di audit.

Consultare `NEEDS-VALIDATION.md` per procedure non invasive, `FINDINGS-DETAIL.md` per la mancanza di finding confermati, `coverage-ledger.json` per gli ambiti rimasti non conclusi.
