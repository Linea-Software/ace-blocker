# Ace Blocker — glossario italiano e controllo editoriale

Revisione: 6 novembre 2026 · Italiano (Italia) · Confronto EN/IT in due passaggi (fedeltà, qualità editoriale). Non è un parere legale né un'approvazione da parte di un revisore umano.

| Concetto EN | Scelta IT | Significato e impiego |
| --- | --- | --- |
| Network Block | **blocco di rete** | Configurazione che collega siti e categorie a orari di applicazione. «Blocco» va bene se il contesto è chiaro. |
| Lock / time-lock | **protezione delle regole**, **protezione delle modifiche**, **protezione a tempo** | Impedisce modifiche che indebolirebbero le regole mentre è attivo; non aggiunge siti né prolunga gli orari del filtro. Per le azioni: «Proteggi»; per lo stato: «Modifiche bloccate». |
| Schedule | **programmazione**, **orari programmati** | Giorni e intervalli in cui il blocco di rete applica le regole. Senza orari, il filtro non si applica. |
| Resistance Mode | **modalità Resistenza**; **Resistenza** nella navigazione | Funzione Pro che modifica l'esperienza dei siti supportati (feed, video, interfaccia, ritardi), senza equivalere al blocco di rete. |
| Categories | **categorie predefinite**, **categorie** | Elenchi offerti da Ace, alcuni Free; il catalogo completo è Pro. «Selezionate» può descriverne la cura editoriale. |
| Custom Lists | **liste personalizzate** | Elenchi di domini importati dall'utente, funzione Pro. |
| Downloaded vs enabled | **scaricata** vs **collegata al blocco** | Scaricare una lista rende disponibili i dati, ma non avvia il blocco: va aggiunta a un blocco di rete con orari attivi. |
| Free / Pro | **Free**, **Pro** | Nomi invariati. Blocco essenziale Free; Pro include Resistenza, tutte le categorie, liste personalizzate e backup nel cloud. |
| Recurring payment | **pagamento ricorrente** | Non «prezzo ricorrente». Distinguere addebito, rinnovo, prezzo e disdetta. |
| Authenticated product release | **versione verificata** | Evitare «release autenticate». Una versione verificata non è necessariamente installabile. |
| Settings | **Impostazioni** | Nome unico per sezione, navigazione, tooltip e istruzioni. |
| Private by Design | **Privacy fin dalla progettazione** | Privacy integrata nella concezione del prodotto, non scelta opzionale dell'utente. |

## Plurali dinamici

Per i contatori con accordo grammaticale, selezionare frasi complete per **0, 1 e >1** tramite `countMessage` o rami espliciti (per i giorni di prova). Esempi: «Nessun blocco», «1 blocco», «2 blocchi»; «Nessuna visita», «1 visita», «2 visite». Non introdurre plurali artificiali per termini invariabili come «record» e «account». Le chiavi e i placeholder originari restano invariati; nuove chiavi sono dedicate esclusivamente alle varianti necessarie.

## Da verificare prima della pubblicazione

- **Revisione legale indipendente:** affermazioni su dati di utilizzo Pro, segnalazioni di errori facoltative, backup e cifratura, terze parti, conservazione, consenso alle modifiche, limiti di responsabilità e sospensione dell'accesso. La traduzione non modifica le condizioni sostanziali dell'inglese.
- **Originali EN da chiarire:** nel sito `faq_a1` e `home_what_is_description` possono far pensare che i lock mantengano i filtri attivi fuori orario; nell'app `category_apply_note` parla impropriamente di una lista «bloccata» anziché dei siti inclusi; `onboarding_resistance_toggle_hint` promette modifiche d'intensità in qualsiasi momento anche in presenza di lock; `onboarding_welcome_body` non esplicita il requisito Pro per Resistenza; `tour_blocked_attempts_body` descrive «sites blocked» benché la metrica sia un conteggio di tentativi. Non correggere di nascosto l'inglese intervenendo solo sull'italiano: serve una revisione separata delle sorgenti.
- **Claim da riconfermare:** FAQ e privacy, piattaforme supportate, rapidità di configurazione («meno di un minuto»), disponibilità categorie e aggiornamenti. L'audit editoriale non equivale a verifica legale o commerciale.

## Verifica

Il catalogo italiano è confrontato integralmente con quello inglese. Per ogni modifica preservare URL, chiavi, parametri e riferimenti commerciali; eseguire compilazione Paraglide, check, test e controllo visuale nelle schermate effettive. Non cambiare frasi già corrette soltanto per preferenza lessicale.
