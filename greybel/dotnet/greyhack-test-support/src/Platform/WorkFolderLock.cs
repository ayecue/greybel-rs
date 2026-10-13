namespace GreyHackTestSupport.Platform;

/// <summary>Exclusive folder <c>.lock</c> file lock that is released on disposal.</summary>
internal sealed class WorkFolderLock : IDisposable
{
    private static readonly TimeSpan RetryInterval = TimeSpan.FromMilliseconds(500);

    private readonly FileStream file;

    private WorkFolderLock(FileStream file)
    {
        this.file = file;
    }

    /// <summary>Creates <paramref name="folder"/> if needed and waits for exclusive access to its lock.</summary>
    public static WorkFolderLock Take(string folder)
    {
        Directory.CreateDirectory(folder);
        string path = Path.Combine(folder, ".lock");
        while (true)
        {
            try
            {
                return new WorkFolderLock(
                    new FileStream(
                        path,
                        FileMode.OpenOrCreate,
                        FileAccess.ReadWrite,
                        FileShare.None
                    )
                );
            }
            catch (IOException)
            {
                Thread.Sleep(RetryInterval);
            }
        }
    }

    public void Dispose() => file.Dispose();
}
