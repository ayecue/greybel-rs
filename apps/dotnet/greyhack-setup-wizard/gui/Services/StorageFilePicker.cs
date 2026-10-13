using Avalonia.Controls;
using Avalonia.Platform.Storage;

namespace GreyHackSetupWizard.Gui.Services;

public sealed class StorageFilePicker(TopLevel window) : IFilePicker
{
    public async Task<string?> PickFolderAsync(string title)
    {
        IReadOnlyList<IStorageFolder> folders = await window.StorageProvider.OpenFolderPickerAsync(
            new FolderPickerOpenOptions { Title = title, AllowMultiple = false }
        );
        return folders.Count == 0 ? null : folders[0].TryGetLocalPath();
    }

    public async Task<string?> PickFileAsync(
        string title,
        string fileKind,
        IReadOnlyList<string> patterns
    )
    {
        IReadOnlyList<IStorageFile> files = await window.StorageProvider.OpenFilePickerAsync(
            new FilePickerOpenOptions
            {
                Title = title,
                AllowMultiple = false,
                FileTypeFilter = [new FilePickerFileType(fileKind) { Patterns = patterns }],
            }
        );
        return files.Count == 0 ? null : files[0].TryGetLocalPath();
    }
}
