# greyscript-textmate

GreyScript TextMate resources: the `source.src` grammar, language configuration
for comments, brackets and indentation, and a dark theme. Editors such as VS
Code can consume them directly.

```ts
import { languageConfig, theme, tmLanguage } from 'greyscript-textmate';
```

The JSON resources are also exported separately:

- `greyscript-textmate/greyscript.tmLanguage.json`
- `greyscript-textmate/greyscriptLanguageConfig.json`
- `greyscript-textmate/greyscript.theme.json`

The grammar is handwritten except for the API function and method names. The
`greyscript-meta` tests check those generated rules. From `greybel/rs`, rewrite
them with:

```sh
UPDATE_TS=1 cargo test -p greyscript-meta
```
