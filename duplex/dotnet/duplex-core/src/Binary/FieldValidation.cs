#nullable enable
using System.Collections.Generic;
using System.Linq;

namespace Duplex;

internal static class FieldValidation
{
    public static List<int> Enter(List<int>? path, int position)
    {
        var fieldPath = path ?? new List<int>();
        fieldPath.Add(position);
        return fieldPath;
    }

    public static void Advance(List<int>? path)
    {
        if (path != null && path.Count > 0)
        {
            path[path.Count - 1]++;
        }
    }

    public static ChannelException CreateViolation(List<int>? path, Requirement requirement) =>
        ChannelException.CreateInvalid(
            new Violation(
                path?.Select(position => (uint)position).ToArray() ?? new uint[0],
                requirement
            )
        );

    public static void ValidateLength(List<int>? path, long length, long min, long max)
    {
        if (length < min)
        {
            throw CreateViolation(path, Requirement.MinLength);
        }
        if (length > max)
        {
            throw CreateViolation(path, Requirement.MaxLength);
        }
    }

    public static long ValidateRange(List<int>? path, long value, long min, long max)
    {
        if (value < min)
        {
            throw CreateViolation(path, Requirement.Minimum);
        }
        if (value > max)
        {
            throw CreateViolation(path, Requirement.Maximum);
        }
        return value;
    }

    public static ulong ValidateRange(List<int>? path, ulong value, ulong min, ulong max)
    {
        if (value < min)
        {
            throw CreateViolation(path, Requirement.Minimum);
        }
        if (value > max)
        {
            throw CreateViolation(path, Requirement.Maximum);
        }
        return value;
    }

    public static double ValidateRange(List<int>? path, double value, double min, double max)
    {
        if (!(value >= min))
        {
            throw CreateViolation(path, Requirement.Minimum);
        }
        if (value > max)
        {
            throw CreateViolation(path, Requirement.Maximum);
        }
        return value;
    }

    public static void ValidateMatch(
        List<int>? path,
        Pattern? pattern,
        byte[] bytes,
        int start,
        int end
    )
    {
        if (pattern != null && !pattern.Matches(bytes, start, end))
        {
            throw CreateViolation(path, Requirement.Pattern);
        }
    }
}
