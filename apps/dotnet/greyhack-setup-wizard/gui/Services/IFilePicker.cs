namespace GreyHackSetupWizard.Gui.Services;

public interface IFilePicker
{
    Task<string?> PickFolderAsync(string title);

    Task<string?> PickFileAsync(string title, string fileKind, IReadOnlyList<string> patterns);
}
