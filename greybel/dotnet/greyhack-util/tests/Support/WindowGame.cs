using GreyHackUtil.Extraction;
using ICSharpCode.Decompiler.CSharp.Syntax;

namespace GreyHackUtil.Tests.Support;

internal static class WindowGame
{
    private static readonly GameAssembly Game = new(typeof(Ventana).Assembly.Location);

    public static (WindowPidReader Reader, Expression Value, AstNode Scope) FindSent(
        string type,
        string method
    )
    {
        SyntaxTree tree = Game.DecompileType(Game.FindType(type));
        MethodDeclaration declaration = tree
            .Descendants.OfType<MethodDeclaration>()
            .Single(m => m.Name == method);
        InvocationExpression send = declaration
            .Descendants.OfType<InvocationExpression>()
            .Single(call => call.Target is MemberReferenceExpression { MemberName: "Send" });
        return (new WindowPidReader(Game), send.Arguments.Single(), declaration);
    }
}
