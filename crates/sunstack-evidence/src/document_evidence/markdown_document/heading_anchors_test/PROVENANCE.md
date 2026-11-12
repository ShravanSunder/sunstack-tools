# Minify heading fixture

`minify-heading.md` is the first line, including its newline, copied verbatim from
[tdewolff/minify README](https://github.com/tdewolff/minify/blob/eaad54db7f2205ed37db251a460cec7fd5f82127/README.md),
commit `eaad54db7f2205ed37db251a460cec7fd5f82127`. The source README SHA256 is
`27703cbf2b0d13f7fe5ff3582a58648ab069c684cce39c412c0a5ac1acdf8dc5`;
the 438-byte heading SHA256 is
`0537c6f86fe2e095ee996edb244b85e68222c73c2fdf6d6302416120d56adc43`.

GitHub's rendered contents API at that revision has the first heading permalink
`user-content-minify----`. It retains the four spaces around the empty anchor
and badge images, while image alt text contributes no heading text. The phantom
`minify--api-reference-go-report-card-codecov` is absent.

The public rendered response SHA256 was
`002003f0fead0b357182a4a328cb9b7b3a5ec0f58ebc567f2ff8f64d33577674`.
This fixture reproduces the observed image-alt bug; it does not depend on fetching
the badge images or executing any Markdown links.
