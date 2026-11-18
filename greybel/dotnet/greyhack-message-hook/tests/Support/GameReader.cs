using System.Reflection;
using Duplex;
using GreyHackMessageHook.Protocol;

namespace GreyHackMessageHook.Tests.Support;

internal static class GameReader
{
    public static IEnumerable<object?> Read(NetworkMessages.RpcMessage game, MessageType type)
    {
        MethodInfo getParam = typeof(NetworkMessages.RpcMessage).GetMethod("GetParam")!;
        foreach (FieldType field in type.Fields)
        {
            Type parameterType = ResolveGameType(
                (string)Tag.Find(field.Tags, Converter.GameParamTypeTag)!.Value!
            );
            yield return getParam.MakeGenericMethod(parameterType).Invoke(game, null);
        }
    }

    private static Type ResolveGameType(string name)
    {
        Type? simple = name switch
        {
            "System.String" => typeof(string),
            "System.Int32" => typeof(int),
            "System.Int64" => typeof(long),
            "System.UInt64" => typeof(ulong),
            "System.Boolean" => typeof(bool),
            "System.Single" => typeof(float),
            "System.Double" => typeof(double),
            "System.Byte[]" => typeof(byte[]),
            "System.Int32[]" => typeof(int[]),
            _ => null,
        };
        if (simple != null)
        {
            return simple;
        }
        if (name.Contains("List"))
        {
            return typeof(List<>).MakeGenericType(
                name.Contains("String") ? typeof(string) : typeof(int)
            );
        }
        if (name.Contains("Nullable") && name.Contains("Boolean"))
        {
            return typeof(bool?);
        }
        if (name.Contains("UnityEngine.Vector2"))
        {
            return name.Contains("Nullable")
                ? typeof(UnityEngine.Vector2?)
                : typeof(UnityEngine.Vector2);
        }
        Type gameType =
            AppDomain
                .CurrentDomain.GetAssemblies()
                .Select(assembly => assembly.GetType(name))
                .FirstOrDefault(type => type != null)
            ?? throw new ArgumentException($"cannot find game parameter type {name}");
        return gameType;
    }
}
