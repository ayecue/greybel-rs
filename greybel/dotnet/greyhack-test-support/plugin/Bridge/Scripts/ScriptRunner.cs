using System;
using System.Collections.Generic;
using System.Linq;
using System.Threading.Tasks;
using GreyHackMessageHook;
using GreyHackMessageHook.Greybel;
using GreyHackTestSupport.Plugin.Bridge.Connection;
using GreyHackTestSupport.Plugin.Bridge.Game;
using GreyHackTestSupport.Protocol;
using GameProtocol = GreyHackMessageHook.Protocol;

namespace GreyHackTestSupport.Plugin.Bridge.Scripts;

/// <summary>
/// Runs test scripts as the player user in message-hook contexts on the player's computer. After game completion,
/// it replies with printed output. It supports standalone scripts, queued input, test-driven input pauses, and pairs
/// whose results are returned together.
/// </summary>
internal sealed class ScriptRunner(MainThread main, Player player)
{
    private readonly Dictionary<string, Execution> executions = new Dictionary<string, Execution>();
    private readonly Dictionary<string, PairExecution> pairs =
        new Dictionary<string, PairExecution>();

    public void Execute(Requester requester, ExecuteRpc request)
    {
        main.Post(
            requester,
            () =>
                Start(
                    CreateId(),
                    new Execution { Requester = requester },
                    request.Basepath,
                    request.Filepath,
                    request.ProgramName,
                    request.Source,
                    player.Computer
                )
        );
    }

    /// <summary>Starts a context that consumes queued input at each input request.</summary>
    public void ExecuteInteractive(Requester requester, ExecuteInteractiveRpc request)
    {
        main.Post(
            requester,
            () =>
                Start(
                    CreateId(),
                    new Execution
                    {
                        Requester = requester,
                        Inputs = new Queue<string>(request.Inputs),
                    },
                    request.Basepath,
                    request.Filepath,
                    request.ProgramName,
                    request.Source,
                    player.Computer
                )
        );
    }

    /// <summary>
    /// Starts a context that sends <see cref="Reply.InputRequested"/> at each input request and waits for
    /// <see cref="SendInput"/> to supply the input.
    /// </summary>
    public void ExecuteUntilInput(Requester requester, ExecuteUntilInputRpc request)
    {
        main.Post(
            requester,
            () =>
                Start(
                    CreateId(),
                    new Execution { Requester = requester, Pauses = true },
                    request.Basepath,
                    request.Filepath,
                    request.ProgramName,
                    request.Source,
                    player.Computer
                )
        );
    }

    /// <summary>Starts two independent contexts and correlates their replies even when completion order differs.</summary>
    public void ExecutePair(Requester requester, ExecutePairRpc request)
    {
        main.Post(
            requester,
            () =>
            {
                string pairId = CreateId();
                pairs.Add(pairId, new PairExecution { Requester = requester });
                Computer host = player.Computer;
                foreach (
                    (bool first, string source) in new[]
                    {
                        (true, request.FirstSource),
                        (false, request.SecondSource),
                    }
                )
                {
                    Start(
                        pairId + (first ? "a" : "b"),
                        new Execution { PairId = pairId, First = first },
                        request.Basepath,
                        request.Filepath,
                        request.ProgramName,
                        source,
                        host
                    );
                }
            }
        );
    }

    /// <summary>
    /// Supplies <paramref name="input"/> to the waiting context. Its next input request or completion reply is sent to
    /// <paramref name="requester"/>.
    /// </summary>
    public void SendInput(Requester requester, string input)
    {
        main.Post(
            requester,
            () =>
            {
                Execution execution =
                    executions.Values.FirstOrDefault(entry => entry.InputRequest != null)
                    ?? throw new InvalidOperationException("No script waits for input");

                GameProtocol.InputSentClientRpc request = execution.InputRequest;
                execution.InputRequest = null;
                execution.Requester = requester;
                player.Server.greyScriptHelper.InputUserServerRpc(
                    CompressString.StringCompressor.Zip(input),
                    request.WindowPid
                );
                if (request.AnyKey)
                {
                    execution.Output.Print(input, false);
                }
            }
        );
    }

    /// <summary>
    /// Processes game messages for bridge contexts on the main thread after their starting request returns. Messages
    /// may originate on the script thread, and because contexts start inside <c>ContextManager.CreateContext</c>, output
    /// and completion can arrive before the PID is available.
    /// </summary>
    public void Handle(Duplex.Message message)
    {
        switch (message)
        {
            case GameProtocol.InputSentClientRpc input
                when FindExecution(input.WindowPid) is Execution { Pauses: true } execution:
                execution.InputRequest = input;
                execution.Requester.Reply(new Reply.InputRequested());
                break;
            case GameProtocol.InputSentClientRpc input:
                string inputText = SendQueuedInput(input.WindowPid);
                if (inputText != null && input.AnyKey)
                {
                    FindExecution(input.WindowPid)?.Output.Print(inputText, false);
                }
                break;
            case GameProtocol.SetPollInputModeClientRpc poll when poll.Enabled:
                int pollPid = poll.WindowPid;
                Task.Run(async () =>
                {
                    await Task.Delay(100);
                    main.Post(() => SendQueuedInput(pollPid));
                });
                break;
            case GameProtocol.PrintSentClientRpc print:
                FindExecution(print.WindowPid)
                    ?.Output.Print(
                        CompressString.StringCompressor.Unzip(print.ZipOutput),
                        print.ReplaceText
                    );
                break;
            case GameProtocol.ClearScreenClientRpc clear:
                FindExecution(clear.TerminalPid)?.Output.Clear();
                break;
            case GameProtocol.FinishedContextRpc contextEnd
                when executions.Remove(contextEnd.ContextId, out Execution execution):
                Finish(
                    execution,
                    new ExecutionResultReply(execution.Output.ToString(), contextEnd.Failed)
                );
                break;
        }
    }

    private void Start(
        string id,
        Execution execution,
        string basepath,
        string filepath,
        string programName,
        string source,
        Computer host
    )
    {
        executions.Add(id, execution);
        try
        {
            ContextManager.CreateContext(
                id,
                false,
                player.Server,
                basepath,
                filepath,
                programName,
                source,
                null,
                true,
                new List<string>(),
                host,
                new List<RuntimeMetadata.Breakpoint>(),
                player.User,
                printsCallStack: false,
                safeTerminal: IsInSafeMode(host)
            );
        }
        catch
        {
            executions.Remove(id);
            throw;
        }
        ContextManager.ContextProcess process = ContextManager.GetContext(id);
        execution.Pid = process.Pid;
    }

    /// <summary>
    /// Whether <paramref name="host"/> runs the protected safe-mode terminal that a restart in safe mode starts
    /// (<c>PlayerHelperServer.CheckIniErrorServerRpc</c>). The programs run on the player's computer then are the
    /// ones typed in that terminal (<c>GreyScriptHelperServer.PrepareTerminalServerRpc</c>:
    /// <c>GreyInterpreter.isSafeTerminal</c>).
    /// </summary>
    private static bool IsInSafeMode(Computer host)
    {
        return host.GetProcesos()
            .Any(process =>
                process.isProtected
                && process.isTerminal
                && process.nombreProceso == "Terminal-safemode"
            );
    }

    private void Finish(Execution execution, ExecutionResultReply result)
    {
        if (execution.PairId == null)
        {
            execution.Requester.Reply(new Reply.ExecutionResult(result));
            return;
        }

        PairExecution pair = pairs[execution.PairId];
        if (execution.First)
        {
            pair.First = result;
        }
        else
        {
            pair.Second = result;
        }

        if (pair.First != null && pair.Second != null)
        {
            pairs.Remove(execution.PairId);
            pair.Requester.Reply(
                new Reply.PairExecutionResult(
                    new PairExecutionResultReply(
                        pair.First.Output,
                        pair.First.Failed,
                        pair.Second.Output,
                        pair.Second.Failed
                    )
                )
            );
        }
    }

    /// <summary>Consumes the next queued input for program <paramref name="pid"/>, or returns <c>null</c> if none remains.</summary>
    private string SendQueuedInput(int pid)
    {
        Execution execution = FindExecution(pid);
        if (execution?.Inputs == null || execution.Inputs.Count == 0)
        {
            return null;
        }
        string input = execution.Inputs.Dequeue();
        player.Server.greyScriptHelper.InputUserServerRpc(
            CompressString.StringCompressor.Zip(input),
            pid
        );
        return input;
    }

    /// <summary>Gets the execution for program <paramref name="pid"/>, or <c>null</c> if it is not tracked.</summary>
    private Execution FindExecution(int pid)
    {
        return executions.Values.FirstOrDefault(item => item.Pid == pid);
    }

    private static string CreateId() => Guid.NewGuid().ToString("N");
}
