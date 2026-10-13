using System.Collections.ObjectModel;

namespace GreyHackSetupWizard.Gui.ViewModels;

internal sealed class LogProgress(ObservableCollection<string> log) : IProgress<string>
{
    public void Report(string value) => log.Add(value);
}
