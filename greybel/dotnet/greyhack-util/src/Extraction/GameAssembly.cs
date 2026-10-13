using System.Buffers.Binary;
using System.Reflection.Metadata;
using System.Reflection.Metadata.Ecma335;
using GreyHackUtil.Contracts;
using ICSharpCode.Decompiler;
using ICSharpCode.Decompiler.CSharp;
using ICSharpCode.Decompiler.CSharp.Syntax;
using ICSharpCode.Decompiler.TypeSystem;

namespace GreyHackUtil.Extraction;

internal sealed class GameAssembly
{
    public const byte Call = 0x28;
    public const byte CallVirtual = 0x6F;
    public const byte NewObject = 0x73;

    public const byte LoadFunction = 0x06;
    public const byte LoadVirtualFunction = 0x07;

    private readonly CSharpDecompiler decompiler;

    public IModule Module { get; }

    public GameAssembly(string path)
    {
        decompiler = new CSharpDecompiler(
            path,
            new DecompilerSettings { ThrowOnAssemblyResolveErrors = false }
        );
        Module = decompiler.TypeSystem.MainModule;
    }

    public string? GameVersion =>
        Module
            .TypeDefinitions.Where(t => t.Name == "GameConfig")
            .SelectMany(t => t.Fields)
            .FirstOrDefault(f => f.Name == "gameVersion" && f.IsConst)
            ?.GetConstantValue() as string;

    public ITypeDefinition FindType(string fullName)
    {
        return Module.GetTypeDefinition(new FullTypeName(fullName))
            ?? throw new ContractException($"type {fullName} not found");
    }

    public IMethod FindMethod(string typeName, string methodName)
    {
        IMethod[] methods = FindType(typeName).Methods.Where(m => m.Name == methodName).ToArray();
        return methods.Length == 1
            ? methods[0]
            : throw new ContractException(
                $"expected one {typeName}.{methodName}, found {methods.Length}"
            );
    }

    public SyntaxTree Decompile(IMethod method)
    {
        return decompiler.Decompile(method.MetadataToken);
    }

    public SyntaxTree DecompileType(ITypeDefinition type)
    {
        return decompiler.DecompileType(type.FullTypeName);
    }

    public IReadOnlyList<ITypeDefinition> FindCallerTypes(
        IEnumerable<IMethod> methods,
        params byte[] opcodes
    )
    {
        var file =
            Module.MetadataFile
            ?? throw new ContractException("the game's assembly has no metadata");
        var tokens = methods
            .Select(method => MetadataTokens.GetToken(method.MetadataToken))
            .ToHashSet();
        var types = new HashSet<ITypeDefinition>();
        foreach (ITypeDefinition type in Module.TypeDefinitions)
        {
            foreach (
                IMethod method in type.Methods.Where(m =>
                    !m.MetadataToken.IsNil && m.MetadataToken.Kind == HandleKind.MethodDefinition
                )
            )
            {
                var definition = file.Metadata.GetMethodDefinition(
                    (MethodDefinitionHandle)method.MetadataToken
                );
                if (definition.RelativeVirtualAddress == 0)
                {
                    continue;
                }
                byte[] il =
                    file.GetMethodBody(definition.RelativeVirtualAddress).GetILBytes() ?? [];
                for (int at = 0; at + 5 <= il.Length; at++)
                {
                    if (
                        opcodes.Contains(il[at])
                        && tokens.Contains(
                            BinaryPrimitives.ReadInt32LittleEndian(il.AsSpan(at + 1))
                        )
                    )
                    {
                        types.Add(GetOutermost(type));
                        break;
                    }
                }
            }
        }
        return types.OrderBy(type => type.FullName, StringComparer.Ordinal).ToList();
    }

    public static ITypeDefinition GetOutermost(ITypeDefinition type)
    {
        while (type.DeclaringTypeDefinition != null)
        {
            type = type.DeclaringTypeDefinition;
        }
        return type;
    }

    public static bool IsGameDeclared(IMethod method)
    {
        return method.ParentModule?.IsMainModule == true;
    }
}
