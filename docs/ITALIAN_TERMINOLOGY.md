<a id="ace-blocker--glossario-italiano-e-controllo-editoriale"></a>
# Ace Blocker — Italian terminology and editorial review

**Review date:** November 6, 2026. **Locale:** Italian (Italy). The EN/IT comparison used two passes: fidelity and editorial quality.
This review is not legal advice or approval by a human reviewer.

| English concept | Italian choice | Meaning and use |
| --- | --- | --- |
| Network Block | **blocco di rete** | Configuration that links sites and categories to enforcement times. Use «Blocco» when the context is clear. |
| Lock / time-lock | **protezione delle regole**, **protezione delle modifiche**, **protezione a tempo** | Prevents changes that weaken rules while active. It does not add sites or extend filtering times. For actions use «Proteggi»; for status use «Modifiche bloccate». |
| Schedule | **programmazione**, **orari programmati** | Days and periods when a Network Block enforces rules. Without schedules, the filter does not apply. |
| Resistance Mode | **modalità Resistenza**; **Resistenza** nella navigazione | Pro feature that changes supported site experiences, such as feeds, videos, interfaces, and delays. It is not a Network Block. |
| Categories | **categorie predefinite**, **categorie** | Lists supplied by Ace. Some are Free; the full catalog requires Pro. «Selezionate» can describe editorial selection. |
| Custom Lists | **liste personalizzate** | User-imported domain lists. This is a Pro feature. |
| Downloaded vs enabled | **scaricata** vs **collegata al blocco** | Downloading a list makes its data available but does not activate blocking. Add it to a Network Block with active hours. |
| Free / Pro | **Free**, **Pro** | Keep these product names. Free includes basic blocking. Pro includes Resistance, all categories, custom lists, and cloud backup. |
| Recurring payment | **pagamento ricorrente** | Do not use «prezzo ricorrente». Distinguish charges, renewal, price, and cancellation. |
| Authenticated product release | **versione verificata** | Avoid «release autenticate». A verified release is not necessarily installable. |
| Settings | **Impostazioni** | Use this term consistently in section names, navigation, tooltips, and instructions. |
| Private by Design | **Privacy fin dalla progettazione** | Privacy is built into product design. It is not an optional user choice. |

<a id="plurali-dinamici"></a>
## Dynamic plurals

For counters that require grammatical agreement, select complete phrases for **0, 1, and >1**.
Use `countMessage` or explicit branches for trial-day counts.
Examples: «Nessun blocco», «1 blocco», «2 blocchi»; «Nessuna visita», «1 visita», «2 visite».
Do not invent plurals for invariable terms such as «record» and «account».
Keep existing keys and placeholders unchanged.
Add new keys only for the necessary variants.

<a id="da-verificare-prima-della-pubblicazione"></a>
## Checks required before publication

- **Independent legal review:** Verify Pro usage-data claims, optional error reports, backup and encryption, third parties, retention, consent to changes, liability limits, and access suspension. The Italian translation must not change the substantive English terms.
- **English sources that need clarification:** On the website, `faq_a1` and `home_what_is_description` can imply that locks keep filters active outside scheduled hours. In the app, `category_apply_note` incorrectly describes a list as «bloccata» instead of its included sites. `onboarding_resistance_toggle_hint` promises intensity changes at any time, even under Lock. `onboarding_welcome_body` does not state that Resistance requires Pro. `tour_blocked_attempts_body` says «sites blocked», although the metric counts attempts. Review the English sources separately. Do not silently correct English claims only in Italian.
- **Claims to confirm again:** Review the FAQ and privacy claims, supported platforms, setup time («meno di un minuto»), category availability, and updates. The editorial audit does not constitute legal or commercial verification.

<a id="verifica"></a>
## Verification

The Italian catalog was compared in full with the English catalog.
Preserve URLs, keys, parameters, and commercial references in each change.
Run Paraglide compilation, checks, and tests.
Inspect the text visually on the actual screens.
Do not change correct phrases based only on word preference.
