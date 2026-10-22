# Ace Engine — audit del refactoring e backlog dedicato

**Data:** 2026-10-22
**Repository:** `C:\Projects\ace-project\ace-blocker` (`main`, audit senza modifiche al codice)
**Ambito:** `crates/ace-engine/` (core, manager, DB, servizi, API/RPC, proxy adapter, lifecycle, CLI, test/documentazione). Dipendenze `ace-domain-blocklist`, `ace-tun`, `mitm-proxy` e `ace-watchdog` considerate *solo* nei contratti e nei punti di integrazione dell'engine; **Ace App esclusa**.
**Procedura:** `AGENTS.md`, `crates/ace-engine/AGENTS.md`, `docs/ai/{REFACTORING_PLAYBOOK,BOUNDARIES_AND_ABSTRACTIONS,DOMAIN_TYPES,REVIEW_GATES}.md`; confronto con `TECH_DEBT_BACKLOG.md` e `docs/IPC_ARCHITECTURE.md`.

> **Nota:** questo è un inventario di opportunità di refactoring, **non una lista di fix già applicati**. La ricognizione ha coperto l'albero sorgente e i principali flussi per sottosistema, con letture mirate dei call site e ricerche statiche; non costituisce una review riga-per-riga di ogni sorgente generato/migrazione/test. `verified` significa che la struttura o il comportamento descritto è direttamente visibile nei file; **non** significa che un bug sia riprodotto in runtime. `candidate` richiede ulteriori verifiche; `decision` richiede approvazione della semantica prima di qualsiasi modifica. Priorità riferite al rischio e al costo di manutenzione, non a line count.

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
| Utility, test e generazione | `src/bin/*`, `build.rs`, `tests/api-harness`, `docs/rest-api.md`, `Cargo.toml` | Artefatti legacy, tool/dev, copertura e documentazione |

## Queue prioritaria

| ID | Priorità | Stato | Area | Motivo / prossima azione |
| --- | --- | --- | --- | --- |
| AE-01 | P0, decision | candidate | Startup/enforcement | Valutare fallback del manager vuoto dopo errore di caricamento |
| AE-02 | P1 | verified | Capability comparison | Rivedere la pretesa di confronto constant-time |
| AE-03 | P1, decision | candidate | Site-pack paths/ID | Validare gli ID prima di derivarne percorsi filesystem |
| AE-04 | P1, decision | candidate | Site-pack DB errors | Evitare di confondere errore SQLite e `NotBlocked` |
| AE-05 | P1 | verified | Backup restore | All-or-nothing / errori ignorati |
| AE-06 | P1 | verified | Site-pack assignment | DB e memoria aggiornati nell'ordine sbagliato (TD-014) |
| AE-07 | P1 | verified | Site-pack creation/import | Memoria inserita prima del DB; fallimenti sorgenti ignorati |
| AE-08 | P1 | verified | Resistance state | Modifiche in memoria prima di scritture fallibili |
| AE-09 | P1 | verified | Download/integrità | Limite bytes controllato dopo `response.bytes()` |
| AE-10 | P1 | verified | Account API | Manager read lock detenuto durante possibile I/O remoto |
| AE-11 | P1, decision | candidate | Backup/scripts activation | Stato filesystem/DB durante errori di commit |
| AE-12 | P2 | verified | Site-pack read APIs | `write().await` / `&mut self` per sole letture |
| AE-13 | P2 | verified | Content updater | Mutex servizio trattenuto nel download/staging/commit |
| AE-14 | P2 | verified | API bridge | Map dei nomi operazioni replicata rispetto al trait |
| AE-15 | P2, decision | candidate | Version/clock | Comparazione versioni e schedule rounding da caratterizzare |
| AE-16 | P2 | verified | Domain ID | `ScriptId` e `SitePackId` non impongono invarianti propri |
| AE-17 | P2 | candidate | Site-pack importer | Ripetizione gestione download/progress |
| AE-18 | P2 | candidate | DB pool | Duplicazione infrastrutturale connessione/migrazione |
| AE-19 | P2 | verified | Test | Override process-global delle directory di test |
| AE-20 | P3 | verified | Legacy test harness | Test REST/porta locali incompatibili con remoc |
| AE-21 | P3 | verified | Legacy API alias | Alias `NetworkMangerOpErr` senza call site rilevati |
| AE-22 | P3 | verified | Paths | Due helper equivalenti per directory temporanee |
| AE-23 | P3 | verified | Docs/comments | Riferimenti SSE/REST e URL obsolete |
| AE-24 | P3 | candidate | Generated code/tooling | Tooling OpenAPI e dependency scope: audit, non patch generati |
| AE-25 | P3 | candidate | Account DTO | Due letture di account con corpo pressoché identico |
| AE-26 | P2 | candidate | Lock/errors | Alcuni errori vengono silenziati durante aggregazione stato |

---

## Dettaglio interventi

### AE-01 — Fallback al manager vuoto [P0, candidate, **decision**]

- **Evidenza:** `src/lib.rs:286-301`: quando `NetworkManager::from` fallisce, viene eseguito `NetworkManager::new_empty` e l'engine prosegue; `src/manager/network/mod.rs:131-144` inizializza mappe vuote. **Rischio dedotto:** policy persistite non caricate potrebbero non essere applicate. Non è stato riprodotto un caso di avvio reale.
- **Refactor proponibile:** modellare esplicitamente esito e motivi del bootstrap, separando “configurazione vuota valida” da “configurazione non caricabile”; definire l'azione corretta solo dopo decisione sulla policy fail-safe. **Non introdurre un fail-closed nuovo in un refactor meccanico.**
- **Gate:** test su DB corrotto, lock persistiti, dati mancanti, recovery, avvio servizi su VM Windows/Linux; considerare effetti su uninstall/restart.

### AE-02 — Confronto capability dichiarato constant-time [P1, verified]

- **Evidenza:** `src/service/api/security.rs:3-9`: SHA-256 dei due token, conversione in hex, confronto `presented_digest == expected_digest`, descritto nel commento come constant-time; `src/service/api/rpc.rs:359-371` lo usa per richieste protette.
- **Costo/rischio:** il confronto ordinario di `String` **non fornisce garanzia esplicita di timing costante**. La valutazione dell'effettiva sfruttabilità è separata. I digest hanno lunghezza costante, ma questo da solo non prova la promessa del commento.
- **Refactor:** confrontare digest byte-wise con primitiva constant-time già disponibile/verificata nella dependency graph, senza modificare token, persistenza, algoritmo o esiti; oppure correggere la documentazione solo se approvata la garanzia reale.
- **Gate:** unit test equal/mismatch e regressione di accesso autorizzato/non autorizzato su remoc/Named Pipe; nessuna modifica ad ACL o al caso capability assente.

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

### AE-05 — Restore backup non atomico [P1, verified]

- **Evidenza:** `src/manager/network/backup.rs:171-362`: per ogni pack/block effettua scritture SQL separate; `get_all_network_blocks().unwrap_or_default()` (255-261); elimina prima di ricreare un block (261-265); ignora errori di associazione pack e script (`let _ = ...`, 345-355); ricostruisce memoria solo al termine (357-360).
- **Costo:** un errore intermedio può lasciare restore parziale e memoria divergente fino al reload; alcuni errori passano inosservati. Sono osservazioni del flusso di codice, non fault injection già eseguita.
- **Refactor:** prima test con failure injection su ogni fase e una definizione del comportamento del restore (best effort vs atomico); poi transazioni **per unità indipendente** o staging con rollback coerente, senza cancellare pack bloccati né cambiare schema.
- **Gate:** backup vecchi, Pro/free, ID catalogo, blocchi locked, errore SQLite, referential integrity, cloud restore, reinizializzazione.

### AE-06 — Associazioni site-pack memoria prima di DB [P1, verified; **TD-014**]

- **Evidenza:** `src/manager/network/network_blocks_to_site_pack.rs:71-84,100-116`: `add_site_pack/remove_site_pack` prima di `db.add_.../remove_...`.
- **Refactor:** validazione delle precondizioni prima, DB persistito prima dell'aggiornamento in memoria, oppure staging atomico; preservare `NetworkBlockNotFound`, lock refusal, errori e idempotenza.
- **Gate:** failure injection attach e detach, lock, pack unsynced, cache invalidation; è un task autonomo già registrato nel backlog generale.

### AE-07 — Creazione pack e source errors [P1, verified]

- **Evidenza:** `src/manager/network/site_pack.rs:357-409`: `self.site_packs.insert` precede `db.add_site_pack`; `import_site_pack` in `:412-458` registra sorgenti con `let _ = ...map_err(log)` (434-445), poi procede comunque all'import.
- **Costo:** possibile presenza di pack “fantasma” in memoria in caso di errore DB e metadati sorgente incompleti; il parser potrebbe usare sorgenti non tutte registrate.
- **Refactor:** creare/persistire metadati con ordering affidabile, propagare o raccogliere esplicitamente errori per sorgente; mantenere progress events, lock Pro, cancellazione e staging di TD-002.
- **Gate:** insert SQL fallito, sorgente n invalida, cancellazione, directory già esistente, riavvio ed eventuali side effects su DB staging.

### AE-08 — Modifiche Resistance Mode prima del commit [P1, verified]

- **Evidenza:** `src/manager/network/resistance_mode.rs:95-110` imposta `script_tree.enabled` prima di `db.set_script_state`; `:143-154,157-190,192-213` modifica `enabled`, lock e intensity prima di `save_resistance_mode_settings()` fallibile.
- **Costo:** possibilità di policy live diversa da SQLite dopo un write failure; particolarmente sensibile con lock attivi.
- **Refactor:** prevalidare e persistere gli aggiornamenti coerenti prima del publish in memoria, uno per setter, preservando lock strictness e transizioni consentite. Non unificare stati diversi in un mutatore generico.
- **Gate:** test fallimento SQLite per script switch, master switch, lock e intensity; riavvio e test su `UnsupportedLockState`.

### AE-09 — Download potenzialmente non limitato durante il body read [P1, verified]

- **Evidenza:** `src/core/utils/file_download.rs:132-175` in `download_and_verify` verifica `Content-Length` quando presente, poi usa `response.bytes().await` e confronta `bytes.len()` con `max_bytes` **solo dopo** l'acquisizione completa; la funzione `download_file_with_limit` (61-124) controlla ogni chunk.
- **Costo:** un server senza length o con length non corretta può richiedere memoria oltre il limite dichiarato prima del rifiuto.
- **Refactor:** lettura incrementale limitata con SHA streaming e stesso contratto `Bytes` solo entro soglia; in alternativa spool a file se il consumer lo permette, ma senza cambiare errori/timeout/retry arbitrariamente.
- **Gate:** HTTP chunked oltre cap, length falsa, checksum errato, empty response, retries e progress, verifica RAM/peak-memory.

### AE-10 — Remote account verification sotto network-manager read lock [P1, verified]

- **Evidenza:** `src/service/api/operations_account.rs:9-35` acquisisce `state.network_manager.read().await` e invoca `manager.is_pro_user().await`; `src/manager/account.rs:63-145,167-213` può effettuare HTTP remoto e scrivere SQLite; pattern analogo in `src/service/api/operations_backup.rs:14-18` e `operations_site_pack.rs:145-155` (qui write lock).
- **Costo:** un controllo licenza potenzialmente lento trattiene il lock di policy e ostacola modifiche; non dedurre rallentamenti misurati in produzione.
- **Refactor:** usare il già posseduto `AppState.account_manager` per verifica Pro fuori dal lock policy, poi riacquisire il manager solo per operazioni; assicurare che debug policy/Pro override non cambino semantica.
- **Gate:** account stale vs fresh, update concorrente, Pro in debug, mutazioni simultanee, latenza con mock server lento e permessi.

### AE-11 — Commit script bundle filesystem + DB [P1, candidate, **decision**]

- **Evidenza:** `src/manager/network/resistance_mode.rs:223-280` sposta directories e ricarica l'albero prima della scrittura `db.update_scripts_version` (265), che può fallire; molte rename/cleanup di rollback sono best-effort. È intenzionalmente separato dai normali setters.
- **Costo/rischio:** un fallimento DB dopo uno swap riuscito può produrre stato disk/mem/version non coordinato; il comportamento preciso post-restart non è stato caratterizzato.
- **Refactor:** test dei crash point e delle scritture fallite, pianificare journal/rollback solo se richiesti, preservare last-known-good e script catalog verificato.
- **Gate:** Windows rename e file handles, versioni vecchie, hash, retry, riavvio su VM.

### AE-12 — Letture site-pack trattate come scritture [P2, verified]

- **Evidenza:** `src/manager/network/site_pack.rs:343-355` (`get_all_site_packs(&mut self)` senza mutazioni), `:693-704` (`get_site_pack_sources(&mut self)`); `src/service/api/operations_site_pack.rs:7-37,129-143` apre `network_manager.write().await` per questi read endpoints.
- **Refactor:** restringere gli accessor a `&self` ove dimostrato e usare `read().await`; rilasciare i `std::sync::RwLockReadGuard` dei singoli pack prima di richiedere altri dati se necessario.
- **Gate:** test paralleli get/list e add/delete/resync, lock-order, performance solo se misurata. Collegabile al **TD-006** senza dichiararlo concluso.

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

### AE-16 — Identità dei tipi a validazione incompleta [P2, verified]

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

### AE-20 — Harness API REST non funzionante rispetto all'IPC attuale [P3, verified]

- **Evidenza:** `tests/api-harness/harness.ts:1-28,68-155` documenta e usa REST locale, porta 10567/port.lock, HTTPS localhost e fetch; il contratto engine attuale è remoc su Named Pipe/Unix socket (`docs/IPC_ARCHITECTURE.md:3-25`).
- **Costo:** test manuali/LLM suggeriti dalla repo mirano a un endpoint rimosso e forniscono falsi segnali.
- **Refactor:** verificare riferimenti in CI/docs e poi archiviare/eliminare l'harness obsoleto oppure sostituirlo con test nativi remoc. **Non** introdurre listener HTTP per mantenerlo.
- **Gate:** test smoke IPC concorrenti, auth e release, script/dev docs.

### AE-21 — Alias misspelled legacy non usato [P3, verified]

- **Evidenza:** `src/manager/network/error.rs:58-60` esporta `NetworkMangerOpErr = NetworkManagerOpErr` con commento “Temporary alias”; ricerca simbolo nell'albero `src` trova solo la dichiarazione.
- **Refactor:** controllare consumer esterni/integration tests e rimuovere l'alias se nessun uso; non cambiare `NetworkManagerOpErr`.
- **Gate:** `cargo check -p ace-engine --all-targets`, ricerca su tutto workspace e confronto API pubbliche.

### AE-22 — Stessa utility temporary sites directory in due owner [P3, verified]

- **Evidenza:** `src/settings.rs:118-130` e `src/core/network_block/site_pack/mod.rs:41-53` costruiscono entrambe `std::env::temp_dir()/AceBlocker/temp_sites` e creano directories.
- **Refactor:** verificare consumers e mantenere un solo punto owner, preferibilmente il path provider `Settings`; nessuna nuova utility crate.
- **Gate:** percorsi Windows/Linux, test con override e errori create_dir.

### AE-23 — Commenti/documentazione dell'API REST ormai superati [P3, verified]

- **Evidenza:** `src/manager/network/site_pack.rs:38-42,252-255` chiama ancora “SSE response body/reconnect” il stream guard; `src/service/content_updater.rs:698-701` documenta `POST /dev/update-check-interval`; `src/platform.rs:53-56,150-155`, `src/protocol.rs:13-21`, `src/settings.rs:159-164` conservano descrizioni HTTP/porta interna.
- **Refactor:** aggiornare *solo testo/commenti* per riflettere remoc, Tauri Channels e product behavior reale, preservando citazioni storiche pertinenti ed esplicite.
- **Gate:** review documentazione, nessun cambiamento binario.

### AE-24 — Tooling/generator dependencies da rivalutare [P3, candidate]

- **Evidenza:** `Cargo.toml` include `crates_llms_txt` come dependency ordinaria, ma la ricerca dei call site Rust la colloca in `src/bin/doc_gen.rs` (bin `dev-tools`); `openapi-to-rust` appartiene a generazione client esterno; `build.rs:1-22` dichiara che non si genera nulla al compile normale.
- **Refactor:** mappare per feature i bin, valutare spostare dipendenze esclusivamente di sviluppo su feature gate e rimuovere quelle effettivamente orfane solo dopo `cargo tree` e controlli `--all-targets --all-features`. Non toccare `src/ace_server_api/*` generati.
- **Gate:** `cargo check -p ace-engine --all-targets --all-features`, dev binaries, release, generate-api command, Windows resources.

### AE-25 — Duplice risposta account API [P3, candidate]

- **Evidenza:** `src/service/api/operations_account.rs:9-35` le funzioni `get_account_info` e `refresh_account` hanno oggi corpi identici; `src/manager/account.rs:159-161` `refresh_pro_status` delega a `is_pro`.
- **Refactor:** possibile funzione privata condivisa se le semantiche dei due endpoint continuano a coincidere; in alternativa documentare la differenza contrattuale prevista e lasciare duplicazione locale.
- **Gate:** `getAccountInfo` lettura anonima, `refreshAccount` con capability (diverse in `rpc.rs:375-399`), scadenza caching ed errore server.

### AE-26 — Aggregazione stato che silenzia fallimenti [P2, candidate]

- **Evidenza:** `src/manager/network/network_blocks_to_site_pack.rs:27-40`: `count_synced_site_packs_of` rende 0 se `get_network_block_to_site_packs` fallisce e conta poison lock come false; `src/service/content_updater.rs:505-512` `get_ignored_remote_site_packs().unwrap_or_default()`; `src/manager/network/backup.rs:254-263` suppress di errori sul lookup blocchi.
- **Costo:** “nessun dato” e “impossibile leggere” diventano indistinguibili, con effetti diversi tra visualizzazione, update e restore. Non generalizzare una singola strategia di fallimento a subsistemi diversi.
- **Refactor:** individuare ogni consumer e scegliere esiti fallibili/telemetria alla boundary corretta dove utile; test success/failure, senza cambiare lock/uninstall fallback.
- **Gate:** query fallite, stale view, ignored packs, lock state, regressioni su API errors.

## Ordine operativo proposto (interventi *separati*)

1. **Review di sicurezza/decisione:** AE-01, AE-03, AE-04 e AE-02. Non trattare la semantica fail-open/fail-closed come refactoring invisibile. Registrare ADR/test requirements prima di cambiare comportamento.
2. **Coerenza stato/DB:** AE-06 (già TD-014), AE-07 e AE-08: un mutatore per commit, test di fallimento SQL *prima* delle modifiche, revisioni indipendenti; AE-05 e AE-11 richiedono una progettazione più articolata.
3. **Resource limits e lock:** AE-09 (test del caso chunked senza content-length), AE-10, AE-12 e AE-13; prestare attenzione alle garanzie single-flight e ai cambi di Pro policy.
4. **Semplificazioni immediate e basso rischio:** AE-21, AE-22, AE-23 e AE-20 (quest'ultimo solo previa verifica del suo uso), ciascuno con commit dedicato.
5. **Consolidamento condizionato:** AE-14–19 e AE-24–26 soltanto dopo characterization tests e review dei confini; non sono mandato per nuove crate, trait universali o riscritture massive.

## Matrice di verifica da applicare *prima* di implementare

| Dominio | Test minimi |
| --- | --- |
| Policy/lock/DB | `cargo test -p ace-engine --lib -- --test-threads=1`, test ad hoc DB write-failure, round-trip reboot, locked/unlocked |
| RPC/security | `cargo test -p ace-engine --lib` (remoc), Named Pipe DACL/capability in VM Windows, release debug-gates |
| Import/backup | Staging/cancel, backup storico, SQLite error injection, checksum/zip bomb, path traversal, retry |
| Proxy/sitepack | Test su `service::proxy`, HTTP/HTTPS tunnel, DOM/script inject, non-browser non decrypt, cache invalidation |
| Watchdog/installer | `cargo test -p ace-watchdog-protocol` e VM SCM/recovery/maintenance/uninstall gate |
| Rust compile | `cargo fmt --all --check` (solo lettura), `cargo check -p ace-engine --all-targets`, lint mirato senza autofix |
| Audit/documentazione | Diff review architettura + dominio/tipi + correttezza **indipendenti**, percorsi e gate aggiornati |

**Risultato dell'audit:** soltanto documento; nessun sorgente, contratto, file generato, modulo Ace App o installer modificato. Nessun test di VM/anti-tampering effettuato in questa fase di ricognizione. Le evidenze statiche servono per creare test prima dei refactor; non costituiscono garanzia di comportamento reale. **Riferimento di backlog generale:** `docs/ai/TECH_DEBT_BACKLOG.md` (TD-001/002/003/004/006/012/014; non duplicare implementazioni concluse).
