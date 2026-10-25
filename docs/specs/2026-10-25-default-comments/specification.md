# Default comments — specification

## Page and review identity

The primary original `ex-document` is the first one in document order outside inspection copies. If the page has no original `ex-comments` anywhere, runtime adds one bound to that document unless it has `comments="off"`. Existing authored bindings remain authoritative; a binding to an opted-out document stays inactive. This follows the existing single normal page shell rather than introducing multiple competing global launchers.

Missing `document-id` defaults to the current URL with credentials, query and fragment removed. Viewer selections, search and walkthrough state therefore do not create new logical pages. Moving a file/URL changes the default identity; an explicit document-id is the way to keep identity across a move. Existing nonempty metadata is preserved; authored empty/invalid metadata still reports an error.

Missing `revision` defaults to `sha256:<hex>` of the original document markup plus source data loaded by its initial Markdown and Explorer islands. Capture markup before runtime upgrades add controls or replace source. Retain the actual loaded text/JSON through each island's existing fetch, without duplicate requests. Theme, comments, selected options, transient graph selections and inspection copies do not enter the fingerprint. Loaded external-source failures use a distinct failed-source marker; recovery changes the fingerprint. Binary/image contents, runtime assets, and later application-driven document edits are outside automatic revision detection. Explicit revision owns versioning when the author needs a different boundary.

A reload of unchanged source at the same page address restores the same review namespace. Changed authored content selects a new namespace and does not overwrite the earlier review. This changes defaults only, not the existing review envelope or localStorage format.

## Automatic targets

Original authored content in the document becomes eligible: p, li, h1–h6, ex-callout, ex-card, ex-tree, ex-file-tree; existing choices and labeled targets remain. Paragraphs and headings rendered from Markdown are included. Generated interactive Explorer/tree UI is excluded; the tree as a whole remains a target. Preserve explicit IDs and labels. Otherwise use deterministic structural IDs scoped to the original document/container and readable labels from title/headings/text. Generated IDs need stability within the same content revision, not across edits.

Empty comment controls appear on hover/focus; targets with saved comments stay visible. A target reserves room so its control does not cover text or inspector actions. Nested content must remain usable without multiple icons overlapping. Selection capture continues excluding controls and inspection copies. A removed/reinserted Comments element must not bind late after disconnection or create duplicate controllers.

## Entities

| Entity | Identity and meaning | Relationship |
| --- | --- | --- |
| E1 Document | One original ex-document; logical identity names its page, revision names its authored content | Owns the eligible content and source inputs |
| E2 Review | Existing demistify-review-1 envelope keyed by documentId/documentRevision | Owns comments and current choice responses for E1 |
| E3 Review target | Original content element identified within one revision | Receives entity/text anchors from E2; explicit or deterministic ID |
| E4 Source input | Initial fetched Markdown text or Explorer JSON, or failed-load marker | Supplements E1 original markup when deriving revision |

Source inputs are fingerprint inputs, not a second persisted document model. No new persisted entity is introduced.
