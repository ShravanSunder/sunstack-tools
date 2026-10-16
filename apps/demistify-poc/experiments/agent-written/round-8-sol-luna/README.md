# Matched Sol and Luna code-review authors

Fresh Sol (`gpt-6.1-sol`, medium) and Luna (`gpt-6-luna`, high) received the same fixed inputs and no inherited conversation. Open `sol-iterated.html` and `luna-iterated.html` independently after the normal lab build. `*-initial.html` and the first accepted `sol.html` / `luna.html` remain verbatim. These HTML sources are excluded from formatting.

Each first run read 2,728 input-file tokens and made one checker call, reporting zero errors and warnings. Sol wrote 2,109 HTML tokens; Luna wrote 1,706. After one shared visual-feedback round, outputs measured 2,138 and 1,849 tokens. This is file-payload accounting with o200k_base. Complete assignment/tool/hidden context and provider billing were not captured.

The first Luna page had no section h2 headings and wrapped its summary's final stat onto another row. Its iteration added seven section headings and a four-column summary. Sol's first page already rendered six Markdown h2 headings; replacing them with literal h2 tags did not improve its outline. This exposes ambiguity in the feedback phrase "real h2", not a model ranking.

Both iterated pages were inspected at an effective 1,419 × 747 CSS viewport: no outer horizontal overflow or reported authoring issue; Sol had seven outline links and Luna eight. Parent source review found F1–F5, three fix options, repository changes, the exact supplied diff and evidence values. The brief is illustrative; its staging numbers and source behavior were not independently established.

Inputs and exact output hashes are preserved here. Receipts replace machine paths with placeholders and retain the authors' own proof limits. Browser screenshots and observations live in the root WIP experiment evidence directory. The normal build copies the complete shared library beside these pages; it is excluded from authored usage counts.
