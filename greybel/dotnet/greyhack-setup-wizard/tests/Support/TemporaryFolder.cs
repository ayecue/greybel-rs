namespace GreyHackSetupWizard.Tests.Support;

internal sealed class TemporaryFolder : IDisposable
{
    public string Path { get; } =
        Directory.CreateTempSubdirectory("greyhack-setup-wizard-").FullName;

    public string GetPath(string name) => System.IO.Path.Combine(Path, name);

    public void Dispose() => Directory.Delete(Path, recursive: true);
}
