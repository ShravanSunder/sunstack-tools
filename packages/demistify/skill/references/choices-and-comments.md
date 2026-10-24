# Answers, supporting details and comments

This reference owns choice and feedback authoring. Inputs are the reader's question, supplied options and sources, requested feedback or handoff, and matching library binding. Return the requested arrangement, its required identities and binding, optional supporting-detail placement and the interactions to observe.

## Ask only for the response the reader needs to give

Use `ex-choice` for an answer: native radios select one option; bare `multiple` uses independent checkboxes. Use `ex-compare` to inspect corresponding topics, ordinary prose to explain, and `ex-card pick` to display an already supplied owner decision. A recommendation, the visible comparison pair and a reader's current answer are different facts.

Add `ex-comments` when the task needs feedback or one combined handoff of comments and current answers. A feedback page can contain no choices. A choice can have no extras or Comments. Keep explanation in document flow and content in its owning place.

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
  <ex-document id="content" document-id="review-rollout" revision="r1">
    <h1>Choose the review rollout</h1>
    <p id="scope" data-comment-label="Rollout scope">The supplied proposal introduces a review page for one team before wider adoption.</p>
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
  <ex-comments for="content"></ex-comments>
</body>
</html>
```

For feedback alone, keep the identified document, content and Comments binding and omit the choice. For a simple answer without feedback or combined handoff, omit Comments and its document review identity; the choice and option IDs still identify the answer. Full extras are optional in either case.

## Author the option hierarchy

Each `ex-choice` requires a stable unique `id` and a question `title`. Its direct `ex-option` children each require a unique `id`, an inline-Markdown `title` and one `template slot="summary"` for the body. Supply configuration before connection; attributes are not a reactive configuration protocol.

The title is one compact inline-Markdown paragraph, supporting emphasis, strong text, inline code, strike-through and line breaks. Unsupported title content is an authoring error; move substantial content into its body or extras instead of dropping it. The title uses the shared 18px semibold role. The quieter body uses 15px regular text.

The summary uses the exact focused Markdown renderer used by Walkthrough narration: paragraphs, emphasis, lists, blockquotes, links and inline code. Headings become bold lead paragraphs without page-outline sections. Images become their text labels, fenced code remains readable code text, and tables remain text; authored HTML stays literal. Put actual diagrams, images, full tables and long code in `slot="details"`, using existing library components. Preserve the supplied Markdown and its line structure; do not flatten it into a sentence to fit a card. `content.md` owns exact-source and literal-delimiter handling.

Clicking noninteractive title, Markdown body or card background selects or toggles the option. Native inputs retain their radio/checkbox keyboard behavior. Links, Details, Comments, other buttons and form controls keep their own actions without changing the answer. Dragging to select text selects no answer.

## Choose the response view and optional detail

Cards use Reading width by default. Supporting extras open through the native inspector: a circular 32px icon button follows the title with an 8px gap and flows beside wrapped title text. Opening it marks the complete source card, leaves its answer unchanged and returns focus on close. Use the existing inspector; no page script or second detail viewer is needed.

Use `view="table"` on the choice when short corresponding option bodies benefit from aligned selectable rows. The same option declarations become a Canvas-width native table; narrow scrolling stays inside the table boundary. Do not invent extra column or criteria attributes. Use `details="inline"` when extras should expand locally beneath an option, accepting that they move later content down. Both variations retain separate selection, detail and feedback actions.

Keep full diagrams and their headings in extras. Opening or closing extras must leave document navigation unchanged. `inspection.md` owns shared sidebar/fullscreen controls and state-return checks; `layouts.md` owns Reading/Canvas and narrow containment.

## Bind one review to explicit identities

`ex-comments for="content"` targets the original `ex-document` whose DOM `id` is `content`. That document requires `document-id` for its logical identity and `revision` for the reviewed content. The DOM ID locates the document; the logical identity and revision scope saved comments. Neither a URL nor a display title replaces them. A changed revision starts a separate review; do not reuse a revision to imply old anchors apply to changed content.

The document, choices and options are automatic comment targets. Other original content opts in with its own unique `id` and readable `data-comment-label`, as `scope` does in the example. Keep target, question and option IDs stable while their meaning stays the same. Duplicate IDs, missing option content or identity, missing review identity, an invalid `for` binding or a second Comments surface for that document are visible authoring errors.

One Comments panel owns composition, existing comments, current selected answers and Copy/Export. Readers can comment on an item or actual selected text. A selection spanning blocks anchors to the document; controls and source carriers are excluded from its source text. The comment retains the selected quote and exact source range/context. Following it reveals the original target when possible. Missing or changed source is reported as unavailable, without guessing a replacement passage.

C toggles Comments outside editable fields; modified commands, composition and repeated key events retain their normal ownership. Cmd+Enter saves a nonempty comment. Enter and Shift+Enter insert newlines. Existing comment counts remain visible; empty target controls reveal on hover/focus and stay reachable on narrow/touch layouts. Comments and native inspection share the right-side panel owner: opening either replaces the other, and fullscreen restores the live original state.

## Keep local saving and explicit handoff distinct

Comments save locally for the same explicit document/revision, retaining review/comment IDs, bodies, anchors and timestamps. Answers remain current native input state and have no reload-persistence promise. Unavailable storage, failed writes and invalid saved content are visible states. Invalid stored data is preserved; new in-memory feedback remains unsaved and can be copied or exported. Do not erase saved data or claim it was saved to dismiss that state.

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
4. When feedback is requested, comment on an item and an exact passage, including a cross-block quote when useful. Inspect the quote and return target, save with Cmd+Enter, retain newline/editing behavior and reopen the same document revision. Compare saved comment identities/content; do not promise restored answers.
5. When a combined handoff is requested, inspect actual Copy/Export contents for the current answer IDs and comments/quotes. Keep any visible storage or clipboard failure explicit, using the provided transfer fallback.
6. Observe the requested controls at desktop and 390 CSS px in both themes. Confirm native panel replacement/focus return, shared comment colors/counts and no covered content or page overflow. Keep source-preview, HTTP, file-origin and physical-touch observations separate; one does not prove the others.

Complete when every requested interaction is checked and observed through the intended opening method, or its precise blocker is returned. Feedback alone, choices without extras and explanation without response UI remain valid compositions. Static validation alone does not prove a usable review page.
