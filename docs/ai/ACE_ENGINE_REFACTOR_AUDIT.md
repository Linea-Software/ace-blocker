# Ace Engine — audit del refactoring e backlog dedicato

**Data:** 2026-10-22
**Repository:** `C:\Projects\ace-project\ace-blocker` (`main`, backlog residuo dopo interventi incrementali)
**Ambito:** `crates/ace-engine/` (core, manager, DB, servizi, API/RPC, proxy adapter, lifecycle, CLI, test/documentazione). Dipendenze `ace-domain-blocklist`, `ace-tun`, `mitm-proxy` e `ace-watchdog` considerate *solo* nei contratti e nei punti di integrazione dell'engine; **Ace App esclusa**.
**Procedura:** `AGENTS.md`, `crates/ace-engine/AGENTS.md`, `docs/ai/{REFACTORING_PLAYBOOK,BOUNDARIES_AND_ABSTRACTIONS,DOMAIN_TYPES,REVIEW_GATES}.md`; confronto con `TECH_DEBT_BACKLOG.md` e `docs/IPC_ARCHITECTURE.md`.

> **Nota:** questa versione contiene **soltanto le questioni ancora aperte**; gli interventi implementati e verificati sono stati rimossi dalla queue e dai dettagli. La ricognizione ha coperto l'albero sorgente e i principali flussi per sottosistema, con letture mirate dei call site e ricerche statiche; non costituisce una review riga-per-riga di ogni sorgente generato/migrazione/test. `verified` significa che la struttura o il comportamento descritto è direttamente visibile nei file; **non** significa che un bug sia riprodotto in runtime. `candidate` richiede ulteriori verifiche; `decision` richiede approvazione della semantica prima di qualsiasi modifica. Priorità riferite al rischio e al costo di manutenzione, non a line count.

**Decisioni approvate (2026-10-22):** vedere [`ACE_ENGINE_REFACTOR_DECISIONS.md`](ACE_ENGINE_REFACTOR_DECISIONS.md). Ace Blocker è pre-release, quindi **la retrocompatibilità con formati/dati/sistemi legacy non è richiesta**; le policy AE-01/03/04/05/11/15/16/26 hanno una scelta di prodotto esplicita. La queue qui sotto rimane un elenco di interventi aperti finché codice e verifiche non sono completati.

## Invarianti non negoziabili

- Blocchi, schedule, lock (anche uninstall gate), safe search, Resistance Mode e priorità delle eccezioni invariati; in caso di fallimento di storage o caricamento non presupporre un nuovo comportamento senza decisione esplicita.
- Engine e watchdog rimangono servizi indipendenti con recovery/maintenance autenticati; Tauri non assume il lifecycle.
- Il controllo locale rimane **remoc RPC tipizzato** su Named Pipe/Unix socket; l'engine mantiene autorizzazione capability e ACL, senza reintrodurre HTTP locale. Il server esterno e il proxy HTTP(S) sono domini differenti.
- Niente decrittazione dei processi non browser. Conservare certificati, streaming, cancellazione, retry, bundle precedenti e compatibilità con dati persistiti, versioni e payload.
- Il client `src/ace_server_api/` è generato: **non modificarlo a mano**. Non eliminare tool binari/protocolli/compatibilità solo perché citano tecnologia precedente senza accertarne gli utilizzatori.

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
| AE-01 | P0 | decision approved, behavior retained | Startup/enforcement | Fail-open con manager vuoto e log critico approvato; recovery end-to-end VM non verificato |
| AE-03 | P1 | implementation, review pending | Site-pack paths/ID | Parser ID fallibile, componenti sicuri e path tipizzati; verificare tutti i file ingress |
| AE-04 | P1 | implementation, review pending | Site-pack DB errors | Result distinto, warn e fail-open solo alla boundary; fault injection superata |
| AE-05 | P1 | implementation, review pending | Backup restore | Restore SQLite atomico per pack e blocco; lock invariati, fault injection SQL e reload su errore verificati |
| AE-11 | P1 | partial | Backup/scripts activation | Rollback su errore SQL + test; resta recovery crash tra rename/DB e VM Windows |
| AE-13 | P2 | verified | Content updater | Mutex servizio trattenuto nel download/staging/commit |
| AE-14 | P2 | verified | API bridge | Map dei nomi operazioni replicata rispetto al trait |
| AE-15 | P2 | partial | Version/clock | Versioni canoniche X.Y.Z + test; schedule mantenuti al minuto, gate DST/round-trip ancora aperti |
| AE-16 | P2 | implementation, review pending | Domain ID | ID vuoti invalidi, serde/DB/RPC validati, test; review consumer finale richiesta |
| AE-17 | P2 | candidate | Site-pack importer | Ripetizione gestione download/progress |
| AE-18 | P2 | implementation, review pending | DB pool | Pool/migration bootstrap condiviso fra DB distinti senza unificare repo/migrazioni; 22 test site-pack e check all-features superati |
| AE-19 | P2 | verified | Test | Override process-global delle directory di test |
| AE-26 | P2 | partial | Lock/errors | `Result` propagato in dashboard, count, site-pack open, updater, restore e version-check; restano fallback da valutare ai confini non modificati |

---

## Dettaglio interventi

### AE-01 — Fallback al manager vuoto [P0, candidate, **decision**]

- **Evidenza:** `src/lib.rs:286-301`: quando `NetworkManager::from` fallisce, viene eseguito `NetworkManager::new_empty` e l'engine prosegue; `src/manager/network/mod.rs:131-144` inizializza mappe vuote. **Rischio dedotto:** policy persistite non caricate potrebbero non essere applicate. Non è stato riprodotto un caso di avvio reale.
- **Refactor proponibile:** modellare esplicitamente esito e motivi del bootstrap, separando “configurazione vuota valida” da “configurazione non caricabile”; definire l'azione corretta solo dopo decisione sulla policy fail-safe. **Non introdurre un fail-closed nuovo in un refactor meccanico.**
- **Gate:** test su DB corrotto, lock persistiti, dati mancanti, recovery, avvio servizi su VM Windows/Linux; considerare effetti su uninstall/restart.

### AE-03 — Identificatori usati come path component [P1, candidate, **decision**]

- **Evidenza:** `src/core/network_block/site_pack/importer/mod.rs:43-58` definisce `SitePackId(String)` senza validazione; `src/core/common/domain_types.rs:4-24` valida `RemoteSitePackId` soltanto come non vuoto; `src/settings.rs:132-135` fa `PathBuf::push(id)`. Le ID canoniche remote non sono UUID.
- **Rischio da investigare:** verificare se e dove ID non attendibili/persistiti possono raggiungere `get_site_pack_path` e altre costruzioni di path; **non è dimostrata in questa audit una path traversal sfruttabile**.
- **Refactor:** imporre la validazione dei *singoli componenti* al confine filesystem/ingresso necessario, non inventare `Uuid` universali; conservare ID legacy `pack-*` e JSON/DB wire format.
- **Gate:** test `../`, separatori, drive prefixes Windows, ID canoniche preesistenti, backup, updater e import.

### AE-04 — Errore database interpretato come sito non bloccato [P1, candidate, **decision**]

- **Evidenza:** `src/core/network_block/site_pack/mod.rs:150-161`: `try_get_with(... db.blocking_result(...)).unwrap_or(BlockingResult::NotBlocked)`; la cache non memorizza errore ma il chiamante riceve la stessa risposta di un miss.
- **Costo:** diagnostica e semantica del fallimento del DB perse durante l'enforcement. **Non** assumere che tutti gli errori vadano convertiti in `Blocked`: è una nuova policy di sicurezza.
- **Refactor:** introdurre prima osservabilità/test per “no match” vs “I/O failure” senza cambiare risposte; discutere poi eventuale modello di decisione fallibile al confine di valutazione.
- **Gate:** DB temporaneamente bloccato/corrotto, ripristino automatico, latenza del proxy e non-browser passthrough, lock, cache.

### AE-05 — Restore backup non atomico [P1, verified, **decision**]

- **Evidenza:** `src/manager/network/backup.rs:171-362`: per ogni pack/block effettua scritture SQL separate; fallback a elenco vuoto su errore di lettura DB (255-261); elimina prima di ricreare un block (261-265); ora registra gli errori di associazione pack/script ma continua in modalità best-effort; ricostruisce memoria solo al termine (357-360).
- **Costo:** un errore intermedio può lasciare restore parziale e memoria divergente fino al reload; alcuni errori passano inosservati. Sono osservazioni del flusso di codice, non fault injection già eseguita.
- **Refactor:** prima test con failure injection su ogni fase e una definizione del comportamento del restore (best effort vs atomico); poi transazioni **per unità indipendente** o staging con rollback coerente, senza cancellare pack bloccati né cambiare schema.
- **Gate:** backup vecchi, Pro/free, ID catalogo, blocchi locked, errore SQLite, referential integrity, cloud restore, reinizializzazione.

### AE-11 — Commit script bundle filesystem + DB [P1, candidate, **decision**]

- **Evidenza:** `src/manager/network/resistance_mode.rs:223-280` sposta directories e ricarica l'albero prima della scrittura `db.update_scripts_version` (265), che può fallire; molte rename/cleanup di rollback sono best-effort. È intenzionalmente separato dai normali setters.
- **Costo/rischio:** un fallimento DB dopo uno swap riuscito può produrre stato disk/mem/version non coordinato; il comportamento preciso post-restart non è stato caratterizzato.
- **Refactor:** test dei crash point e delle scritture fallite, pianificare journal/rollback solo se richiesti, preservare last-known-good e script catalog verificato.
- **Gate:** Windows rename e file handles, versioni vecchie, hash, retry, riavvio su VM.

### AE-13 — Mutex dell'updater detenuto lungo tutta la passata [P2, verified]

- **Evidenza:** `src/service/content_updater.rs:647-685` apre `content_service.lock().await` prima di `check_for_updates`, `stage_pending_updates`, account calls e commit; `src/service/api/operations_resistance_mode.rs:108-124` tiene lo stesso mutex durante `stage_resistance_scripts(...).await`.
- **Costo:** il lock dell'intero servizio può serializzare download e comandi non dipendenti; è necessario capire quali campi realmente richiedono mutabilità condivisa.
- **Refactor:** tracciare stato e consumers, provare snapshot per metadata/parametri e lock più brevi senza introdurre race fra due update; non eliminare il serializzatore finché non c'è un'altra garanzia single-flight.
- **Gate:** download concorrente/background, cancellazione, release/version race, scripts vs site packs, retry.

### AE-14 — Doppio registry dei metodi remoc/bridge [P2, verified; collegato a TD-012]

- **Evidenza:** `src/service/api/rpc.rs:70+` dichiara il trait con 63 operazioni; `src/service/api/bridge.rs:23-641,644-675` mappa manualmente stringhe, JSON arguments, encoding ed errori a metodi dello stesso trait. È un adapter al confine Tauri, **non** un HTTP router.
- **Costo:** modificare/aggiungere un metodo richiede toccare più punti e può provocare drift nei nomi/argomenti/shape UI.
- **Refactor:** prima test di corrispondenza operation names + schema/response; ridurre solo boilerplate *identico* dentro il bridge o valutarne la generazione da un catalogo controllato. Non creare nuova RPC dinamica, framework dispatch o protocollo.
- **Gate:** 63 operazioni, capability, debug-gated, stream/cancel, `confirmUnlock` 425, error envelope, Windows reconnect.

### AE-15 — Semantica versioni e precisione schedule [P2, candidate, **decision**]

- **Evidenza:** `src/core/common/domain_types.rs:69-99`: `SitePackVersion::is_newer_than` splitta i segmenti e ripiega sul confronto lessicografico non numerico; `src/manager/network/schedule.rs:120-155`: canonicalizzazione delle entries con `num_seconds_from_midnight()/60`; `src/core/common/schedule.rs:44-53,73-139` modella `NaiveTime` completo.
- **Rischio:** le semantiche di versionamento e arrotondamento temporale sono contratti di prodotto, non rumore sintattico; non affermare un bug in assenza di casi caratterizzati.
- **Refactor:** documentare contratti con test su versioni semver/bundle non confrontabili e secondi negli orari; solo poi semplificare parser/interval folding **a comportamento invariato**.
- **Gate:** round-trip DB, backup, midnight, precisione subminuto, DST e migrazioni storiche.

### AE-16 — Identità dei tipi a validazione incompleta [P2, verified, **decision**]

- **Evidenza:** `src/core/common/injectable_scripts.rs:24-42` `ScriptId(String)` ammette default stringa vuota; `src/core/network_block/site_pack/importer/mod.rs:43-58` `SitePackId(String)` ha `From/Deserialize` non fallibili; `RemoteSitePackId` controlla solo non-empty (`src/core/common/domain_types.rs:4-24`).
- **Refactor:** identificare l'invariante *effettivo* di ciascuna identità e l'authority del dato; valutare parser fallibili a ingress/persistence o validatore specifico di path quando serve. Non cambiare indiscriminatamente serializzazione o convertire ID canonici in UUID.
- **Gate:** payload remoc, manifest esterni, backup storici, DB rows, factory/test fixtures, error code.

### AE-17 — Download e progress ripetuti nell'importer [P2, candidate]

- **Evidenza:** `src/core/network_block/site_pack/importer/importer.rs:68-178` ripete costruzione client, status, controllo Content-Length/chunks, limiti, scrittura e progress; `src/core/utils/file_download.rs:52-124` ha già pipeline streaming limitata con yield delle percentuali.
- **Valutazione:** la duplicazione tecnica è reale, ma il reporting di progress multi-sorgente e la semantica d'errore non sono uguali; **non** estrarre un helper generico per il solo numero di righe.
- **Refactor:** fissare nei test la forma degli eventi e i limiti, poi riusare piccole primitive già presenti *solo se* stessi ownership e failure modes.
- **Gate:** molte sorgenti, cancellazione, retry, client disconnect, percentuali e tempfile cleanup.

### AE-18 — Setup pool e migrazioni duplicati [P2, candidate]

- **Evidenza:** `src/db.rs:36-77` e `src/core/network_block/site_pack/site_pack_db.rs:39-87` ripetono setup Diesel pool, ottenimento connessione, map error e `run_pending_migrations`.
- **Valutazione:** database principale e per-pack sono **domini di persistenza distinti** con migrazioni differenti. Una utility condivisa sarebbe giustificata solo per il codice infrastrutturale invariabile; non un generic repository.
- **Gate:** DB missing/readonly, pool exhaustion, migration failures, responsabilità separate, nessuna dipendenza invertita.

### AE-19 — Override globali delle directory test [P2, verified]

- **Evidenza:** `src/settings.rs:27-66,82-110`: `TEST_SITES_DIR`, `TEST_RESISTANCE_SCRIPTS_DIR`, `TEST_BUNDLED_PACKS_DIR` sono condivise/globali; `src/manager/network/backup.rs:374-393` deve usare una directory process-wide con `OnceLock` e `mem::forget` per evitare race.
- **Costo:** isolamento tra test difficoltoso e fixture globali/immortali; ampliare test parallelizzati può provocare collisioni (la codebase lo dichiara esplicitamente).
- **Refactor:** localizzare i path a un test context/injected settings **solo** in componenti con seam già disponibile; altrimenti test serializzati o fixture condivise esplicite. Non introdurre settings globali ulteriori.
- **Gate:** `cargo test -p ace-engine --lib -- --test-threads=1` e suite parallela con race/fixture clean up.

### AE-26 — Aggregazione stato con fallback non distinguibili [P2, candidate]

- **Evidenza:** `src/manager/network/network_blocks_to_site_pack.rs`: `count_synced_site_packs_of` rende 0 se una lettura fallisce e conta poison lock come false; `src/service/content_updater.rs` usa lista vuota se il DB degli ignored pack fallisce; `src/manager/network/backup.rs` usa lista vuota sul lookup dei blocchi. I fallback sono ora accompagnati da warning, senza alterare il comportamento esistente.
- **Costo:** “nessun dato” e “impossibile leggere” diventano indistinguibili, con effetti diversi tra visualizzazione, update e restore. Non generalizzare una singola strategia di fallimento a subsistemi diversi.
- **Refactor:** individuare ogni consumer e scegliere esiti fallibili/telemetria alla boundary corretta dove utile; test success/failure, senza cambiare lock/uninstall fallback.
- **Gate:** query fallite, stale view, ignored packs, lock state, regressioni su API errors.

## Ordine operativo dei problemi residui

1. **Decisioni su policy, compatibilità e recovery:** AE-01, AE-03, AE-04, AE-05, AE-11, AE-15, AE-16. Non modificare comportamenti fail-open/fail-closed o contratti persistiti senza definire prima la semantica.
2. **Concorrenza e orchestrazione:** AE-13 e AE-14. Servono characterization tests per single-flight, aggiornamenti concorrenti e correttezza del dispatch RPC.
3. **Isolamento e deduplicazione prudente:** AE-17, AE-18, AE-19. Prima verificare gli effetti su progress streaming, connessioni ai DB e fixture process-global.
4. **Error handling:** AE-26. La nuova diagnostica mantiene i fallback esistenti; resta da definire se e a quale boundary propagare gli errori.

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

**Baseline precedente (2026-10-22):** prima delle decisioni pre-release, 227/227 test engine superati; `cargo fmt --all --check` riportava difformità preesistenti in binari e sorgenti generati non modificati.

**Progresso implementazione (2026-10-22):** `cargo test -p ace-engine --lib -- --test-threads=1`: **238 superati, 0 falliti**. `cargo check -p ace-engine --all-targets --all-features`: passato; `cargo test -p ace-watchdog-protocol`: 3/3; `git diff --check`: passato. Il test aggiuntivo di restore su blocco locked ha portato i test mirati del backup a 8/8 dopo la suite completa. Aggiunti test di fault injection per restore di pack e blocchi (rollback per unità e sincronizzazione memoria/DB dopo un errore), commit scripts con errore SQLite, errori di membership site pack, ID/path traversal e versioni canoniche. `SitePack::from` propaga adesso l'errore di apertura DB. Nessun test su servizi installer/watchdog installati in VM Windows, recovery tramite crash reale, oppure review indipendente conclusiva. Il backlog generale rimane `docs/ai/TECH_DEBT_BACKLOG.md` (TD-001/002/003/004/006/012/014).
