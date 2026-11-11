<a id="ace-blocker-desktop--localizzazione-con-llm"></a>
# Ace Blocker Desktop — LLM localization workflow

This guide supplements `ace-blocker-site/docs/LLM_LOCALIZATION_WORKFLOW.md` when both repositories are available.
You can also use this guide without the website repository.

<a id="come-operare"></a>
## Procedure

1. Use `apps/ace-app/messages/en.json` as the source catalog. Inspect the actual screen and behavior before translation. Use complete features of 20–50 keys per batch, not isolated words.
2. Specify the target language and region, glossary, tone, and context. The app is an operational tool. Use natural Italian and direct sentences. Avoid moralizing, calques, sales language, and arbitrary synonyms.
3. Translate into JSON with an LLM. Keep keys, `{name}` / `{count}` / `{version}` variables, tags, URLs, formats, and facts unchanged. State product consequences and limits explicitly.
4. Use a second, **independent** LLM call to compare English, translated output, and the screen. Correct meaning, unnatural vocabulary, inconsistent terms, and truncation. Do not treat automatic back-translation as approval.
5. Obtain a native-speaker review. Review locks, deadlines, times and time zones, irreversible deletions, protection states, data, downloads, and updates. Check text in the app with real dimensions and representative data.
6. Save changes only to the requested `messages/{locale}.json` files. Do not reorder unrelated entries. Run `bun run i18n:audit` and `bun run check` from `apps/ace-app`. The audit **checks EN references, not locale coverage**. Run applicable tests. Record the model/version, date, reviewer, and unresolved keys.

<a id="glossario-minimo-da-approvare"></a>
## Minimum glossary for approval

- **Network Block:** A configuration that blocks sites or categories according to rules and times. Proposed Italian term: `blocco di rete`.
- **Lock / time-lock:** Protection against changes that weaken rules. It does **not** block additional sites. Proposed Italian terms: `blocco delle modifiche` / `protezione delle regole`, depending on context. Do not confuse Lock with Network Block.
- **Schedule:** The times when filters apply. Proposed Italian terms: `programmazione` / `orari programmati`.
- **Resistance Mode:** Measures that add friction on supported sites. It is not a Network Block. Agree on the translated name.
- **Category / Site Pack:** A list of sites. Downloading the list alone does not activate filtering. Distinguish categories from custom lists.
- **Ace Blocker / Pro:** Keep the commercial names unchanged.

The glossary is a **proposal**, not an approved translation.
Align it with website FAQ, UI, and checkout terminology before publication.
For `control_lock_help_detail`, avoid the literal “il blocco blocca...”.
Explain that Lock protects changes but does not expand the list of filtered sites.

<a id="stato-iniziale-verificato-2026-11-06"></a>
## Verified initial state (2026-11-06)

Paraglide configures `en`, `de`, `it`, `es`, `fr`, `pt`, and `pt-BR`.
The initial `messages/en.json` contained 720 keys, and `it.json` contained 13.
The other locales contained 0 keys. These counts are a snapshot, not verified UI coverage.
Do not mark a language complete only because it is declared in `project.inlang/settings.json`.

<a id="stato-dopo-la-prima-implementazione-2026-11-06"></a>
## State after the first implementation (2026-11-06)

After the initial snapshot, the app's English and Italian catalogs each contained 731 keys.
This includes accessibility labels and language controls.
Other locales were not expanded. The selector shows only Sistema, EN, and IT.
If the operating system uses an untranslated locale, the app falls back to English.
This does **not** certify native-speaker review or legal validation.
Complete an additional editorial review of locks, dates, and privacy.

<a id="checklist-finale"></a>
## Final checklist

- Verify valid JSON, unchanged keys and placeholders, and no invented feature claims.
- Check concise, idiomatic text on actual screens. Prevent critical truncation. Use locale-correct dates, numbers, and plurals.
- Obtain qualified review of high-risk strings and consistent website terminology.
- Verify compilation, checks, tests, and visual screenshots of principal screens.
- Include the unresolved-key list and updated glossary in the same PR.

The shared `.agents/skills/i18n-localization/SKILL.md` helps with *technical* localization: keys, placeholders, fallbacks, and RTL.
It does not replace this editorial workflow.
The codebase uses Paraglide with `m.nome_chiave()` and Bun.
Do not follow generic React/Next or npm examples from community skills.
