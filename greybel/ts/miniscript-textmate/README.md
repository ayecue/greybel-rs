# miniscript-textmate

MiniScript TextMate resources: the `source.ms` grammar, language configuration
for comments, brackets and indentation, and a dark theme. The grammar follows
the reference MiniScript implementation and excludes Greybel's block comments
and directives.

```ts
import { languageConfig, theme, tmLanguage } from 'miniscript-textmate';
```

The JSON resources are also exported separately:

- `miniscript-textmate/miniscript.tmLanguage.json`
- `miniscript-textmate/miniscriptLanguageConfig.json`
- `miniscript-textmate/miniscript.theme.json`

The grammar is handwritten except for the API function and method names. The
`miniscript-meta` tests check those generated rules. From `greybel/rs`, rewrite
them with:

```sh
UPDATE_TS=1 cargo test -p miniscript-meta
```
