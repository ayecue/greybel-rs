import stylistic from '@stylistic/eslint-plugin';
import tseslint from 'typescript-eslint';

import earlyExit from './eslint/early-exit.js';

/**
 * The layout Prettier does not reformat: blank lines around a block of declarations (none above it as a block's first
 * statement), after blocks, before a `return` and between class members; and early exits: over an `if` wrapping the
 * remainder of a function or loop body (`presets/early-exit`), over an `else` after a branch that returns
 * (`no-else-return`), and an `else { if … }` as `else if` (`no-lonely-if`). `pnpm format` fixes them, then runs
 * Prettier. Blocks nest at most 3 levels (`max-depth`) and ternaries don't nest (`no-nested-ternary`); those have no fix. Only TypeScript is checked; modules a crate's
 * tests generate are left as their generator writes them.
 */
export default tseslint.config(
  {
    ignores: [
      '**/node_modules/',
      '**/dist/',
      '**/dist-test/',
      '**/out/',
      '**/.vscode-test/',
      '**/.vscode-test-web/',
      '**/*.generated.*',
      '**/src/crates/'
    ]
  },
  {
    files: ['**/*.{ts,tsx,mts,cts}'],
    languageOptions: { parser: tseslint.parser },
    linterOptions: { reportUnusedDisableDirectives: 'off' },
    plugins: { '@stylistic': stylistic, presets: { rules: { 'early-exit': earlyExit } } },
    rules: {
      '@stylistic/padding-line-between-statements': [
        'error',
        { blankLine: 'always', prev: ['const', 'let', 'var'], next: '*' },
        { blankLine: 'always', prev: '*', next: ['const', 'let', 'var'] },
        { blankLine: 'any', prev: ['const', 'let', 'var'], next: ['const', 'let', 'var'] },
        { blankLine: 'always', prev: 'directive', next: '*' },
        { blankLine: 'always', prev: 'import', next: '*' },
        { blankLine: 'any', prev: 'import', next: 'import' },
        { blankLine: 'always', prev: 'block-like', next: '*' },
        { blankLine: 'always', prev: '*', next: 'return' }
      ],
      '@stylistic/lines-between-class-members': ['error', 'always', { exceptAfterSingleLine: true }],
      'presets/early-exit': 'error',
      'no-else-return': ['error', { allowElseIf: false }],
      'no-lonely-if': 'error',
      'max-depth': ['error', 3],
      'no-nested-ternary': 'error'
    }
  }
);
