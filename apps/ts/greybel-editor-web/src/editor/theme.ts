import { HighlightStyle, syntaxHighlighting } from '@codemirror/language';
import { EditorView } from '@codemirror/view';
import { tags } from '@lezer/highlight';

/** Grey Hack's code editor colors, from its `CodeStyling` asset; `type` and `default` from its signature bar. */
export const GREY_HACK_COLORS = {
  background: '#040719',
  activeLine: '#141728',
  lineNumber: '#6d6f78',
  text: '#ffffff',
  keyword: '#f92672',
  string: '#dadb69',
  number: '#44aa44',
  comment: '#666666',
  operator: '#99ccff',
  colon: '#cc66ff',
  library: '#42c8ef',
  type: '#4d7cf4',
  parens: ['#73a4e7', '#db73e7', '#e78c73', '#a9e773', '#73e7d6', '#8873e7'],
  squares: ['#7a7aff', '#ff7ad4', '#ffcf7a', '#7fff7a', '#7adaff', '#ca7aff'],
  unmatched: '#ff0000'
} as const;

const colors = GREY_HACK_COLORS;

const bracketRules = Object.fromEntries([
  ...colors.parens.map((color, index) => [`.cm-paren-${index + 1}`, { color }]),
  ...colors.squares.map((color, index) => [`.cm-square-${index + 1}`, { color }]),
  ['.cm-bracket-unmatched', { color: colors.unmatched }]
]);

export const editorTheme = EditorView.theme(
  {
    '&': { height: '100%', backgroundColor: colors.background, color: colors.text },
    '.cm-scroller': { fontFamily: 'Menlo, Consolas, "DejaVu Sans Mono", monospace', fontSize: '14px' },
    '.cm-content': { caretColor: colors.text },
    '.cm-cursor, .cm-dropCursor': { borderLeftColor: colors.text },
    '&.cm-focused .cm-selectionBackground, .cm-selectionBackground, .cm-content ::selection': {
      backgroundColor: '#2a3f6b'
    },
    '.cm-gutters': { backgroundColor: colors.background, color: colors.lineNumber, border: 'none' },
    '.cm-activeLine': { backgroundColor: '#ffffff08' },
    '.cm-activeLineGutter': { backgroundColor: colors.activeLine, color: '#c8c9ce' },
    '.cm-stop-line': { backgroundColor: '#ffcc0033' },
    '.cm-frame-line': { backgroundColor: '#7fff0024' },
    '.cm-breakpoint-gutter .cm-gutterElement': { width: '14px', cursor: 'pointer' },
    '.cm-breakpoint': {
      display: 'inline-block',
      width: '9px',
      height: '9px',
      borderRadius: '50%',
      backgroundColor: '#e51400'
    },
    '.cm-tooltip': { backgroundColor: '#141617', color: '#d0d0d0', border: '1px solid #2a2a2a' },
    '.cm-manifest-hover': { maxWidth: '480px', padding: '4px 8px' },
    '.cm-manifest-hover p': { margin: '4px 0 0' },
    '.cm-tooltip-autocomplete ul li[aria-selected]': { backgroundColor: '#1d2a4a' },
    '.cm-panels': { backgroundColor: '#141617', color: '#d0d0d0' },
    '.cm-token-keyword, .cm-token-constant': { color: colors.keyword },
    '.cm-token-string': { color: colors.string },
    '.cm-token-number': { color: colors.number },
    '.cm-token-comment': { color: colors.comment, fontStyle: 'italic' },
    '.cm-token-function, .cm-token-variable, .cm-token-property, .cm-token-parameter': { color: colors.text },
    '.cm-token-operator, .cm-token-punctuator': { color: colors.operator },
    '.cm-token-colon': { color: colors.colon },
    '.cm-token-type': { color: colors.type },
    '.cm-token-default-library': { color: colors.library },
    ...bracketRules
  },
  { dark: true }
);

export const editorHighlightStyle = syntaxHighlighting(
  HighlightStyle.define([
    { tag: [tags.keyword, tags.operatorKeyword, tags.atom, tags.bool, tags.null], color: colors.keyword },
    { tag: tags.string, color: colors.string },
    { tag: tags.number, color: colors.number },
    { tag: tags.comment, color: colors.comment, fontStyle: 'italic' },
    { tag: tags.function(tags.variableName), color: colors.library },
    { tag: [tags.variableName, tags.propertyName], color: colors.text },
    { tag: tags.special(tags.variableName), color: colors.text, fontStyle: 'italic' },
    { tag: tags.typeName, color: colors.type },
    { tag: [tags.operator, tags.punctuation], color: colors.operator }
  ])
);
