# Ace Engine — audit del refactoring e backlog dedicato

**Ultimo aggiornamento:** 2026-10-22
**Repository:** `C:\Projects\ace-project\ace-blocker` (`main`, backlog residuo dopo interventi incrementali)
**Ambito:** `crates/ace-engine/` (core, manager, DB, servizi, API/RPC, proxy adapter, lifecycle, CLI, test/documentazione). Dipendenze `ace-domain-blocklist`, `ace-tun`, `mitm-proxy` e `ace-watchdog` considerate *solo* nei contratti e nei punti di integrazione dell'engine; **Ace App esclusa**.
**Procedura:** `AGENTS.md`, `crates/ace-engine/AGENTS.md`, `docs/ai/{REFACTORING_PLAYBOOK,BOUNDARIES_AND_ABSTRACTIONS,DOMAIN_TYPES,REVIEW_GATES}.md`; confronto con `TECH_DEBT_BACKLOG.md` e `docs/IPC_ARCHITECTURE.md`.

> **Nota:** questa versione è un backlog operativo di **soli interventi da completare**; gli AE con implementazione conclusa sono rimossi da queue e dettagli. La ricognizione ha coperto l'albero sorgente e i principali flussi per sottosistema, con letture mirate dei call site e ricerche statiche; non costituisce una review riga-per-riga di ogni sorgente generato/migrazione/test. `verified` significa che la struttura o il comportamento descritto è direttamente visibile nei file; **non** significa che un bug sia riprodotto in runtime. `candidate` richiede ulteriori verifiche; `decision` richiede approvazione della semantica prima di qualsiasi modifica. Priorità riferite al rischio e al costo di manutenzione, non a line count.

**Decisioni approvate (2026-10-22):** vedere [`ACE_ENGINE_REFACTOR_DECISIONS.md`](ACE_ENGINE_REFACTOR_DECISIONS.md). Ace Blocker è pre-release, quindi **la retrocompatibilità con formati/dati/sistemi legacy non è richiesta**; le scelte architetturali approvate sono registrate nel documento delle decisioni. La queue contiene solo gli interventi con implementazione residua; le review trasversali e i test di integrazione ancora necessari sono elencati nella matrice di verifica.

## Invarianti non negoziabili

- Blocchi, schedule, lock (anche uninstall gate), safe search, Resistance Mode e priorità delle eccezioni invariati; in caso di fallimento di storage o caricamento non presupporre un nuovo comportamento senza decisione esplicita.
- Engine e watchdog rimangono servizi indipendenti con recovery/maintenance autenticati; Tauri non assume il lifecycle.
- Il controllo locale rimane **remoc RPC tipizzato** su Named Pipe/Unix socket; l'engine mantiene autorizzazione capability e ACL, senza reintrodurre HTTP locale. Il server esterno e il proxy HTTP(S) sono domini differenti.
- Niente decrittazione dei processi non browser. Conservare certificati, streaming, cancellazione, retry e contratti correnti; non è richiesta la retrocompatibilità con formati storici pre-release.
- Il client `src/ace_server_api/` è generato: **non modificarlo a mano**. Non eliminare tool binari o protocolli ancora utilizzati senza accertarne i consumer effettivi; nessuno shim per versioni legacy è richiesto.

## Mappa dei componenti esaminati

| Area | Evidenza / entry point | Focus |
| --- | --- | --- |
| Bootstrap e lifecycle | `src/main.rs`, `src/lib.rs`, `src/platform.rs`, `src/watchdog.rs`, `src/lock_check.rs` | Startup, failover, separazione SCM, maintenance/uninstall |
| Regole e domini | `src/core/common/{domain_types,schedule,lock,injectable_scripts}.rs`, `src/core/network_block/{mod,user_defined_rules,regex_rules}.rs` | Type invariants, parsing, schedule, policy cache |
| Site pack | `src/core/network_block/site_pack/{mod,site_pack_db,importer/*}.rs` | Staging, errori, DB, identificatori, download |
| Orchestrazione | `src/manager/network/{mod,site_pack,network_blocks,network_blocks_to_site_pack,resistance_mode,schedule,backup}.rs`, `src/manager/account.rs` | DB vs memoria, locks, singole responsabilità |
| Persistenza | `src/db.rs`, `src/db/{schedule_repo,site_pack_repo,...}.rs`, migrations | Transazioni, fallimenti e modello SQL |
| Service/control | `src/service/api/{rpc,transport,bridge,security,operations_*}.rs` | Trust boundary e autorizzazione, error mapping |
| Update/proxy | `src/service/{content_updater,bundled_site_packs,proxy/*}.rs`, `src/core/utils/file_download.rs` | Staging, limite memoria, lock, contenuto |
| Utility, test e generazione | `src/bin/*`, `build.rs`, `docs/rest-api.md`, `Cargo.toml` | Artefatti legacy, tool/dev, copertura e documentazione |

## Queue prioritaria

| ID | Priorità | Stato | Area | Motivo / prossima azione |
| --- | --- | --- | --- | --- |
| AE-11 | P1 | implementation, VM pending | Script bundle activation | Journal e recupero al bootstrap; 5 crash point simulati e rollback SQL; resta kill/restart reale Windows |
| AE-13 | P2 | implementation, stress pending | Content updater | Snapshot e gate single-flight separati dal mutex metadata; pass manuale/background unificato; stress/cancel aperto |
| AE-14 | P2 | implementation, integration pending | API bridge | 27 chiamate semplificate, test di confronto 63 RPC; gate Tauri, status 425 e reconnect Windows ancora aperti |
| AE-15 | P2 | partial | Schedule/minute semantics | Secondi e frazioni rifiutati, test mezzanotte e merge; DST e round-trip integrato ancora aperti |
| AE-17 | P2 | implementation, stress pending | Site-pack importer | Download delegato a bounded downloader, progress conservato; 10 test importer; cancel/multi-source ancora aperti |
| AE-19 | P2 | partial | Test | Fixture possedute da OnceLock<TempDir> e recovery con path injected; override Settings globali ancora aperti |
| AE-26 | P2 | partial | Typed error propagation | Load Resistance Mode e cronologia updater ora propaga `Err` e ha test di corruzione DB; resta audit degli altri consumer e boundary |

---

## Dettaglio interventi

### AE-11 — Recovery dopo crash degli script [P1, verifica parziale]

- **Implementato:** journal persistente scritto prima delle rinomine, riconciliazione della versione SQLite al bootstrap e ripristino dell'ultima versione funzionante. Cinque test sui punti di interruzione e un test di errore SQL dopo lo swap sono superati.
- **Rimane:** test di terminazione reale del processo/riavvio SCM, handle Windows bloccati, perdita improvvisa di alimentazione e journal illeggibile. Nessuna validazione VM conclusiva.
- **Gate:** crash/riavvio ripetuti in VM Windows, coerenza versione/DB, enforcement dei lock e fault injection filesystem.

### AE-13 — Durata del mutex nell'updater [P2, verifica parziale]

- **Implementato:** mutex dei metadata mantenuto breve, snapshot del servizio per I/O di rete, gate single-flight condiviso per aggiornamenti manuali, automatici e download degli script, pubblicazione esplicita della cache. I test locali di concorrenza passano.
- **Rimane:** stress test con aggiornamenti di rete simultanei, cancellazione, cambi di account e retry.
- **Gate:** aggiornamenti background/manuale concorrenti, cancellazione dello staging e race sulle versioni.

### AE-14 — Duplicazioni nel bridge remoc [P2, verifica parziale]

- **Implementato:** macro locale per 27 chiamate remoc identiche senza argomenti, senza cambiare trasporto tipizzato e risposte JSON. Il test verifica la presenza nel bridge di tutti i 63 metodi del trait, inclusi gli stream.
- **Rimane:** integrazione su schema payload, capability/autorizzazione, status 425 di confirmUnlock, stream/cancellazione e reconnect Windows.
- **Invariante:** nessun nuovo protocollo JSON, framework RPC dinamico o cambiamento al contratto server.

### AE-15 — Contratto degli schedule al minuto [P2, verifica parziale]

- **Implementato:** entrambi i costruttori rifiutano secondi e nanosecondi non nulli con errore tipizzato (scelta A). Test caratterizzano la mezzanotte come limite di fine giornata; i test di merge passano. Il versionamento è già canonico.
- **Rimane:** verificare il passaggio ora legale/solare e round-trip DB/backup integrati, senza introdurre la precisione ai secondi.
- **Gate:** ore ripetute/inesistenti per DST, mezzanotte, lock legati ai giorni e persistenza DB.

### AE-17 — Download HTTP duplicato nell'importer [P2, verifica parziale]

- **Implementato:** gli URL dell'importer usano file_download_with_limit, conservando il limite di 2 GiB e il progress a percentuali intere. Aggiunto test per errore HTTP; dieci test importer superati.
- **Rimane:** test con molte sorgenti, cancellazione, disconnessione client, dimensioni HTTP e cleanup dei file temporanei.
- **Invariante:** conservare gli eventi streaming e gli errori per sorgente; nessuna nuova astrazione di download.

### AE-19 — Override globali delle directory di test [P2, parziale]

- **Implementato:** le directory condivise sono possedute da OnceLock<TempDir> senza mem::forget; cinque test di recovery usano percorsi espliciti, senza override globale Settings. La suite library è passata anche con quattro thread.
- **Rimane:** TEST_SITES_DIR, TEST_RESISTANCE_SCRIPTS_DIR e TEST_BUNDLED_PACKS_DIR usano ancora override process-global. Iniettare gradualmente i path nei componenti, evitando sostituzioni thread-local nei percorsi asincroni.
- **Gate:** suite parallele ripetute, isolamento delle fixture, cleanup e rimozione delle dipendenze dallo stato globale.

### AE-26 — Errori residui ancora codificati come valori validi [P2, partial]

- **Stato attuale:** dashboard, conteggi, lookup dei site pack, restore e update-check RPC propagano `Result` e distinguono errore, assenza del dato e valore vuoto.
- **Implementato (2026-10-22):** `load_resistance_manager` restituisce `Result` per settings/intensità/lock invalidi; la cronologia updater è caricata in modo fallibile durante il controllo aggiornamenti e non al bootstrap. Test di valori persistiti invalidi e letture fallite superati.
- **Rimane:** audit completo dei consumer e dei boundary ancora non trattati, senza indebolire la disponibilità approvata per l'avvio.
- **Prossimo refactor:** trasformare i fallimenti reali in `Err`/errori tipizzati, distinguendoli dall'assenza prevista. Solo al boundary di enforcement applicare le policy fail-open esplicitamente approvate, senza introdurre sentinelle primitive né modificare le garanzie lock/uninstall.
- **Gate:** errori di query e parse, vista stale, comportamento update-check, locked state, mapping degli errori RPC.

## Ordine operativo dei problemi residui

1. **Recovery e coerenza di enforcement:** AE-11, con test crash/restart e VM Windows.
2. **Concorrenza e orchestrazione RPC:** AE-13 e AE-14; caratterizzare single-flight, updater concorrente e dispatch tipizzato.
3. **Precisione schedule ed errori residui:** AE-15 e AE-26; rendere espliciti i contratti e gli errori senza cambiare la policy concordata.
4. **Semplificazioni P2 circoscritte:** AE-17 e AE-19; riuso prudente delle primitive di download e isolamento dei test.

## Matrice di verifica da applicare *prima* di implementare

| Dominio | Test minimi |
| --- | --- |
| Policy/lock/DB | `cargo test -p ace-engine --lib -- --test-threads=1`, test ad hoc DB write-failure, round-trip reboot, locked/unlocked |
| RPC/security | `cargo test -p ace-engine --lib` (remoc), Named Pipe DACL/capability in VM Windows, release debug-gates |
| Import/backup | Staging/cancel, backup corrente, SQLite error injection, checksum/zip bomb, path traversal, retry |
| Proxy/sitepack | Test su `service::proxy`, HTTP/HTTPS tunnel, DOM/script inject, non-browser non decrypt, cache invalidation |
| Watchdog/installer | `cargo test -p ace-watchdog-protocol` e VM SCM/recovery/maintenance/uninstall gate |
| Rust compile | `cargo fmt --all --check` (solo lettura), `cargo check -p ace-engine --all-targets`, lint mirato senza autofix |
| Audit/documentazione | Diff review architettura + dominio/tipi + correttezza **indipendenti**, percorsi e gate aggiornati |

**Ultimi gate registrati (2026-10-22):** `cargo test -p ace-engine --lib -- --test-threads=1`: **252/252** superati; suite parallela a 4 thread **251/251** superati prima dell'ultimo test recovery; `cargo check -p ace-engine --all-targets --all-features`: superato; `cargo test -p ace-watchdog-protocol`: **3/3**; `git diff --check`: superato. Non risultano conclusi test VM Windows su servizi/installer, recovery tramite crash reale e review indipendente. `cargo fmt --all --check` evidenziava difformità preesistenti in altri sorgenti non toccati. Per le decisioni di prodotto vedere `ACE_ENGINE_REFACTOR_DECISIONS.md`; per il debito generale `TECH_DEBT_BACKLOG.md`.
