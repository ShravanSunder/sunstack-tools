// Usage: node generate-lowercase-fixtures.mjs /absolute/path/to/github-slugger/index.js output.json
import { writeFileSync } from 'node:fs'
import { pathToFileURL } from 'node:url'

const { slug } = await import(pathToFileURL(process.argv[2]).href)
const fixtures = []
for (let code = 0; code <= 0x10ffff; code++) {
  if (code >= 0xd800 && code <= 0xdfff) continue
  const input = String.fromCodePoint(code)
  if (input.toLowerCase() !== input) fixtures.push([input, slug(input)])
}
const lines = []
for (let i = 0; i < fixtures.length; i += 4) {
  lines.push('  ' + fixtures.slice(i, i + 4).map(JSON.stringify).join(', '))
}
writeFileSync(process.argv[3], '[\n' + lines.join(',\n') + '\n]\n')
console.log(JSON.stringify({ count: fixtures.length, node: process.version, unicode: process.versions.unicode }))
