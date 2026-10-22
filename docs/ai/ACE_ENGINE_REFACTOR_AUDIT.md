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
| AE-11 | P1 | partial | Script bundle activation | Rollback dopo errore DB implementato e testato; resta recovery da crash tra rename/DB e test VM Windows |
| AE-13 | P2 | verified | Content updater | Mutex servizio trattenuto nel download/staging/commit |
| AE-14 | P2 | verified | API bridge | Map dei nomi operazioni replicata rispetto al trait |
| AE-15 | P2 | partial | Schedule/minute semantics | Versionamento canonico già implementato; restano caratterizzazione di arrotondamento al minuto, mezzanotte, DST e round-trip |
| AE-17 | P2 | candidate | Site-pack importer | Ripetizione gestione download/progress |
| AE-19 | P2 | verified | Test | Override process-global delle directory di test |
| AE-26 | P2 | partial | Typed error propagation | Restano fallback ambigui nel caricamento Resistance Mode e della cronologia updater, e audit dei consumer non ancora trattati |

---

## Dettaglio interventi

### AE-11 — Recovery da crash durante l'attivazione degli script [P1, partial]

- **Stato attuale:** `src/manager/network/resistance_mode.rs` conserva l'albero precedente e ripristina filesystem/memoria quando `update_scripts_version` fallisce; il caso SQLite è coperto da fault injection.
- **Rischio residuo:** se il processo termina fra rinomina della directory, reload e commit della versione, il riavvio può osservare filesystem, memoria e versione DB incoerenti. Alcuni rollback delle rinomine sono ancora best-effort.
- **Prossimo refactor:** caratterizzare i singoli crash point e implementare riconciliazione idempotente all'avvio, mantenendo il catalogo last-known-good.
- **Gate:** crash/restart, errore file handles e rename Windows, DB/version consistency, attivazione e rollback su VM.

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

### AE-15 — Semantica degli schedule al minuto [P2, partial]

- **Stato attuale:** `SitePackVersion` usa il formato canonico `X.Y.Z` con confronto numerico, senza compatibilità con versioni legacy. Il versionamento non richiede ulteriori interventi in questa voce.
- **Evidenza residua:** `src/manager/network/schedule.rs` normalizza `NaiveTime` con `num_seconds_from_midnight() / 60`, mentre il dominio conserva secondi; è approvata la precisione **al minuto**.
- **Prossimo refactor:** testare e documentare in modo esplicito canonicalizzazione, input con secondi, merge dei periodi, mezzanotte e salvataggio/ricaricamento, senza introdurre precisione ai secondi.
- **Gate:** round-trip DB/backup, boundary 00:00 e 24:00, DST, lock durante schedule.

### AE-17 — Download e progress ripetuti nell'importer [P2, candidate]

- **Evidenza:** `src/core/network_block/site_pack/importer/importer.rs:68-178` ripete costruzione client, status, controllo Content-Length/chunks, limiti, scrittura e progress; `src/core/utils/file_download.rs:52-124` ha già pipeline streaming limitata con yield delle percentuali.
- **Valutazione:** la duplicazione tecnica è reale, ma il reporting di progress multi-sorgente e la semantica d'errore non sono uguali; **non** estrarre un helper generico per il solo numero di righe.
- **Refactor:** fissare nei test la forma degli eventi e i limiti, poi riusare piccole primitive già presenti *solo se* stessi ownership e failure modes.
- **Gate:** molte sorgenti, cancellazione, retry, client disconnect, percentuali e tempfile cleanup.

### AE-19 — Override globali delle directory test [P2, verified]

- **Evidenza:** `src/settings.rs:27-66,82-110`: `TEST_SITES_DIR`, `TEST_RESISTANCE_SCRIPTS_DIR`, `TEST_BUNDLED_PACKS_DIR` sono condivise/globali; `src/manager/network/backup.rs:374-393` deve usare una directory process-wide con `OnceLock` e `mem::forget` per evitare race.
- **Costo:** isolamento tra test difficoltoso e fixture globali/immortali; ampliare test parallelizzati può provocare collisioni (la codebase lo dichiara esplicitamente).
- **Refactor:** localizzare i path a un test context/injected settings **solo** in componenti con seam già disponibile; altrimenti test serializzati o fixture condivise esplicite. Non introdurre settings globali ulteriori.
- **Gate:** `cargo test -p ace-engine --lib -- --test-threads=1` e suite parallela con race/fixture clean up.

### AE-26 — Errori residui ancora codificati come valori validi [P2, partial]

- **Stato attuale:** dashboard, conteggi, lookup dei site pack, restore e update-check RPC propagano `Result` e distinguono errore, assenza del dato e valore vuoto.
- **Evidenza residua:** `src/manager/network/mod.rs::load_resistance_manager` ritorna un manager con intensità `None` se la lettura o il parsing falliscono; `src/service/content_updater.rs::load_latest_check` ignora fallimenti di lettura/parsing dell'ultimo check. Occorre esaminare gli ulteriori ingress e il motivo dei fallback.
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

**Ultimi gate registrati (2026-10-22):** `cargo test -p ace-engine --lib -- --test-threads=1`: **240/240** superati; `cargo check -p ace-engine --all-targets --all-features`: superato; `cargo test -p ace-watchdog-protocol`: **3/3**; `git diff --check`: superato. Non risultano conclusi test VM Windows su servizi/installer, recovery tramite crash reale e review indipendente. `cargo fmt --all --check` evidenziava difformità preesistenti in altri sorgenti non toccati. Per le decisioni di prodotto vedere `ACE_ENGINE_REFACTOR_DECISIONS.md`; per il debito generale `TECH_DEBT_BACKLOG.md`.
