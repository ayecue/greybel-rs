/** @import { Rule } from 'eslint' */
/** @import * as ESTree from 'estree' */

/** Statements that exit their function or loop. */
const EXITS = new Set(['ReturnStatement', 'ThrowStatement', 'ContinueStatement', 'BreakStatement']);

/**
 * Requires an early exit rather than an `if` wrapping the remainder of a function or loop body: the last statement of a
 * body may not be an `if` without `else`. `if (ready) start();` ending a function becomes
 * `if (!ready) return; start();`, and `continue` in a loop.
 *
 * @type {Rule.RuleModule}
 */
const earlyExit = {
  meta: {
    type: 'suggestion',
    docs: { description: 'Require an early exit over an `if` wrapping the remainder of a function or loop body' },
    messages: {
      earlyExit: 'Exit early with `if (…) {{exit}};` rather than wrapping the remainder of the body in an `if`.'
    },
    schema: []
  },
  create(context) {
    /**
     * Reports the `if` ending `body`, which `exit` skips.
     *
     * @param {ESTree.Node} body
     * @param {'return' | 'continue'} exit
     */
    function check(body, exit) {
      if (body.type !== 'BlockStatement') return;
      const last = body.body.at(-1);

      if (last?.type !== 'IfStatement' || last.alternate) return;
      if (exitsAlone(last.consequent)) return;
      context.report({ node: last, messageId: 'earlyExit', data: { exit } });
    }

    /** @param {ESTree.Function} node */
    const functionBody = (node) => check(node.body, 'return');
    /** @param {ESTree.ForStatement | ESTree.ForInStatement | ESTree.ForOfStatement | ESTree.WhileStatement | ESTree.DoWhileStatement} node */
    const loopBody = (node) => check(node.body, 'continue');

    return {
      FunctionDeclaration: functionBody,
      FunctionExpression: functionBody,
      ArrowFunctionExpression: functionBody,
      ForStatement: loopBody,
      ForInStatement: loopBody,
      ForOfStatement: loopBody,
      WhileStatement: loopBody,
      DoWhileStatement: loopBody
    };
  }
};

/**
 * Whether `statement` contains nothing but an exit, which an early exit would only repeat.
 *
 * @param {ESTree.Statement} statement
 */
function exitsAlone(statement) {
  if (statement.type !== 'BlockStatement') return EXITS.has(statement.type);

  return statement.body.length === 0 || (statement.body.length === 1 && EXITS.has(statement.body[0].type));
}

export default earlyExit;
