import { HighlightStyle, syntaxHighlighting } from '@codemirror/language';
import { EditorView } from '@codemirror/view';
import { tags } from '@lezer/highlight';

export const editorTheme = EditorView.theme(
  {
    '&': { height: '100%', backgroundColor: '#1e1e1e', color: '#d4d4d4' },
    '.cm-scroller': { fontFamily: 'Menlo, Consolas, "DejaVu Sans Mono", monospace', fontSize: '14px' },
    '.cm-content': { caretColor: '#aeafad' },
    '.cm-cursor, .cm-dropCursor': { borderLeftColor: '#aeafad' },
    '&.cm-focused .cm-selectionBackground, .cm-selectionBackground, .cm-content ::selection': {
      backgroundColor: '#264f78'
    },
    '.cm-gutters': { backgroundColor: '#1e1e1e', color: '#858585', border: 'none' },
    '.cm-activeLine': { backgroundColor: '#ffffff0a' },
    '.cm-activeLineGutter': { backgroundColor: '#ffffff0a', color: '#c6c6c6' },
    '.cm-stop-line': { backgroundColor: '#ffcc0033' },
    '.cm-breakpoint-gutter .cm-gutterElement': { width: '14px', cursor: 'pointer' },
    '.cm-breakpoint': {
      display: 'inline-block',
      width: '9px',
      height: '9px',
      borderRadius: '50%',
      backgroundColor: '#e51400'
    },
    '.cm-tooltip': { backgroundColor: '#252526', color: '#cccccc', border: '1px solid #454545' },
    '.cm-manifest-hover': { maxWidth: '480px', padding: '4px 8px' },
    '.cm-manifest-hover p': { margin: '4px 0 0' },
    '.cm-tooltip-autocomplete ul li[aria-selected]': { backgroundColor: '#04395e' },
    '.cm-panels': { backgroundColor: '#252526', color: '#cccccc' },
    '.cm-token-keyword': { color: '#c586c0' },
    '.cm-token-constant': { color: '#569cd6' },
    '.cm-token-string': { color: '#ce9178' },
    '.cm-token-number': { color: '#b5cea8' },
    '.cm-token-comment': { color: '#6a9955' },
    '.cm-token-function': { color: '#dcdcaa' },
    '.cm-token-variable': { color: '#9cdcfe' },
    '.cm-token-property': { color: '#9cdcfe' },
    '.cm-token-parameter': { color: '#9cdcfe', fontStyle: 'italic' },
    '.cm-token-operator': { color: '#d4d4d4' },
    '.cm-token-punctuator': { color: '#d4d4d4' },
    '.cm-token-type': { color: '#4ec9b0' }
  },
  { dark: true }
);

export const editorHighlightStyle = syntaxHighlighting(
  HighlightStyle.define([
    { tag: [tags.keyword, tags.operatorKeyword], color: '#c586c0' },
    { tag: [tags.atom, tags.bool, tags.null], color: '#569cd6' },
    { tag: tags.string, color: '#ce9178' },
    { tag: tags.number, color: '#b5cea8' },
    { tag: tags.comment, color: '#6a9955' },
    { tag: tags.function(tags.variableName), color: '#dcdcaa' },
    { tag: [tags.variableName, tags.propertyName], color: '#9cdcfe' },
    { tag: tags.special(tags.variableName), color: '#9cdcfe', fontStyle: 'italic' },
    { tag: tags.typeName, color: '#4ec9b0' },
    { tag: [tags.operator, tags.punctuation], color: '#d4d4d4' }
  ])
);
