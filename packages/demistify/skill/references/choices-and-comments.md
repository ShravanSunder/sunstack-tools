# Answers, supporting details and comments

This reference owns choice and feedback authoring. Inputs are the reader's question, supplied options and sources, requested feedback or handoff, and matching library binding. Return the requested arrangement, its required identities and binding, optional supporting-detail placement and the interactions to observe.

## Ask only for the response the reader needs to give

Use `ex-choice` for an answer: native radios select one option; bare `multiple` uses independent checkboxes. Use `ex-compare` to inspect corresponding topics, ordinary prose to explain, and `ex-card pick` to display an already supplied owner decision. A recommendation, the visible comparison pair and a reader's current answer are different facts.

Comments is always available automatically on an ordinary document, with one panel for feedback and the current-answer handoff. Add choices only when the reader needs to answer a question; feedback alone needs none. Extras remain optional. Keep explanation in document flow and content in its owning place.

## A complete review page

This example combines requested answers, optional supporting material and feedback. It assumes the complete matching library is served at `./demistify/` in an HTTP source preview. Bind its guide and assets to that same build. For a published binding, use that release's guide and replace both asset imports together; an older public runtime does not gain these elements from a newer guide.

```html
<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width,initial-scale=1">
  <title>Choose the review rollout</title>
  <link rel="stylesheet" href="./demistify/theme.css">
  <script type="module" src="./demistify/runtime.js"></script>
</head>
<body>
  <ex-nav for="content"></ex-nav>
  <ex-document id="content">
    <h1>Choose the review rollout</h1>
    <p id="scope">The supplied proposal introduces a review page for one team before wider adoption.</p>
    <ex-choice id="rollout" title="How should the review page roll out?">
      <ex-option id="pilot" title="Start with a **small pilot**">
        <template slot="summary">
Try one real workflow before expanding.

- Keep the first handoff visible.
- Review the [rollout scope](#scope) before widening access.

> The pilot is a proposed scope, not an accepted delivery date.
        </template>
        <div slot="details">
          <p>The proposed path keeps the review with its originating team.</p>
          <ex-mermaid id="pilot-path" label="Proposed pilot review path"><script type="text/plain">
flowchart LR
  Author[Author] --> Review[Team review]
  Review --> Handoff[Recorded handoff]
          </script></ex-mermaid>
        </div>
      </ex-option>
      <ex-option id="all-teams" title="Open to all teams">
        <template slot="summary">Make the same review page available to every team at once.</template>
      </ex-option>
    </ex-choice>
    <p>Choose a rollout, then use Comments to qualify the answer or quote a passage.</p>
  </ex-document>
</body>
</html>
```

For feedback alone, omit the choice. The document gets Comments without manual setup; question/option IDs still identify answers when choices are present. Full extras are optional in either case.

## Author the option hierarchy

Each `ex-choice` requires a stable unique `id` and a question `title`. Its direct `ex-option` children each require a unique `id`, an inline-Markdown `title` and one `template slot="summary"` for the body. Supply configuration before connection; attributes are not a reactive configuration protocol.

The title is one compact inline-Markdown paragraph, supporting emphasis, strong text, inline code, strike-through and line breaks. Unsupported title content is an authoring error; move substantial content into its body or extras instead of dropping it. The title uses the shared 18px semibold role. The quieter body uses 15px regular text.

The summary uses the exact focused Markdown renderer used by Walkthrough narration: paragraphs, emphasis, lists, blockquotes, links and inline code. Headings become bold lead paragraphs without page-outline sections. Images become their text labels, fenced code remains readable code text, and tables remain text; authored HTML stays literal. Put actual diagrams, images, full tables and long code in `slot="details"`, using existing library components. Preserve the supplied Markdown and its line structure; do not flatten it into a sentence to fit a card. `content.md` owns exact-source and literal-delimiter handling.

Clicking noninteractive title, Markdown body or card background selects or toggles the option. Native inputs retain their radio/checkbox keyboard behavior. Links, Details, Comments, other buttons and form controls keep their own actions without changing the answer. Dragging to select text selects no answer.

## Choose the response view and optional detail

Cards use Reading width by default. Supporting extras open through the native inspector: a compact 32px-high button with the shared document icon and visible **Details** label follows the title with an 8px gap and flows beside wrapped title text. Opening it marks the complete source card, leaves its answer unchanged and returns focus on close. Use the existing inspector; no page script or second detail viewer is needed.

Use `view="table"` on the choice when short corresponding option bodies benefit from aligned selectable rows. The same option declarations become a Canvas-width native table; narrow scrolling stays inside the table boundary. Do not invent extra column or criteria attributes. Use `details="inline"` when extras should expand locally beneath an option, accepting that they move later content down. Both variations retain separate selection, detail and feedback actions.

Keep full diagrams and their headings in extras. Opening or closing extras must leave document navigation unchanged. `inspection.md` owns shared sidebar/fullscreen controls and state-return checks; `layouts.md` owns Reading/Canvas and narrow containment.

## Use defaults, then choose deliberate overrides

The runtime supplies one `ex-comments` binding for the first original `ex-document` when the page has no authored Comments surface. Its drawer starts closed. Ordinary prose, headings, list items, callouts, cards and trees get comment controls without feedback-only IDs or labels, including prose rendered from Markdown. Generated interactive UI and inspector copies do not become feedback targets. The document remains the anchor for cross-block quotations; its shared Comments launcher avoids a duplicate icon over the first heading.

Missing `document-id` defaults to the page URL without credentials, query or fragment. Missing `revision` defaults to an authored-content SHA-256 fingerprint: original document markup plus the initial fetched Markdown/Explorer inputs, using the existing loads once. Comments bind after those inputs settle. A failed source has a distinct marker; successful recovery on a fresh load changes revision. Theme, selected answers, comments and transient viewer state do not change it. Binary/image bytes, runtime assets and later application-driven edits are outside automatic version detection.

The same page and unchanged content reopen the same local review. Moving its address or changing authored content can select another namespace; preparing a differently structured embedded copy can also change the fingerprint. Earlier saved data stays in its original namespace, without relocation or migration. Use explicit metadata when identity must survive a move or the author needs to own the revision boundary:

```html
<ex-document id="content" document-id="review-rollout" revision="r1">
  <h1>Choose the review rollout</h1>
  <p>The author owns when this revision changes.</p>
</ex-document>
```

Each field is an independent override: supply either or both; the missing field derives its default. Supplied empty/invalid values remain errors. Change an explicit revision when reviewed content changes; retaining a value does not prove old anchors still apply. Question/option IDs and IDs used by links remain authored identities; generated feedback IDs need stability only within the same content revision. A readable `data-comment-label` and unique `id` remain useful when deliberately naming another original target.

An authored `ex-comments for="content"` remains supported for a deliberate binding to an original document with DOM ID `content`; omit it for the ordinary default. Duplicate target IDs, missing choice/option content or identity, invalid explicit review values, an invalid `for` binding or a second Comments surface for the same document are authoring errors. Runtime derives defaults without changing the existing review envelope or local-storage format.

One Comments panel owns composition, existing comments, current selected answers and Copy/Export. Each question, option or ordinary target has one editable comment: its button reopens that comment, and selecting text inside the same item edits it with the passage as quote context. A new passage updates the quote on Save; reopening the whole-item button retains the saved quote. Question and option comments stay independent. The panel and copied handoff show a selected answer once with its associated comments underneath, while feedback on unselected options remains available.

A selection spanning blocks inside one choice question or option stays on that item; other cross-block selections anchor to the document. Controls and source carriers are excluded from its source text. The comment retains the selected quote and exact source range/context. Following it reveals the original target when possible. Missing or changed source is reported as unavailable, without guessing a replacement passage.

C toggles Comments outside editable fields; modified commands, composition and repeated key events retain their normal ownership. Cmd+Enter saves a nonempty comment. Enter and Shift+Enter insert newlines. Existing comment counts remain visible; empty target controls reveal on hover/focus and stay reachable on narrow/touch layouts. Comments and native inspection share the right-side panel owner: opening either replaces the other, and fullscreen restores the live original state.

## Keep local saving and explicit handoff distinct

Comments save locally in this browser origin for the same resolved document/revision, retaining review/comment IDs, bodies, anchors and timestamps. Answers remain current native input state and have no reload-persistence promise. Unavailable storage, failed writes and invalid saved content are visible states. Invalid stored data is preserved; new in-memory feedback remains unsaved and can be copied or exported. Do not erase saved data or claim it was saved to dismiss that state.

Copy carries readable comments and selected answers together. Export carries the same review as `demistify-review-1`: `documentId`, `documentRevision`, `reviewId`, `comments` and `answers`. Each comment retains its `commentId`, anchor, body and creation/update times. Whole-item anchors identify the target; text anchors also retain the actual quote and verified source range/context. Answers identify the question and selected option IDs with readable labels. An unanswered choice contributes no answer to the review. A clipboard rejection provides a read-only manual-copy path; inspect the carried content rather than accepting a success message as proof.

After the components have bound and rendered, a consuming web UI can read the fresh `ex-choice.response` value and its bubbling, composed `choice-change` event, or `ex-comments.review` and `review-change`. A response carries `questionId`, `question` and `selected` entries with `optionId` and `label`. Returned snapshots do not grant mutation of the component's state. The source review model owns validation; do not maintain a second page schema or response store. Plain HTML needs no page script for these interactions.

Nothing is submitted to an agent or server automatically. There is no account, reply-thread, prototype-storage migration, cross-revision relocation or multi-tab merge contract. Reconnecting the same instances retains their current state without duplicate controls. Sidebar inspection copies are read-only response snapshots; they do not join the original radio group or become original feedback targets. Fullscreen uses and restores the original component.

## Check the requested interactions

1. Check every final HTML file with the matching source CLI from the monorepo root:

   ```sh
   node packages/demistify/bin/demistify.ts check /absolute/path/to/review.html
   ```

   Require a nonzero file count and zero errors and warnings. Check the actual binding and complete source separately; a markup pass establishes neither.
2. When answers are requested, select by body click and native control/keyboard. Try a real link, Details or Comments action without changing the answer; drag to select text without selecting an option. Check single versus independent answers and their actual IDs.
3. When extras are authored, open them, inspect the actual rendered Markdown/SVG, close them and confirm the same answer, focus, source content and outline. Observe the whole-card/row marker, not an empty internal wrapper. Exercise table rows and inline details when those variants are used.
4. Check the automatic Comments launcher and an ordinary semantic target without manual feedback markup. When feedback is requested, comment on an item and an exact passage, including a cross-block quote when useful. Inspect the quote and return target, save with Cmd+Enter, retain newline/editing behavior and reopen the same document revision. Compare saved comment identities/content; do not promise restored answers.
5. When a combined handoff is requested, inspect actual Copy/Export contents for the current answer IDs and comments/quotes. Keep any visible storage or clipboard failure explicit, using the provided transfer fallback.
6. Keep source-preview, HTTP, file-origin and physical-touch observations separate; one does not prove the others.

Complete when every requested interaction is checked and observed through the intended opening method, or its precise blocker is returned. Feedback alone, choices without extras and explanation without response UI remain valid compositions. Static validation alone does not prove a usable review page.
