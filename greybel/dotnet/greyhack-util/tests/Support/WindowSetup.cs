using GreyHackUtil.Contracts;
using GreyHackUtil.Extraction;

namespace GreyHackUtil.Tests.Support;

internal static class WindowSetup
{
    internal static FieldContract CreateIntField(string name) =>
        new() { Name = name, FieldType = "intValues" };

    internal static SentValue CreateValue(bool window = false, bool own = false) =>
        new("intValues", null, window, own, null);

    internal static SentLayout CreateSent(params SentValue[] values) =>
        new("Sender.Send", null, values);
}
