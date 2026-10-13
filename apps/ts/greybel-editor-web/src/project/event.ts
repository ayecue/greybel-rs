/** File creation or deletion event sent to the language server. */
export interface FileChange {
  readonly path: string;
  readonly kind: 'creation' | 'deletion';
}

/** Project change to file contents, entry file, or file membership. */
export type ProjectEvent =
  | { readonly kind: 'edit'; readonly path: string }
  | { readonly kind: 'entry' }
  | { readonly kind: 'files'; readonly changes: readonly FileChange[] };
