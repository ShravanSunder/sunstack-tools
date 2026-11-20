# Demistify

Demistify provides native web components for authored HTML and a CLI for preparing and checking documents. The CLI requires Node 26 or later.

Run `npm exec -- demistify` for usage. The [authoring guide](authoring/SKILL.md) describes the components and CLI. Its `demistify` examples assume the command is on `PATH`. With a local npm installation, prefix those examples with `npm exec --`.

Browser builds can resolve `@sunstack/demistify/runtime` and `@sunstack/demistify/theme.css`. For plain HTML, serve or copy the package's entire `dist/` directory and load `theme.css` and `runtime.js` from that directory. The browser loads fonts, the diagram worker and lazy chunks relative to those entry files.
