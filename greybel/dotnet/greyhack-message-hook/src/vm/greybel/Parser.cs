using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using Miniscript;
using Newtonsoft.Json;

namespace GreyHackMessageHook.Greybel
{
    public class Parser : Miniscript.Parser
    {
        private class BackPatch
        {
            public int lineNum;

            public string waitingFor;
        }

        private class JumpPoint
        {
            public int lineNum;

            public string keyword;
        }

        private class ParseState
        {
            public List<TAC.Line> code = new List<TAC.Line>();

            public List<BackPatch> backpatches = new List<BackPatch>();

            public List<JumpPoint> jumpPoints = new List<JumpPoint>();

            public int nextTempNum;

            public void Add(TAC.Line line)
            {
                code.Add(line);
            }

            public void AddBackpatch(string waitFor)
            {
                backpatches.Add(new BackPatch { lineNum = code.Count - 1, waitingFor = waitFor });
            }

            public void AddJumpPoint(string jumpKeyword)
            {
                jumpPoints.Add(new JumpPoint { lineNum = code.Count, keyword = jumpKeyword });
            }

            public JumpPoint CloseJumpPoint(string keyword)
            {
                int num = jumpPoints.Count - 1;
                if (num < 0 || jumpPoints[num].keyword != keyword)
                {
                    throw new CompilerException(
                        string.Format("'end {0}' without matching '{0}'", keyword)
                    );
                }
                JumpPoint result = jumpPoints[num];
                jumpPoints.RemoveAt(num);
                return result;
            }

            public bool IsJumpTarget(int lineNum)
            {
                for (int i = 0; i < code.Count; i++)
                {
                    TAC.Line.Op op = code[i].op;
                    if (
                        (
                            op == TAC.Line.Op.GotoA
                            || op == TAC.Line.Op.GotoAifB
                            || op == TAC.Line.Op.GotoAifNotB
                            || op == TAC.Line.Op.GotoAifTrulyB
                        )
                        && code[i].rhsA is ValNumber
                        && code[i].rhsA.IntValue() == lineNum
                    )
                    {
                        return true;
                    }
                }
                for (int j = 0; j < jumpPoints.Count; j++)
                {
                    if (jumpPoints[j].lineNum == lineNum)
                    {
                        return true;
                    }
                }
                return false;
            }

            public void Patch(string keywordFound, int reservingLines = 0)
            {
                Patch(keywordFound, alsoBreak: false, reservingLines);
            }

            public void Patch(string keywordFound, bool alsoBreak, int reservingLines = 0)
            {
                Value rhsA = TAC.Num(code.Count + reservingLines);
                bool flag = false;
                int num = backpatches.Count - 1;
                while (num >= 0 && !flag)
                {
                    bool flag2 = false;
                    if (backpatches[num].waitingFor == keywordFound)
                    {
                        flag2 = (flag = true);
                    }
                    else
                    {
                        if (!(backpatches[num].waitingFor == "break"))
                        {
                            throw new CompilerException(
                                "'"
                                    + keywordFound
                                    + "' skips expected '"
                                    + backpatches[num].waitingFor
                                    + "'"
                            );
                        }
                        flag2 = alsoBreak;
                    }
                    if (flag2)
                    {
                        code[backpatches[num].lineNum].rhsA = rhsA;
                        backpatches.RemoveAt(num);
                    }
                    num--;
                }
                if (!flag)
                {
                    throw new CompilerException(
                        "'" + keywordFound + "' without matching block starter"
                    );
                }
            }

            public void PatchIfBlock()
            {
                Value rhsA = TAC.Num(code.Count);
                for (int num = backpatches.Count - 1; num >= 0; num--)
                {
                    BackPatch backPatch = backpatches[num];
                    if (backPatch.waitingFor == "if:MARK")
                    {
                        backpatches.RemoveAt(num);
                        return;
                    }
                    if (backPatch.waitingFor == "end if" || backPatch.waitingFor == "else")
                    {
                        code[backPatch.lineNum].rhsA = rhsA;
                        backpatches.RemoveAt(num);
                    }
                    else if (!(backpatches[num].waitingFor == "break"))
                    {
                        throw new CompilerException("'end if' without matching 'if'");
                    }
                }
                throw new CompilerException("'end if' without matching 'if'");
            }
        }

        private delegate Value ExpressionParsingMethod(
            Lexer tokens,
            bool asLval = false,
            bool statementStart = false
        );

        public new string errorContext;

        public bool isExternal = false;
        public string FilePath;

        /// <summary>
        /// True when statement-initial <c>debugger</c> is a breakpoint, matching non-strict Greybel code. Otherwise it
        /// is an identifier, matching the game.
        /// </summary>
        public bool debuggerStatements;

        /// <summary>Maps each parsed line of built code to its original file and line.</summary>
        public SourceMap sourceMap;

        private string partialInput;
        private bool debugMode = false;
        private int tempNumOffset;

        private Stack<ParseState> outputStack;

        private ParseState output;

        private ParseState pendingState;

        public Parser(int tempNumOffset = 0)
        {
            FilePath = "virtual-" + Guid.NewGuid().ToString();
            this.tempNumOffset = tempNumOffset;
            Reset();
        }

        public Parser(bool debugMode, int tempNumOffset = 0)
            : this(tempNumOffset)
        {
            this.debugMode = debugMode;
        }

        public new void Reset()
        {
            int initialTempNum = tempNumOffset;
            if (output != null && output.nextTempNum > initialTempNum)
            {
                initialTempNum = output.nextTempNum;
            }
            output = new ParseState();
            output.nextTempNum = initialTempNum;
            if (outputStack == null)
            {
                outputStack = new Stack<ParseState>();
            }
            else
            {
                outputStack.Clear();
            }
            outputStack.Push(output);
        }

        public new void PartialReset()
        {
            if (outputStack == null)
            {
                outputStack = new Stack<ParseState>();
            }
            while (outputStack.Count > 1)
            {
                outputStack.Pop();
            }
            output = outputStack.Peek();
            output.backpatches.Clear();
            output.jumpPoints.Clear();
            output.nextTempNum = tempNumOffset;
            partialInput = null;
            pendingState = null;
        }

        public new bool NeedMoreInput()
        {
            if (!string.IsNullOrEmpty(partialInput))
            {
                return true;
            }
            if (outputStack.Count > 1)
            {
                return true;
            }
            if (output.backpatches.Count > 0)
            {
                return true;
            }
            return false;
        }

        public static new bool EndsWithLineContinuation(string sourceCode)
        {
            try
            {
                switch (Lexer.LastToken(sourceCode).type)
                {
                    case Token.Type.Unknown:
                    case Token.Type.Keyword:
                    case Token.Type.Number:
                    case Token.Type.String:
                    case Token.Type.Identifier:
                    case Token.Type.RParen:
                    case Token.Type.RSquare:
                    case Token.Type.RCurly:
                    case Token.Type.EOL:
                        return false;
                    default:
                        return true;
                }
            }
            catch (LexerException)
            {
                return false;
            }
        }

        public void Parse(string sourceCode)
        {
            Parse(sourceCode, false, null, false);
        }

        public void Parse(string sourceCode, bool replMode = false)
        {
            Parse(sourceCode, replMode, null, false);
        }

        public new void Parse(
            string sourceCode,
            bool replMode = false,
            PlayerUtils.HelperImport helperImport = null
        )
        {
            Parse(sourceCode, replMode, helperImport, true);
        }

        public void Parse(
            string sourceCode,
            bool replMode = false,
            PlayerUtils.HelperImport helperImport = null,
            bool isNative = false
        )
        {
            if (replMode && EndsWithLineContinuation(sourceCode))
            {
                partialInput += Lexer.TrimComment(sourceCode);
                return;
            }
            Lexer lexer = new Lexer(partialInput + sourceCode);
            partialInput = null;
            ParseMultipleLines(lexer, helperImport, isNative);
            if (!replMode && NeedMoreInput())
            {
                lexer.lineNum++;
                if (outputStack.Count > 1)
                {
                    throw new CompilerException(
                        errorContext,
                        lexer.lineNum,
                        "'function' without matching 'end function'"
                    );
                }
                CheckForOpenBackpatches(lexer.lineNum);
            }
        }

        /// <summary>
        /// Throws if the current function has an open block at <c>end function</c> or end of code
        /// (<c>Parser.CheckForOpenBackpatches</c>).
        /// </summary>
        private void CheckForOpenBackpatches(int sourceLineNum)
        {
            if (output.backpatches.Count == 0)
            {
                return;
            }
            string message;
            switch (output.backpatches[output.backpatches.Count - 1].waitingFor)
            {
                case "end for":
                    message = "'for' without matching 'end for'";
                    break;
                case "end if":
                case "else":
                    message = "'if' without matching 'end if'";
                    break;
                case "end while":
                    message = "'while' without matching 'end while'";
                    break;
                default:
                    message = "unmatched block opener";
                    break;
            }
            throw new CompilerException(errorContext, sourceLineNum, message);
        }

        public new TAC.Machine CreateVM(TextOutputMethod standardOutput)
        {
            return new TAC.Machine(new TAC.Context(output.code), standardOutput);
        }

        public List<TAC.Line> GetCode()
        {
            return output.code;
        }

        public int GetCurrentTempNum()
        {
            return output.nextTempNum;
        }

        public new Function CreateImport()
        {
            ValVar rhsA = new ValVar("locals");
            output.Add(new TAC.Line(TAC.LTemp(0), TAC.Line.Op.ReturnA, rhsA));
            return new Function(output.code);
        }

        public new void REPL(string line)
        {
            Parse(line);
            TAC.Machine machine = CreateVM(null);
            while (!machine.done)
            {
                machine.Step();
            }
        }

        private void AllowLineBreak(Lexer tokens)
        {
            while (tokens.Peek().type == Token.Type.EOL && !tokens.AtEnd)
            {
                tokens.Dequeue();
            }
        }

        private void ParseMultipleLines(
            Lexer tokens,
            PlayerUtils.HelperImport helperImport = null,
            bool isNative = false
        )
        {
            while (!tokens.AtEnd)
            {
                if (tokens.Peek().type == Token.Type.EOL)
                {
                    tokens.Dequeue();
                    continue;
                }
                int updatedLine = tokens.lineNum;
                if (
                    sourceMap != null
                    && sourceMap.TryLocate(tokens.lineNum, out string source, out int sourceLine)
                )
                {
                    errorContext = source;
                    updatedLine = sourceLine;
                }
                else if (helperImport != null)
                {
                    errorContext = helperImport.GetInfoErrorUpdated(
                        tokens.lineNum,
                        out updatedLine
                    );
                }
                else if (!isNative)
                {
                    errorContext = FilePath;
                }
                SourceLoc location = new SourceLoc(errorContext, updatedLine);
                if (
                    tokens.Peek().type == Token.Type.Keyword
                    && tokens.Peek().text == "end function"
                )
                {
                    tokens.Dequeue();
                    if (outputStack.Count > 1)
                    {
                        CheckForOpenBackpatches(updatedLine);
                        outputStack.Pop();
                        output = outputStack.Peek();
                        continue;
                    }
                    throw new CompilerException("'end function' without matching block starter")
                    {
                        location = location,
                    };
                }
                int count = output.code.Count;
                try
                {
                    ParseStatement(tokens);
                }
                catch (MiniscriptException ex)
                {
                    if (ex.location == null)
                    {
                        ex.location = location;
                    }
                    throw;
                }
                for (int i = count; i < output.code.Count; i++)
                {
                    TAC.Line currentLine = output.code[i];
                    currentLine.location = location;
                }
            }
        }

        private void AddDebug()
        {
            if (debugMode)
            {
                output.Add(new TAC.Line(null, (TAC.Line.Op)Op.DebuggerCheck, null));
            }
        }

        private void ParseStatement(Lexer tokens, bool allowExtra = false)
        {
            if (
                debuggerStatements
                && tokens.Peek().type == Token.Type.Identifier
                && tokens.Peek().text == "debugger"
            )
            {
                tokens.Dequeue();
                output.Add(new TAC.Line(null, (TAC.Line.Op)Op.Debugger, null, null));
                return;
            }
            if (
                tokens.Peek().type == Token.Type.Keyword
                && tokens.Peek().text != "not"
                && tokens.Peek().text != "true"
                && tokens.Peek().text != "false"
            )
            {
                string text = tokens.Dequeue().text;
                switch (text)
                {
                    case "return":
                    {
                        AddDebug();
                        Value rhsA = null;
                        if (tokens.Peek().type != Token.Type.EOL)
                        {
                            rhsA = ParseExpr(tokens);
                        }
                        output.Add(new TAC.Line(TAC.LTemp(0), TAC.Line.Op.ReturnA, rhsA));
                        break;
                    }
                    case "if":
                    {
                        AddDebug();
                        Value rhsB2 = ParseExpr(tokens);
                        RequireToken(tokens, Token.Type.Keyword, "then");
                        output.Add(new TAC.Line(null, TAC.Line.Op.GotoAifNotB, null, rhsB2));
                        output.AddBackpatch("if:MARK");
                        output.AddBackpatch("else");
                        if (tokens.Peek().type != Token.Type.EOL)
                        {
                            ParseStatement(tokens, allowExtra: true);
                            if (
                                tokens.Peek().type == Token.Type.Keyword
                                && tokens.Peek().text == "else"
                            )
                            {
                                tokens.Dequeue();
                                StartElseClause();
                                ParseStatement(tokens, allowExtra: true);
                            }
                            else
                            {
                                RequireEitherToken(
                                    tokens,
                                    Token.Type.Keyword,
                                    "else",
                                    Token.Type.EOL
                                );
                            }
                            output.PatchIfBlock();
                        }
                        else
                        {
                            tokens.Dequeue();
                        }
                        return;
                    }
                    case "else":
                        StartElseClause();
                        break;
                    case "else if":
                    {
                        AddDebug();
                        StartElseClause();
                        Value rhsB = ParseExpr(tokens);
                        RequireToken(tokens, Token.Type.Keyword, "then");
                        output.Add(new TAC.Line(null, TAC.Line.Op.GotoAifNotB, null, rhsB));
                        output.AddBackpatch("else");
                        break;
                    }
                    case "end if":
                        output.PatchIfBlock();
                        break;
                    case "while":
                    {
                        AddDebug();
                        output.AddJumpPoint(text);
                        Value rhsB3 = ParseExpr(tokens);
                        output.Add(new TAC.Line(null, TAC.Line.Op.GotoAifNotB, null, rhsB3));
                        output.AddBackpatch("end while");
                        break;
                    }
                    case "end while":
                    {
                        JumpPoint jumpPoint3 = output.CloseJumpPoint("while");
                        output.Add(
                            new TAC.Line(null, TAC.Line.Op.GotoA, TAC.Num(jumpPoint3.lineNum))
                        );
                        output.Patch(text, alsoBreak: true);
                        break;
                    }
                    case "for":
                    {
                        AddDebug();
                        Token token = RequireToken(tokens, Token.Type.Identifier);
                        ValVar lhs = new ValVar(token.text);
                        RequireToken(tokens, Token.Type.Keyword, "in");
                        Value value = ParseExpr(tokens);
                        if (value == null)
                        {
                            throw new CompilerException(
                                errorContext,
                                tokens.lineNum,
                                "sequence expression expected for 'for' loop"
                            );
                        }
                        ValVar valVar = new ValVar("__" + token.text + "_idx");
                        output.Add(new TAC.Line(valVar, TAC.Line.Op.AssignA, TAC.Num(-1.0)));
                        output.AddJumpPoint(text);
                        output.Add(new TAC.Line(valVar, TAC.Line.Op.APlusB, valVar, TAC.Num(1.0)));
                        ValTemp valTemp = new ValTemp(output.nextTempNum++);
                        output.Add(new TAC.Line(valTemp, TAC.Line.Op.LengthOfA, value));
                        ValTemp valTemp2 = new ValTemp(output.nextTempNum++);
                        output.Add(
                            new TAC.Line(valTemp2, TAC.Line.Op.AGreatOrEqualB, valVar, valTemp)
                        );
                        output.Add(new TAC.Line(null, TAC.Line.Op.GotoAifB, null, valTemp2));
                        output.AddBackpatch("end for");
                        output.Add(new TAC.Line(lhs, TAC.Line.Op.ElemBofIterA, value, valVar));
                        break;
                    }
                    case "end for":
                    {
                        JumpPoint jumpPoint2 = output.CloseJumpPoint("for");
                        output.Add(
                            new TAC.Line(null, TAC.Line.Op.GotoA, TAC.Num(jumpPoint2.lineNum))
                        );
                        output.Patch(text, alsoBreak: true);
                        break;
                    }
                    case "break":
                        AddDebug();
                        output.Add(new TAC.Line(null, TAC.Line.Op.GotoA));
                        output.AddBackpatch("break");
                        break;
                    case "continue":
                    {
                        AddDebug();
                        if (output.jumpPoints.Count == 0)
                        {
                            throw new CompilerException(
                                errorContext,
                                tokens.lineNum,
                                "'continue' without open loop block"
                            );
                        }
                        JumpPoint jumpPoint = output.jumpPoints.Last();
                        output.Add(
                            new TAC.Line(null, TAC.Line.Op.GotoA, TAC.Num(jumpPoint.lineNum))
                        );
                        break;
                    }
                    default:
                        throw new CompilerException(
                            errorContext,
                            tokens.lineNum,
                            "unexpected keyword '" + text + "' at start of line"
                        );
                }
            }
            else
            {
                AddDebug();
                ParseAssignment(tokens, allowExtra);
            }
            if (!allowExtra)
            {
                RequireToken(tokens, Token.Type.EOL);
            }
            if (pendingState != null)
            {
                output = pendingState;
                outputStack.Push(output);
                pendingState = null;
            }
        }

        private void StartElseClause()
        {
            output.Add(new TAC.Line(null, TAC.Line.Op.GotoA));
            output.Patch("else");
            output.AddBackpatch("end if");
        }

        private void ParseAssignment(Lexer tokens, bool allowExtra = false)
        {
            Value value = ParseExpr(tokens, asLval: true, statementStart: true);
            Token token = tokens.Peek();
            if (
                token.type == Token.Type.EOL
                || (token.type == Token.Type.Keyword && token.text == "else")
            )
            {
                Value rhsA = FullyEvaluate(value);
                output.Add(new TAC.Line(null, TAC.Line.Op.AssignImplicit, rhsA));
                return;
            }
            if (token.type == Token.Type.OpAssign)
            {
                tokens.Dequeue();
                Value lhs = value;
                Value rhsA = ParseExpr(tokens);
                if (
                    rhsA is ValTemp
                    && output.code.Count > 0
                    && !output.IsJumpTarget(output.code.Count)
                )
                {
                    TAC.Line line = output.code[output.code.Count - 1];
                    if (line.lhs.Equals(rhsA))
                    {
                        line.lhs = lhs;
                        return;
                    }
                }
                output.Add(new TAC.Line(lhs, TAC.Line.Op.AssignA, rhsA));
                return;
            }
            Value rhsA2 = value;
            int num = 0;
            while (true)
            {
                Value rhsA3 = ParseExpr(tokens);
                output.Add(new TAC.Line(null, TAC.Line.Op.PushParam, rhsA3));
                num++;
                if (
                    tokens.Peek().type == Token.Type.EOL
                    || (tokens.Peek().type == Token.Type.Keyword && tokens.Peek().text == "else")
                )
                {
                    break;
                }
                if (tokens.Peek().type == Token.Type.Comma)
                {
                    tokens.Dequeue();
                    AllowLineBreak(tokens);
                }
                else if (
                    RequireEitherToken(tokens, Token.Type.Comma, Token.Type.EOL).type
                    == Token.Type.EOL
                )
                {
                    break;
                }
            }
            ValTemp valTemp = new ValTemp(output.nextTempNum++);
            output.Add(new TAC.Line(valTemp, TAC.Line.Op.CallFunctionA, rhsA2, TAC.Num(num)));
            output.Add(new TAC.Line(null, TAC.Line.Op.AssignImplicit, valTemp));
        }

        private Value ParseExpr(Lexer tokens, bool asLval = false, bool statementStart = false)
        {
            return new ExpressionParsingMethod(ParseFunction)(tokens, asLval, statementStart);
        }

        private Value ParseFunction(Lexer tokens, bool asLval = false, bool statementStart = false)
        {
            ExpressionParsingMethod nextLevel = ParseOr;
            Token token = tokens.Peek();
            if (token.type != Token.Type.Keyword || token.text != "function")
            {
                return nextLevel(tokens, asLval, statementStart);
            }
            tokens.Dequeue();
            Function function = new Function(null);
            token = tokens.Peek();
            if (token.type != Token.Type.EOL)
            {
                RequireToken(tokens, Token.Type.LParen);
                while (tokens.Peek().type != Token.Type.RParen)
                {
                    Token token2 = tokens.Dequeue();
                    if (token2.type != Token.Type.Identifier)
                    {
                        throw new CompilerException(
                            errorContext,
                            tokens.lineNum,
                            "got " + token2?.ToString() + " where an identifier is required"
                        );
                    }
                    Value defaultValue = null;
                    if (tokens.Peek().type == Token.Type.OpAssign)
                    {
                        tokens.Dequeue();
                        defaultValue = ParseExpr(tokens);
                        if (defaultValue is ValTemp)
                        {
                            throw new CompilerException(
                                errorContext,
                                tokens.lineNum,
                                "parameter default value must be a literal value"
                            );
                        }
                    }
                    function.parameters.Add(new Function.Param(token2.text, defaultValue));
                    if (tokens.Peek().type == Token.Type.RParen)
                    {
                        break;
                    }
                    RequireToken(tokens, Token.Type.Comma);
                }
                RequireToken(tokens, Token.Type.RParen);
            }
            if (pendingState != null)
            {
                throw new CompilerException(
                    errorContext,
                    tokens.lineNum,
                    "can't start two functions in one statement"
                );
            }
            pendingState = new ParseState();
            pendingState.nextTempNum = tempNumOffset + 1;
            function.code = pendingState.code;
            ValFunction valFunction = new ValFunction(function);
            ValTemp valTemp = new ValTemp(output.nextTempNum++);
            output.Add(new TAC.Line(valTemp, TAC.Line.Op.BindAssignA, valFunction));
            return valTemp;
        }

        private Value ParseOr(Lexer tokens, bool asLval = false, bool statementStart = false)
        {
            ExpressionParsingMethod nextLevel = ParseAnd;
            Value value = nextLevel(tokens, asLval, statementStart);
            List<TAC.Line> list = null;
            Token token = tokens.Peek();
            while (token.type == Token.Type.Keyword && token.text == "or")
            {
                tokens.Dequeue();
                value = FullyEvaluate(value);
                AllowLineBreak(tokens);
                TAC.Line line = new TAC.Line(null, TAC.Line.Op.GotoAifTrulyB, null, value);
                output.Add(line);
                if (list == null)
                {
                    list = new List<TAC.Line>();
                }
                list.Add(line);
                Value rhsB = nextLevel(tokens);
                int tempNum = output.nextTempNum++;
                output.Add(new TAC.Line(TAC.LTemp(tempNum), TAC.Line.Op.AOrB, value, rhsB));
                value = TAC.RTemp(tempNum);
                token = tokens.Peek();
            }
            if (list != null)
            {
                output.Add(new TAC.Line(null, TAC.Line.Op.GotoA, TAC.Num(output.code.Count + 2)));
                output.Add(new TAC.Line(value, TAC.Line.Op.AssignA, ValNumber.one));
                foreach (TAC.Line item in list)
                {
                    item.rhsA = TAC.Num(output.code.Count - 1);
                }
            }
            return value;
        }

        private Value ParseAnd(Lexer tokens, bool asLval = false, bool statementStart = false)
        {
            ExpressionParsingMethod nextLevel = ParseNot;
            Value value = nextLevel(tokens, asLval, statementStart);
            List<TAC.Line> list = null;
            Token token = tokens.Peek();
            while (token.type == Token.Type.Keyword && token.text == "and")
            {
                tokens.Dequeue();
                value = FullyEvaluate(value);
                AllowLineBreak(tokens);
                TAC.Line line = new TAC.Line(null, TAC.Line.Op.GotoAifNotB, null, value);
                output.Add(line);
                if (list == null)
                {
                    list = new List<TAC.Line>();
                }
                list.Add(line);
                Value rhsB = nextLevel(tokens);
                int tempNum = output.nextTempNum++;
                output.Add(new TAC.Line(TAC.LTemp(tempNum), TAC.Line.Op.AAndB, value, rhsB));
                value = TAC.RTemp(tempNum);
                token = tokens.Peek();
            }
            if (list != null)
            {
                output.Add(new TAC.Line(null, TAC.Line.Op.GotoA, TAC.Num(output.code.Count + 2)));
                output.Add(new TAC.Line(value, TAC.Line.Op.AssignA, ValNumber.zero));
                foreach (TAC.Line item in list)
                {
                    item.rhsA = TAC.Num(output.code.Count - 1);
                }
            }
            return value;
        }

        private Value ParseNot(Lexer tokens, bool asLval = false, bool statementStart = false)
        {
            ExpressionParsingMethod nextLevel = ParseIsA;
            Token token = tokens.Peek();
            if (token.type == Token.Type.Keyword && token.text == "not")
            {
                tokens.Dequeue();
                AllowLineBreak(tokens);
                Value rhsA = nextLevel(tokens);
                int tempNum = output.nextTempNum++;
                output.Add(new TAC.Line(TAC.LTemp(tempNum), TAC.Line.Op.NotA, rhsA));
                return TAC.RTemp(tempNum);
            }
            return nextLevel(tokens, asLval, statementStart);
        }

        private Value ParseIsA(Lexer tokens, bool asLval = false, bool statementStart = false)
        {
            ExpressionParsingMethod nextLevel = ParseComparisons;
            Value value = nextLevel(tokens, asLval, statementStart);
            if (tokens.Peek().type == Token.Type.Keyword && tokens.Peek().text == "isa")
            {
                tokens.Dequeue();
                AllowLineBreak(tokens);
                value = FullyEvaluate(value);
                Value rhsB = nextLevel(tokens);
                int tempNum = output.nextTempNum++;
                output.Add(new TAC.Line(TAC.LTemp(tempNum), TAC.Line.Op.AisaB, value, rhsB));
                value = TAC.RTemp(tempNum);
            }
            return value;
        }

        private Value ParseComparisons(
            Lexer tokens,
            bool asLval = false,
            bool statementStart = false
        )
        {
            ExpressionParsingMethod nextLevel = ParseAddSub;
            Value value = nextLevel(tokens, asLval, statementStart);
            Value val = value;
            TAC.Line.Op op = ComparisonOp(tokens.Peek().type);
            bool flag = true;
            while (op != 0)
            {
                tokens.Dequeue();
                val = FullyEvaluate(val);
                AllowLineBreak(tokens);
                Value value2 = nextLevel(tokens);
                int num = output.nextTempNum++;
                output.Add(new TAC.Line(TAC.LTemp(num), op, val, value2));
                if (flag)
                {
                    flag = false;
                }
                else
                {
                    num = output.nextTempNum++;
                    output.Add(
                        new TAC.Line(TAC.LTemp(num), TAC.Line.Op.ATimesB, value, TAC.RTemp(num - 1))
                    );
                }
                value = TAC.RTemp(num);
                val = value2;
                op = ComparisonOp(tokens.Peek().type);
            }
            return value;
        }

        private static TAC.Line.Op ComparisonOp(Token.Type tokenType)
        {
            return tokenType switch
            {
                Token.Type.OpEqual => TAC.Line.Op.AEqualB,
                Token.Type.OpNotEqual => TAC.Line.Op.ANotEqualB,
                Token.Type.OpGreater => TAC.Line.Op.AGreaterThanB,
                Token.Type.OpGreatEqual => TAC.Line.Op.AGreatOrEqualB,
                Token.Type.OpLesser => TAC.Line.Op.ALessThanB,
                Token.Type.OpLessEqual => TAC.Line.Op.ALessOrEqualB,
                _ => TAC.Line.Op.Noop,
            };
        }

        private Value ParseAddSub(Lexer tokens, bool asLval = false, bool statementStart = false)
        {
            ExpressionParsingMethod nextLevel = ParseMultDiv;
            Value value = nextLevel(tokens, asLval, statementStart);
            Token token = tokens.Peek();
            while (
                token.type == Token.Type.OpPlus
                || (
                    token.type == Token.Type.OpMinus
                    && (!statementStart || !token.afterSpace || tokens.IsAtWhitespace())
                )
            )
            {
                tokens.Dequeue();
                AllowLineBreak(tokens);
                value = FullyEvaluate(value);
                Value rhsB = nextLevel(tokens);
                int tempNum = output.nextTempNum++;
                output.Add(
                    new TAC.Line(
                        TAC.LTemp(tempNum),
                        (token.type == Token.Type.OpPlus)
                            ? TAC.Line.Op.APlusB
                            : TAC.Line.Op.AMinusB,
                        value,
                        rhsB
                    )
                );
                value = TAC.RTemp(tempNum);
                token = tokens.Peek();
            }
            return value;
        }

        private Value ParseMultDiv(Lexer tokens, bool asLval = false, bool statementStart = false)
        {
            ExpressionParsingMethod nextLevel = ParseUnaryMinus;
            Value value = nextLevel(tokens, asLval, statementStart);
            Token token = tokens.Peek();
            while (
                token.type == Token.Type.OpTimes
                || token.type == Token.Type.OpDivide
                || token.type == Token.Type.OpMod
            )
            {
                tokens.Dequeue();
                AllowLineBreak(tokens);
                value = FullyEvaluate(value);
                Value rhsB = nextLevel(tokens);
                int tempNum = output.nextTempNum++;
                switch (token.type)
                {
                    case Token.Type.OpTimes:
                        output.Add(
                            new TAC.Line(TAC.LTemp(tempNum), TAC.Line.Op.ATimesB, value, rhsB)
                        );
                        break;
                    case Token.Type.OpDivide:
                        output.Add(
                            new TAC.Line(TAC.LTemp(tempNum), TAC.Line.Op.ADividedByB, value, rhsB)
                        );
                        break;
                    case Token.Type.OpMod:
                        output.Add(
                            new TAC.Line(TAC.LTemp(tempNum), TAC.Line.Op.AModB, value, rhsB)
                        );
                        break;
                }
                value = TAC.RTemp(tempNum);
                token = tokens.Peek();
            }
            return value;
        }

        private Value ParseUnaryMinus(
            Lexer tokens,
            bool asLval = false,
            bool statementStart = false
        )
        {
            ExpressionParsingMethod nextLevel = ParseNew;
            if (tokens.Peek().type != Token.Type.OpMinus)
            {
                return nextLevel(tokens, asLval, statementStart);
            }
            tokens.Dequeue();
            AllowLineBreak(tokens);
            Value value = nextLevel(tokens);
            if (value is ValNumber)
            {
                ValNumber obj = (ValNumber)value;
                obj.value = 0.0 - obj.value;
                return obj;
            }
            int tempNum = output.nextTempNum++;
            output.Add(new TAC.Line(TAC.LTemp(tempNum), TAC.Line.Op.AMinusB, TAC.Num(0.0), value));
            return TAC.RTemp(tempNum);
        }

        private Value ParseNew(Lexer tokens, bool asLval = false, bool statementStart = false)
        {
            ExpressionParsingMethod nextLevel = ParseAddressOf;
            if (tokens.Peek().type != Token.Type.Keyword || tokens.Peek().text != "new")
            {
                return nextLevel(tokens, asLval, statementStart);
            }
            tokens.Dequeue();
            AllowLineBreak(tokens);
            Value value = nextLevel(tokens);
            ValMap valMap = new ValMap();
            valMap.SetElem(ValString.magicIsA, value);
            Value value2 = new ValTemp(output.nextTempNum++);
            output.Add(new TAC.Line(value2, TAC.Line.Op.CopyA, valMap));
            return value2;
        }

        private Value ParseAddressOf(Lexer tokens, bool asLval = false, bool statementStart = false)
        {
            ExpressionParsingMethod nextLevel = ParsePower;
            if (tokens.Peek().type != Token.Type.AddressOf)
            {
                return nextLevel(tokens, asLval, statementStart);
            }
            tokens.Dequeue();
            AllowLineBreak(tokens);
            Value value = nextLevel(tokens, asLval: true, statementStart);
            if (value is ValVar)
            {
                ((ValVar)value).noInvoke = true;
            }
            else if (value is ValSeqElem)
            {
                ((ValSeqElem)value).noInvoke = true;
            }
            return value;
        }

        private Value ParsePower(Lexer tokens, bool asLval = false, bool statementStart = false)
        {
            ExpressionParsingMethod nextLevel = ParseCallExpr;
            Value value = nextLevel(tokens, asLval, statementStart);
            Token token = tokens.Peek();
            while (token.type == Token.Type.OpPower)
            {
                tokens.Dequeue();
                AllowLineBreak(tokens);
                value = FullyEvaluate(value);
                Value rhsB = nextLevel(tokens);
                int tempNum = output.nextTempNum++;
                output.Add(new TAC.Line(TAC.LTemp(tempNum), TAC.Line.Op.APowB, value, rhsB));
                value = TAC.RTemp(tempNum);
                token = tokens.Peek();
            }
            return value;
        }

        private Value FullyEvaluate(Value val)
        {
            if (val is ValVar)
            {
                ValVar valVar = (ValVar)val;
                if (valVar.noInvoke)
                {
                    return val;
                }
                if (valVar.identifier == "super" || valVar.identifier == "self")
                {
                    return val;
                }
                ValTemp valTemp = new ValTemp(output.nextTempNum++);
                output.Add(new TAC.Line(valTemp, TAC.Line.Op.CallFunctionA, val, ValNumber.zero));
                return valTemp;
            }
            if (val is ValSeqElem)
            {
                if (((ValSeqElem)val).noInvoke)
                {
                    return val;
                }
                ValTemp valTemp2 = new ValTemp(output.nextTempNum++);
                output.Add(new TAC.Line(valTemp2, TAC.Line.Op.CallFunctionA, val, ValNumber.zero));
                return valTemp2;
            }
            return val;
        }

        private Value ParseCallExpr(Lexer tokens, bool asLval = false, bool statementStart = false)
        {
            Value value = new ExpressionParsingMethod(ParseMap)(tokens, asLval, statementStart);
            while (true)
            {
                if (tokens.Peek().type == Token.Type.Dot)
                {
                    tokens.Dequeue();
                    AllowLineBreak(tokens);
                    Token token = RequireToken(tokens, Token.Type.Identifier);
                    value = FullyEvaluate(value);
                    value = new ValSeqElem(value, new ValString(token.text));
                    if (tokens.Peek().type == Token.Type.LParen && !tokens.Peek().afterSpace)
                    {
                        value = ParseCallArgs(value, tokens);
                    }
                }
                else if (tokens.Peek().type == Token.Type.LSquare && !tokens.Peek().afterSpace)
                {
                    tokens.Dequeue();
                    AllowLineBreak(tokens);
                    value = FullyEvaluate(value);
                    if (tokens.Peek().type == Token.Type.Colon)
                    {
                        tokens.Dequeue();
                        AllowLineBreak(tokens);
                        Value toIdx = null;
                        if (tokens.Peek().type != Token.Type.RSquare)
                        {
                            toIdx = ParseExpr(tokens);
                        }
                        ValTemp valTemp = new ValTemp(output.nextTempNum++);
                        Intrinsics.CompileSlice(output.code, value, null, toIdx, valTemp.tempNum);
                        value = valTemp;
                    }
                    else
                    {
                        Value value2 = ParseExpr(tokens);
                        if (tokens.Peek().type == Token.Type.Colon)
                        {
                            tokens.Dequeue();
                            AllowLineBreak(tokens);
                            Value toIdx2 = null;
                            if (tokens.Peek().type != Token.Type.RSquare)
                            {
                                toIdx2 = ParseExpr(tokens);
                            }
                            ValTemp valTemp2 = new ValTemp(output.nextTempNum++);
                            Intrinsics.CompileSlice(
                                output.code,
                                value,
                                value2,
                                toIdx2,
                                valTemp2.tempNum
                            );
                            value = valTemp2;
                        }
                        else if (statementStart)
                        {
                            if (value is ValSeqElem)
                            {
                                ValSeqElem valSeqElem = (ValSeqElem)value;
                                ValTemp valTemp3 = new ValTemp(output.nextTempNum++);
                                output.Add(
                                    new TAC.Line(
                                        valTemp3,
                                        TAC.Line.Op.ElemBofA,
                                        valSeqElem.sequence,
                                        valSeqElem.index
                                    )
                                );
                                value = valTemp3;
                            }
                            value = new ValSeqElem(value, value2);
                        }
                        else
                        {
                            ValTemp valTemp4 = new ValTemp(output.nextTempNum++);
                            output.Add(new TAC.Line(valTemp4, TAC.Line.Op.ElemBofA, value, value2));
                            value = valTemp4;
                        }
                    }
                    RequireToken(tokens, Token.Type.RSquare);
                }
                else
                {
                    if (
                        ((!(value is ValVar) || ((ValVar)value).noInvoke) && !(value is ValSeqElem))
                        || (
                            asLval
                            && (tokens.Peek().type != Token.Type.LParen || tokens.Peek().afterSpace)
                        )
                    )
                    {
                        break;
                    }
                    value = ParseCallArgs(value, tokens);
                }
            }
            return value;
        }

        private Value ParseMap(Lexer tokens, bool asLval = false, bool statementStart = false)
        {
            ExpressionParsingMethod nextLevel = ParseList;
            if (tokens.Peek().type != Token.Type.LCurly)
            {
                return nextLevel(tokens, asLval, statementStart);
            }
            tokens.Dequeue();
            ValMap valMap = new ValMap();
            if (tokens.Peek().type == Token.Type.RCurly)
            {
                tokens.Dequeue();
            }
            else
            {
                do
                {
                    AllowLineBreak(tokens);
                    if (tokens.Peek().type == Token.Type.RCurly)
                    {
                        tokens.Dequeue();
                        break;
                    }
                    Value value = ParseExpr(tokens);
                    RequireToken(tokens, Token.Type.Colon);
                    AllowLineBreak(tokens);
                    Value value2 = ParseExpr(tokens);
                    valMap.map[value ?? ValNull.instance] = value2;
                } while (
                    RequireEitherToken(tokens, Token.Type.Comma, Token.Type.RCurly).type
                    != Token.Type.RCurly
                );
            }
            Value value3 = new ValTemp(output.nextTempNum++);
            output.Add(new TAC.Line(value3, TAC.Line.Op.CopyA, valMap));
            return value3;
        }

        private Value ParseList(Lexer tokens, bool asLval = false, bool statementStart = false)
        {
            ExpressionParsingMethod nextLevel = ParseQuantity;
            if (tokens.Peek().type != Token.Type.LSquare)
            {
                return nextLevel(tokens, asLval, statementStart);
            }
            tokens.Dequeue();
            ValList valList = new ValList();
            if (tokens.Peek().type == Token.Type.RSquare)
            {
                tokens.Dequeue();
            }
            else
            {
                do
                {
                    AllowLineBreak(tokens);
                    if (tokens.Peek().type == Token.Type.RSquare)
                    {
                        tokens.Dequeue();
                        break;
                    }
                    Value item = ParseExpr(tokens);
                    valList.values.Add(item);
                } while (
                    RequireEitherToken(tokens, Token.Type.Comma, Token.Type.RSquare).type
                    != Token.Type.RSquare
                );
            }
            if (statementStart)
            {
                return valList;
            }
            Value value = new ValTemp(output.nextTempNum++);
            output.Add(new TAC.Line(value, TAC.Line.Op.CopyA, valList));
            return value;
        }

        private Value ParseQuantity(Lexer tokens, bool asLval = false, bool statementStart = false)
        {
            ExpressionParsingMethod nextLevel = ParseAtom;
            if (tokens.Peek().type != Token.Type.LParen)
            {
                return nextLevel(tokens, asLval, statementStart);
            }
            tokens.Dequeue();
            AllowLineBreak(tokens);
            Value result = ParseExpr(tokens);
            RequireToken(tokens, Token.Type.RParen);
            return result;
        }

        private Value ParseCallArgs(Value funcRef, Lexer tokens)
        {
            int num = 0;
            if (tokens.Peek().type == Token.Type.LParen)
            {
                tokens.Dequeue();
                if (tokens.Peek().type == Token.Type.RParen)
                {
                    tokens.Dequeue();
                }
                else
                {
                    do
                    {
                        AllowLineBreak(tokens);
                        Value rhsA = ParseExpr(tokens);
                        output.Add(new TAC.Line(null, TAC.Line.Op.PushParam, rhsA));
                        num++;
                    } while (
                        RequireEitherToken(tokens, Token.Type.Comma, Token.Type.RParen).type
                        != Token.Type.RParen
                    );
                }
            }
            ValTemp valTemp = new ValTemp(output.nextTempNum++);
            output.Add(new TAC.Line(valTemp, TAC.Line.Op.CallFunctionA, funcRef, TAC.Num(num)));
            return valTemp;
        }

        private Value ParseAtom(Lexer tokens, bool asLval = false, bool statementStart = false)
        {
            Token token = (!tokens.AtEnd) ? tokens.Dequeue() : Token.EOL;
            if (token.type == Token.Type.Number)
            {
                if (
                    double.TryParse(
                        token.text,
                        NumberStyles.Number | NumberStyles.AllowExponent,
                        CultureInfo.InvariantCulture,
                        out var result
                    )
                )
                {
                    return new ValNumber(result);
                }
                throw new CompilerException("invalid numeric literal: " + token.text);
            }
            if (token.type == Token.Type.String)
            {
                return new ValString(token.text);
            }
            if (token.type == Token.Type.Identifier)
            {
                if (token.text == "self")
                {
                    return ValVar.self;
                }
                return new ValVar(token.text);
            }
            if (token.type == Token.Type.Keyword)
            {
                switch (token.text)
                {
                    case "null":
                        return null;
                    case "true":
                        return ValNumber.one;
                    case "false":
                        return ValNumber.zero;
                }
            }
            throw new CompilerException(
                $"got {token} where number, string, or identifier is required"
            );
        }

        private Token RequireToken(Lexer tokens, Token.Type type, string text = null)
        {
            Token token = tokens.AtEnd ? Token.EOL : tokens.Dequeue();
            if (token.type != type || (text != null && token.text != text))
            {
                Token arg = new Token(type, text);
                throw new CompilerException(
                    errorContext,
                    tokens.lineNum,
                    $"got {token} where {arg} is required"
                );
            }
            return token;
        }

        private Token RequireEitherToken(
            Lexer tokens,
            Token.Type type1,
            string text1,
            Token.Type type2,
            string text2 = null
        )
        {
            Token token = tokens.AtEnd ? Token.EOL : tokens.Dequeue();
            if (
                (token.type != type1 && token.type != type2)
                || (text1 != null && token.text != text1 && text2 != null && token.text != text2)
            )
            {
                Token arg = new Token(type1, text1);
                Token arg2 = new Token(type2, text2);
                throw new CompilerException(
                    errorContext,
                    tokens.lineNum,
                    $"got {token} where {arg} or {arg2} is required"
                );
            }
            return token;
        }

        private Token RequireEitherToken(
            Lexer tokens,
            Token.Type type1,
            Token.Type type2,
            string text2 = null
        )
        {
            return RequireEitherToken(tokens, type1, null, type2, text2);
        }
    }
}
