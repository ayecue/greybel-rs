using GreyHackSetupWizard.Gui.Services;

namespace GreyHackSetupWizard.Tests.Support;

internal sealed class FakeFilePicker : IFilePicker
{
    private readonly Queue<string> paths = new();

    public void Enqueue(string path) => paths.Enqueue(path);

    public Task<string?> PickFolderAsync(string title) =>
        Task.FromResult(paths.TryDequeue(out string? path) ? path : null);

    public Task<string?> PickFileAsync(
        string title,
        string fileKind,
        IReadOnlyList<string> patterns
    ) => PickFolderAsync(title);
}
