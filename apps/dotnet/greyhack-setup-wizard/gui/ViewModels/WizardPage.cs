using CommunityToolkit.Mvvm.ComponentModel;

namespace GreyHackSetupWizard.Gui.ViewModels;

public abstract partial class WizardPage : ObservableObject
{
    public abstract string Title { get; }

    public abstract string? Problem { get; }

    [ObservableProperty]
    public partial bool IsCurrent { get; set; }

    public virtual Task EnterAsync() => Task.CompletedTask;
}
