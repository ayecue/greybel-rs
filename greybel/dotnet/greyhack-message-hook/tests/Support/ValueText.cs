using System.Collections;
using System.Globalization;
using System.Reflection;

namespace GreyHackMessageHook.Tests.Support;

internal static class ValueText
{
    public static string Format(object? value)
    {
        switch (value)
        {
            case null:
                return "null";
            case string text:
                return '"' + text + '"';
            case byte[] bytes:
                return Convert.ToBase64String(bytes);
            case Enum member:
                return Convert
                    .ToInt64(member, CultureInfo.InvariantCulture)
                    .ToString(CultureInfo.InvariantCulture);
            case UnityEngine.Vector2 vector:
                return string.Create(
                    CultureInfo.InvariantCulture,
                    $"{{x:{vector.x},y:{vector.y}}}"
                );
            case IFormattable number:
                return number.ToString(null, CultureInfo.InvariantCulture);
            case bool flag:
                return flag ? "true" : "false";
            case IEnumerable items:
                return "[" + string.Join(",", items.Cast<object?>().Select(Format)) + "]";
        }
        Type type = value.GetType();
        IEnumerable<(string Name, object? Value)> members =
            type.IsValueType && !type.IsPrimitive
                ? type.GetFields(BindingFlags.Instance | BindingFlags.Public)
                    .Select(field => (field.Name, field.GetValue(value)))
                : type.GetProperties(BindingFlags.Instance | BindingFlags.Public)
                    .Select(property => (property.Name, property.GetValue(value)));
        return "{"
            + string.Join(
                ",",
                members
                    .OrderBy(member => member.Name.ToLowerInvariant())
                    .Select(member => member.Name.ToLowerInvariant() + ":" + Format(member.Value))
            )
            + "}";
    }
}
