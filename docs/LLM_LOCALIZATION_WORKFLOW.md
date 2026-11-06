# Ace Blocker Desktop — localizzazione con LLM

Questa guida integra il playbook condiviso `ace-blocker-site/docs/LLM_LOCALIZATION_WORKFLOW.md` quando entrambi i repository sono aperti nello stesso workspace. Rimane utilizzabile anche quando il sito non è disponibile.

## Come operare

1. Considerare `apps/ace-app/messages/en.json` la fonte di partenza; prima di tradurre leggere la schermata e il comportamento reale. Per ogni batch usare una feature completa (20–50 chiavi), non parole isolate.
2. Definire lingua e variante regionale, glossario, tono e contesto. L'app è uno strumento operativo: italiano naturale, frasi dirette, niente moralismi, calchi, enfasi commerciale o sinonimi arbitrari.
3. Tradurre con LLM in JSON mantenendo identiche chiavi, variabili `{name}` / `{count}` / `{version}`, tag, URL, formati e fatti. Mantenere esplicite conseguenze e limitazioni del prodotto.
4. Usare una seconda chiamata LLM **indipendente** per confrontare EN, output e schermata; correggere significato errato, lessico non idiomatico, incoerenze e troncamenti. Non trattare la retrotraduzione automatica come approvazione.
5. Revisionare con un madrelingua; controllo obbligatorio per lock, scadenze, orari e fusi, rimozioni irreversibili, stati di protezione, dati, download e aggiornamenti. Convalidare testo in app con dati e dimensioni reali.
6. Salvare soltanto nei `messages/{locale}.json` richiesti, senza riordinare inutilmente il resto. Eseguire dalla directory `apps/ace-app` i comandi `bun run i18n:audit` (**valida riferimenti EN, non copertura del locale**) e `bun run check`, oltre ai test applicabili. Annotare modello/versione, data, autore revisione e chiavi rimaste dubbie.

## Glossario minimo da approvare

- **Network Block**: configurazione di blocco dei siti o categorie secondo regole e orari; candidato IT `blocco di rete`.
- **Lock / time-lock**: protezione che impedisce di indebolire le regole; **non** blocca siti aggiuntivi. Candidati IT `blocco delle modifiche` / `protezione delle regole` a seconda del contesto; non confonderlo con Network Block.
- **Schedule**: quando vengono applicati i filtri; candidati IT `programmazione` / `orari programmati`.
- **Resistance Mode**: misure che introducono attrito sui siti supportati; non è un Network Block. Nome tradotto da concordare.
- **Category / Site Pack**: elenco di siti; scaricarlo da solo non attiva il filtro. Distinguere dalle liste personalizzate.
- **Ace Blocker / Pro**: mantenere i nomi commerciali.

Il glossario è una **proposta**, non una traduzione già approvata. Uniformarlo a quello delle FAQ, della UI e del checkout del sito prima della pubblicazione. Un esempio critico in `control_lock_help_detail`: evitare il letterale “il blocco blocca...”; esprimere chiaramente che il vincolo protegge le modifiche ma non amplia la lista dei siti filtrati.

## Stato iniziale verificato (2026-11-06)

La configurazione Paraglide ha `en`, `de`, `it`, `es`, `fr`, `pt` e `pt-BR`. In `messages/en.json` ci sono 720 chiavi; `it.json` ne contiene 13 e gli altri locali 0. Il conteggio è uno snapshot, non copertura di UI. Evitare di considerare le lingue già pronte solo perché dichiarate in `project.inlang/settings.json`.

## Checklist finale

- JSON valido, chiavi e placeholder conservati; nessuna affermazione funzionale inventata.
- Frasi brevi e idiomatiche viste nello schermo reale; niente truncation critica, date/numeri/plurali appropriati al locale.
- Revisione competente per stringhe ad alto rischio e terminologia coerente col sito.
- Compilazione e controlli superati, screenshot/verifica visuale delle schermate principali.
- Lista delle chiavi da rivedere e glossario aggiornati insieme alla PR.

La skill condivisa `.agents/skills/i18n-localization/SKILL.md` è utile per la parte _tecnica_ (chiavi, placeholder, fallback, RTL), non sostituisce questa guida editoriale. In particolare il codebase usa Paraglide con `m.nome_chiave()` e Bun, non gli esempi generici React/Next o npm delle skill comunitarie.
