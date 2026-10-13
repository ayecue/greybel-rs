using System.Text.RegularExpressions;
using GreyHackSetupWizard.Game;

namespace GreyHackSetupWizard.Plugins;

public static partial class PluginConfig
{
    public const int DefaultPort = 7777;

    public static string GetPath(GameLayout game) =>
        Path.Combine(game.ConfigFolder, PluginBuild.MessageHook.Guid + ".cfg");

    public static void SetPort(GameLayout game, int port)
    {
        string path = GetPath(game);
        string text = File.Exists(path) ? File.ReadAllText(path) : "";
        Directory.CreateDirectory(game.ConfigFolder);
        File.WriteAllText(path, WithPort(text, port));
    }

    public static string WithPort(string configText, int port)
    {
        if (GetPortLineRegex().IsMatch(configText))
        {
            return GetPortLineRegex().Replace(configText, $"Port = {port}", 1);
        }
        if (GetGeneralSectionRegex().IsMatch(configText))
        {
            return GetGeneralSectionRegex().Replace(configText, $"[General]\n\nPort = {port}\n", 1);
        }
        return configText.Length == 0
            ? $"[General]\n\nPort = {port}\n"
            : configText.TrimEnd() + $"\n\n[General]\n\nPort = {port}\n";
    }

    public static int? GetPort(GameLayout game)
    {
        string path = GetPath(game);
        if (!File.Exists(path))
        {
            return null;
        }
        Match match = GetPortLineRegex().Match(File.ReadAllText(path));
        return match.Success ? int.Parse(match.Groups[1].Value) : null;
    }

    [GeneratedRegex(@"^\[General\]\r?\n", RegexOptions.Multiline)]
    private static partial Regex GetGeneralSectionRegex();

    [GeneratedRegex(@"^Port[ \t]*=[ \t]*(\d+)[ \t]*(?=\r?$)", RegexOptions.Multiline)]
    private static partial Regex GetPortLineRegex();
}
