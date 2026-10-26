# Default comments — requirements

A reader can comment on an ordinary Demistify document without the author configuring feedback. This is the owner's October 25 request forwarded by Sun and Station Chief as one job.

- R1: Runtime supplies the existing Comments surface when a page has no authored `ex-comments`. Comments are enabled, with the drawer initially closed.
- R2: Missing document identity comes from the page address; missing revision comes from a content fingerprint. Explicit author values keep their existing meaning and storage namespace.
- R3: Comments is always available on ordinary documents. There is no document flag to disable it.
- R4: Paragraphs, list items, headings, callouts, cards and trees receive comment controls without IDs or labels authored for feedback. Existing choice and explicitly labeled targets continue to work.
- R5: The normal shell, safe C shortcut, selection anchors, local saving, export and choice responses remain the existing feature. Inspector copies and generated UI are not extra comment targets.
- R6: Runtime/API guide and presentation-html instructions teach the defaults, overrides and limits. Permanent tests establish the new behavior; observe the built UI on desktop and phone.
- R7: Open source PRs for the owner to merge. Do not merge, tag, deploy or release this follow-up.

Out of scope: new comment storage/protocol, cross-revision migration, new rail design, builder API, walkthrough fixes, arbitrary HTML pages without ex-document.
