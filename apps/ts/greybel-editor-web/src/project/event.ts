export interface FileChange {
  readonly path: string;
  readonly kind: 'creation' | 'deletion';
}

/** A rename also emits `files` changes. */
export type ProjectEvent =
  | { readonly kind: 'edit'; readonly path: string }
  | { readonly kind: 'entry' }
  | { readonly kind: 'files'; readonly changes: readonly FileChange[] }
  | { readonly kind: 'rename'; readonly from: string; readonly to: string };
