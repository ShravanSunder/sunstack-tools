# Explicit rendering checks

For a requested render check or a concrete rendering defect, work from the directory containing the affected pages and their local assets. Install Playwright in that workspace and its Chromium browser, then invoke the installed CLI.

```sh
pnpm add -D playwright
pnpm exec playwright install chromium
npm exec -- demistify check --render overview.html
```

Chromium checks the page at 1400 and 390 CSS pixels. The command reports local link and fragment failures, page overflow, visible components with render failures, and raw markup in table cells. Fix the listed file and viewport, then rerun the command.
