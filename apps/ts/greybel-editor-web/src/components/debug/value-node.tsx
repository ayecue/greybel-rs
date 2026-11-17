import type { Variable } from 'greybel-runtime';
import { useEffect, useState } from 'preact/hooks';

import type { DebugState } from '../../runtime/debug-state.js';
import type { RuntimeSession } from '../../runtime/session.js';

export interface ValueRow {
  readonly name: string;
  readonly value: string;
  readonly typeName: string;
  /** Greater than 0 for lists and maps whose items can be listed. */
  readonly reference: number;
}

export interface ValueNodeConfiguration {
  readonly session: RuntimeSession;
  readonly debug: DebugState;
  readonly row: ValueRow;
  /** Identifies the node in `debug.openPaths`. */
  readonly path: string;
  /** False while the program runs or once `row` belongs to an earlier stop. */
  readonly isCurrent: boolean;
  /** The scope, list or map containing the row; undefined when the value can't be changed. */
  readonly parentReference?: number;
}

/** A value with its type, expandable when it is a list or map, and editable by double-click when it has a parent. */
export function ValueNode({ session, debug, row, path, isCurrent, parentReference }: ValueNodeConfiguration) {
  const [current, setCurrent] = useState(row);
  const [isOpen, setOpen] = useState(() => debug.openPaths.has(path));
  const [children, setChildren] = useState<readonly Variable[]>([]);
  const [draft, setDraft] = useState<string>();
  const [problem, setProblem] = useState<string>();
  const isExpandable = current.reference > 0;

  useEffect(() => setCurrent(row), [row]);

  useEffect(() => {
    if (!isOpen || !isExpandable || !isCurrent) return;

    let isLatest = true;

    session.getVariables(current.reference).then(
      (variables) => {
        if (!isLatest) return;
        setChildren(variables);
      },
      () => {
        if (!isLatest) return;
        setChildren([]);
      }
    );

    return () => {
      isLatest = false;
    };
  }, [isOpen, isExpandable, isCurrent, current.reference, session]);

  const toggle = () => {
    if (!isExpandable) return;
    if (isOpen) debug.openPaths.delete(path);
    else debug.openPaths.add(path);
    setOpen(!isOpen);
  };

  const commit = async (value: string) => {
    if (parentReference === undefined) return;
    try {
      const variable = await session.setVariable(parentReference, current.name, value);

      setCurrent(variable);
      setProblem(undefined);
    } catch (error) {
      setProblem(session.describeFailure(error));
    }

    setDraft(undefined);
  };

  const canEdit = isCurrent && parentReference !== undefined;

  return (
    <li class="value-node" data-path={path}>
      <div class="value-row">
        <button
          type="button"
          class={`value-toggle${isExpandable ? '' : ' is-leaf'}`}
          disabled={!isExpandable}
          aria-expanded={isExpandable ? isOpen : undefined}
          onClick={toggle}
        >
          {getToggleSymbol(isExpandable, isOpen)}
        </button>
        <span class="value-name">{current.name}</span>
        {draft === undefined ? (
          <span
            class="value-text"
            title={canEdit ? 'Double-click to change the value' : undefined}
            onDblClick={() => {
              if (!canEdit) return;
              setDraft(current.value);
            }}
          >
            {current.value}
          </span>
        ) : (
          <input
            type="text"
            class="value-edit"
            value={draft}
            ref={(input) => input?.focus()}
            onInput={(event) => setDraft(event.currentTarget.value)}
            onKeyDown={(event) => {
              if (event.key === 'Escape') setDraft(undefined);
              if (event.key !== 'Enter') return;
              void commit(event.currentTarget.value);
            }}
            onBlur={() => setDraft(undefined)}
          />
        )}
        <span class="value-type">{current.typeName}</span>
      </div>
      {problem !== undefined && <div class="value-problem">{problem}</div>}
      {isOpen && isExpandable && (
        <ul class="value-children">
          {children.map((child) => (
            <ValueNode
              key={child.name}
              session={session}
              debug={debug}
              row={child}
              path={`${path}/${child.name}`}
              isCurrent={isCurrent}
              parentReference={current.reference}
            />
          ))}
        </ul>
      )}
    </li>
  );
}

function getToggleSymbol(isExpandable: boolean, isOpen: boolean): string {
  if (!isExpandable) return '';

  return isOpen ? '▾' : '▸';
}
