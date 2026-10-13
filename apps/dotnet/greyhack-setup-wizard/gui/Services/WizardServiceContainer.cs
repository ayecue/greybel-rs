using GreyHackSetupWizard.Game;
using GreyHackSetupWizard.Setup;

namespace GreyHackSetupWizard.Gui.Services;

public sealed record WizardServiceContainer(
    IFilePicker Picker,
    DownloadSettings Download,
    Func<GameLayout?> LocateGame,
    string Repository
);
