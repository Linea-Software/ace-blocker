# German localization (de-DE): terminology and editorial decisions

Updated: November 6, 2026. Source: `messages/en.json`.
Use Italian translations only to help understand the source.

| Product term (EN) | Preferred German terms | Use and distinction |
| --- | --- | --- |
| Network Block | **Netzwerksperre** | A rule set for websites and Categories with its own schedule. UI strings: „Netzwerksperre erstellen“, „Diese Netzwerksperre greift gerade“. |
| Lock / time lock | **Änderungsschutz**; **zeitgebundener Änderungsschutz** | Protects a Network Block or Resistance settings against disabling or weakening. It does **not** block more websites. UI strings: „Änderungsschutz einrichten“, „Schutzfrist verlängern“. |
| Reflection Lock | **Reflexionsschutz** | A lock that users can release after answering a question and waiting five minutes. UI string: „Reflexionsschutz aktivieren“. |
| Schedule | **Zeitplan**; **Sperrzeiten** | Sets **when** the rules apply. UI strings: „Zeitplan festlegen“, „Ohne Zeitplan wird nichts gesperrt“. Use Montag–Freitag, not the ambiguous „werktags“. |
| Resistance Mode | **Widerstandsmodus**; short form **Widerstand** | Changes supported websites, including feeds, colors, and videos. It does **not** mean that all websites are blocked. UI string: „Widerstandsmodus aktivieren“. |
| Friction / intensity | **zusätzliche Hürden** / **Intensität** | Describes specific interventions that reduce stimulation in Resistance Mode. UI string: „Intensität der Hürden“. |
| Categories / Site Packs | **Kategorien**; for technical packages **Listenpakete** | Selected website sets. A download does **not** enable a Network Block. Users must attach the set to a Network Block. |
| Custom Lists | **eigene Listen** | Imported domain lists. UI string: „Eigene Liste importieren“. |
| Free / Pro | **Free / Pro** | Keep the commercial plan names unchanged. Use „kostenloser Tarif“ and „Pro-Tarif“ when explanation is necessary. |

## Style

- Use standard German for Germany.
- Address users with natural `du/dein` forms. Do not mix these forms with `Sie` without a reason.
- Keep UI actions short and clear. Explain security and cancellation effects directly.
- Do not use „Block“ to mean both Netzwerksperre and Änderungsschutz. These are different concepts.
- Use „Widerstand“ for „Widerstandsmodus“ when a control has limited space.
- Use the English source as the factual reference. Consult Italian only to resolve unclear meaning.
- Keep placeholders and all three count forms (zero, one, more than one). Check German grammar for each form.

## Release review

The translated privacy and contract text must keep the meaning of the English source.
Translation is **not a legal review**.
Before release in Germany, ask a qualified specialist to review data collection, encryption claims,
withdrawal and refunds, liability limits, age limits, and subscription renewal.
Before release, check claims in the English source against actual product behavior.
