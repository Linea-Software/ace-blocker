# Needs validation — piano di verifica difensiva

Nessuna gravità formale è attribuita ai seguenti record: il contratto della skill riserva la severity ai finding con prova indipendente della violazione.

## AB-SA-01 — Dati segreti sotto ACL leggibili da Users

- **Trace:** `installer/installer.wxs:210-214` → `crates/ace-engine/src/settings.rs:93-100,144-165` → `crates/ace-engine/src/db/account_repo.rs:19-32`, `crates/mitm-proxy/src/ca.rs:199-216`, `settings.rs:168-189`.
- **Fatto accertato nel sorgente:** cartella SharedData configurata con lettura BU ereditabile; segreti persistiti senza ACL specifiche visibili.
- **Blocco:** non sono note le DACL finali e la presenza di ACE protette sulle copie reali; manca una VM Windows isolata.
- **Verifica locale sicura:** in VM Windows usa-e-getta con chiavi **fittizie**, installare MSI di test e leggere esclusivamente metadata DACL di directory, DB, capability e CA key con due account locali temporanei. Non leggere, copiare o esportare chiavi reali. Verificare che Users sia negato sulla CA key e sulla credenziale cloud, e che l'UI autorizzata continui a funzionare.
- **Verifica release owner:** test automatico ACL prima della pubblicazione, file creati ex novo e migrazione da installazioni precedenti. Confermare root CA trust e piano di rotazione.

## AB-SA-02 — IPC di lettura senza capability

- **Trace:** `crates/ace-engine/src/service/api/transport.rs:25-80` → `rpc.rs:438-447,822-831` → `manager/network/backup.rs:72-163`.
- **Fatto nel sorgente:** `export_backup` non richiede token, pipe AU o socket 0666 consentono connessioni locali.
- **Blocco:** non è documentata una policy di confidenzialità della configurazione tra utenti dello stesso computer e non è verificato il comportamento runtime dell'RPC.
- **Verifica locale sicura:** con dummy block nominato `dummy-private-block` e due utenti temporanei in VM, osservare soltanto se un processo senza capability ottiene la stringa dummy via `export_backup`. Non includere vere abitudini/elenchi.
- **Verifica release owner:** approvare formalmente il modello di lettura (per-installazione vs per-utente) e aggiungere test che corrispondano alla decisione.

## AB-SA-03 — Callback account non associata alla richiesta

- **Trace:** `crates/ace-engine/src/protocol.rs:11-20,76-153` → `crates/ace-engine/src/bin/ace_redirect.rs:38-72` → `crates/ace-engine/src/service/api/operations_account.rs:44-62`.
- **Fatto nel sorgente:** path allowlisted, ma nessuna verifica di nonce/state pendente nel client; il client inoltra la chiave presente nell'URL.
- **Blocco:** flusso esterno di sito/autenticazione non incluso nel repository, conferme browser/OS, e danno effettivo non dimostrati.
- **Verifica locale sicura:** in macchina/browser isolati, con due account sintetici e controllo esplicito dell'apertura app, osservare se un link non richiesto può cambiare il key binding senza un login iniziato nell'app. Nessun account reale.
- **Verifica release owner:** testare retry/replay e callback stantie; progettare correlazione monouso end-to-end, se la minaccia si conferma.

## Ulteriori aree non validate

Watchdog recovery e race, flussi proxy/tunnel non-browser, FFI e policy WFP, integrità dei manifest update, dependency CVEs, MSI rollback/upgrade e permessi di service. Effettuare i test di runtime solo con un sandbox OS-enforced completo (network esterno negato, env allowlist, filesystem isolato, limiti risorse).
