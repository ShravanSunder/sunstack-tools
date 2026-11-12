import { pathToFileURL } from 'node:url'
const { regex } = await import(pathToFileURL(process.argv[2]).href)
import { writeFileSync } from 'node:fs'
let ranges = [], start = null, previous = null, partial = []
for (let code = 0; code <= 0x10ffff; code++) {
  if (code >= 0xd800 && code <= 0xdfff) continue
  const input = String.fromCodePoint(code)
  const result = input.replace(regex, '')
  if (result !== '' && result !== input) partial.push([code, result])
  if (result === '') {
    if (start === null) start = code
    else if (code !== previous + 1) { ranges.push([start, previous]); start = code }
    previous = code
  }
}
if (start !== null) ranges.push([start, previous])
console.log(JSON.stringify({partialCount: partial.length, partial: partial.slice(0, 10), ranges: ranges.length, node: process.version, unicode: process.versions.unicode}))
if (partial.length) process.exit(1)
const tuples = ranges.map(([a,b]) => `(0x${a.toString(16)}, 0x${b.toString(16)}),`)
const lines = []
for (let i = 0; i < tuples.length; i += 3) lines.push('    ' + tuples.slice(i, i + 3).join(' '))
writeFileSync(process.argv[3], `// Generated from github-slugger 2.0.0 regex.js; see github_heading_slug_test/PROVENANCE.md.\nconst REMOVED_SCALAR_RANGES: &[(u32, u32)] = &[\n${lines.join('\n')}\n];\n`)
