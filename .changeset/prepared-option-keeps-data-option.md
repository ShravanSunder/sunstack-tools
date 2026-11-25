---
'@sunstack/demistify': patch
---

`design check` no longer fails its own prepared page on a record-bound choice: preparation keeps each option's `data-option` (as the browser's hydration already did), so the check's "A record-bound option needs a nonempty data-option" no longer fires on valid markup, and a settled choice in a prepared page marks its chosen option.
