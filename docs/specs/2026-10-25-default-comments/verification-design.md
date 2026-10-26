# Default comments — verification design

| Obligation | Deciding observation | Wrong build rejected |
| --- | --- | --- |
| R1/R2/R4 | Built runtime fixture without Comments tag, metadata, IDs or labels creates one closed trigger, derived metadata, all requested target kinds; save/reload resolves same comment | Current opt-in runtime fails |
| R2 | Same source with changed query/hash has same namespace; changed markup and fetched Markdown/Explorer source change revision; theme/choice state do not | Hashing rendered UI or only src URLs fails |
| R2 | Explicit identity/revision preserved; partial overrides fill only missing field; explicit surface not duplicated | Blind overwrite/default injection fails |
| R3 | Automatic and explicit bindings still create one launcher and semantic controls even if old unsupported comments="off" markup is supplied; checker rejects that removed attribute | Either retained disabling branch or vocabulary entry fails |
| R4/R5 | Stable generated target IDs on reload, Markdown nested content included; inspector copies and generated UI excluded; no duplicate controls after refresh; selection and keyboard existing suite | Random IDs, blanket p/li scan or duplicate controllers fail |
| R4/R5 | Existing browser geometry and new desktop/phone target bounds; native CUA hover, open/save/close and phone shell inspection | Text/control overlap or broken normal shell fails |
| R6 | Checker fixtures accept absent metadata but retain invalid binding checks; current guide examples checked; independent one-skill reviews and bounded author application of revised guidance | Stale opt-in/required metadata teaching fails |
| R7 | Fresh PR head, checks, comments, threads, mergeability inspected; PR remains unmerged | Local-only or merged delivery fails |

Use existing Vitest Node and built-runtime Chromium browser harnesses. Add permanent regression cases at the narrowest fitting layer and demonstrate failure against the pre-change code. Keep existing tests; adapt only assumptions explicitly superseded by the owner's removal of the opt-out. A choice-only page still gets Comments. Run root check/build, scoped Node suite and browser suite, then native CUA against matching served assets. CUA direct-name Chrome baseline was acquired and Comments open/close observed; this proves feasibility only. No mock source or test-only runtime path counts as integration proof. Record counts, exits and native observations separately. Browser automation remains the established test runner, not a replacement for requested native visual inspection.

## Adverse ordering and lifecycle observations (R2/R5)

Use the existing built-fixture middleware and browser runner; hold real HTTP source responses at an explicit harness gate, no sleep. Start with identical authored markup and fixed source references. Release Markdown before Explorer, then Explorer before Markdown in another fixture instance; both complete inputs must produce the same revision. Change only the late fetched payload and require a different revision. Request counters must read exactly one existing-loader request per input, rejecting a second fingerprint fetch. A failed source must show its existing visible error and derive a failed-marker revision; a separate successful reload of the same authored content must derive a different revision. The gate stays in permanent test harness code, not production or a new driver.

While initial identity is held, remove the Comments element and then release source completion: there must be no trigger/controller installed by the detached instance. Repeat with removal/reinsertion before release: exactly one current trigger and one target control after settlement. Start two explicit instances while identity is pending: only one binds after release and the other reports the existing duplicate-surface error. These are component lifecycle observations; no synthetic action is reported as native pointer proof. The wrong builds are an unclosed job set, pre-await-only connectivity check, absent generation check, absent post-await uniqueness check or duplicate source fetch.

The Program Design obligation map links each existing R/E and observable paragraph to its deciding rows above; these additions change proof/realization only, not Requirements/Specification meaning.
