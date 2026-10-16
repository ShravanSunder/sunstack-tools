Create two HTML pages for an engineering review, using the supplied Demistify library and authoring guide. Work only in $EXPERIMENT/luna. Read $EXPERIMENT/inputs/SKILL.md, the review brief and graph inventory from $EXPERIMENT/inputs; follow relevant reference pointers in that supplied guide. Use no other repository examples, source code, network, browser or helpers.

Write review.html from review-brief.md: make the exact change, findings, options and evidence easy to review, with useful section navigation and a compact summary. Include the repository changes as a tree. The supplied facts are illustrative, so distinguish intended behavior from actual findings and supplied evidence from verified runtime behavior.

Write explorer.html for the supplied planning.json graph. Start with spec-design in the proposed view. Add a short reading guide that explains selection, current/proposed views, hover tracing, arrow-to-evidence rows and incoming consumers. Retain that this is a dated proposal and source quotations do not prove execution or a shipped implementation. Use the shared component and existing complete data; do not regenerate the corpus or application code.

Both pages load demistify/theme.css and demistify/runtime.js. Write ordinary HTML with library composition; do not write application JavaScript or presentation CSS. Keep your first drafts as review-initial.html and explorer-initial.html before running checks; repair only review.html and explorer.html. The initial and repaired outputs are reviewable artifacts.

From $WORKTREE, run the prescribed command separately for each final page: node packages/demistify/bin/demistify.ts check ABSOLUTE_PAGE_PATH. Save each command's output to a numbered text file and capture its numeric exit code. At most three calls per page; stop checking a page once it reports zero errors and warnings. Do not alter the checker, library, data, input files or other author's directory. Do not commit, push, deploy, launch external providers or spend API credits.

Return a short note with output files, checker commands/results, any repairs and unverified claims. The Lead handles browser review.
