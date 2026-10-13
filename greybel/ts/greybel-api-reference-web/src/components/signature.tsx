import { Literal, MemberKind, TypePart, type Argument, type TypeText } from '../protocol.generated.js';
import { TypeName } from './link.js';

export function Signature({ kind }: { kind: MemberKind }) {
  if (kind instanceof MemberKind.Value) {
    return (
      <span class="signature">
        {': '}
        <Types types={[kind.type]} />
      </span>
    );
  }

  const { arguments: argumentList, returns } = kind as MemberKind.Function;

  return (
    <span class="signature">
      (
      {argumentList.map((argument, index) => (
        <>
          {index > 0 && ', '}
          <ArgumentText argument={argument} />
        </>
      ))}
      {'): '}
      <Types types={returns} />
    </span>
  );
}

export function LiteralValue({ literal }: { literal: Literal }) {
  return literal instanceof Literal.String ? (
    <span class="token string">{JSON.stringify(literal.value)}</span>
  ) : (
    <span class="token number">{String((literal as Literal.Number).value)}</span>
  );
}

export function Types({ types }: { types: readonly TypeText[] }) {
  return (
    <span class="types">
      {types.map((type, index) => (
        <>
          {index > 0 && <span class="or"> or </span>}
          {type.parts.map((part) =>
            part instanceof TypePart.Text ? part.text : <TypeName name={(part as TypePart.Name).name} />
          )}
        </>
      ))}
    </span>
  );
}

function ArgumentText({ argument }: { argument: Argument }) {
  return (
    <span class="argument">
      <span class="argument-name">{argument.optional ? `${argument.name}?` : argument.name}</span>
      {': '}
      <Types types={argument.types} />
      {argument.default !== undefined && (
        <>
          {' = '}
          <LiteralValue literal={argument.default} />
        </>
      )}
    </span>
  );
}
