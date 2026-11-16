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
    private const int NoTerminal = -1;

    private readonly Dictionary<string, Execution> executions = new Dictionary<string, Execution>();
    private readonly Dictionary<string, PairExecution> pairs =
        new Dictionary<string, PairExecution>();

    /// <summary>
    /// Runs the script as typed into the terminal of the player's computer: the safe-mode terminal while the computer
    /// runs one (the desktop isn't running then, so it is the only terminal), otherwise a terminal that isn't
    /// protected (which needs no process to tell). The other execution requests do the same.
    /// </summary>
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

    /// <summary>
    /// Runs the script as typed into the safe-mode terminal that a restart in safe mode starts, or into a terminal
    /// process of the player's computer that isn't protected (<c>Computer.AddProcess</c>, as
    /// <c>ShellIntrinsics.StartProgram</c> adds the process of a terminal). The terminal process ends with the script.
    /// </summary>
    public void ExecuteInTerminal(Requester requester, ExecuteInTerminalRpc request)
    {
        main.Post(
            requester,
            () =>
            {
                Computer host = player.Computer;
                int terminal = request.SafeTerminal
                    ? FindSafeTerminal(host)
                    : host.AddProcess(
                        "Terminal",
                        player.User,
                        Computer.GetRamUsage("Terminal"),
                        isScript: false,
                        isTerminal: true,
                        host.GetID(),
                        -1,
                        isProtected: false
                    );

                if (terminal == NoTerminal)
                {
                    throw new InvalidOperationException(
                        "The player's computer runs no safe-mode terminal"
                    );
                }

                Execution execution = new Execution
                {
                    Requester = requester,
                    OwnedTerminal = request.SafeTerminal ? NoTerminal : terminal,
                };

                Start(
                    CreateId(),
                    execution,
                    request.Basepath,
                    request.Filepath,
                    request.ProgramName,
                    request.Source,
                    host,
                    terminal
                );
            }
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
        Computer host,
        int? terminal = null
    )
    {
        executions.Add(id, execution);
        int terminalPid = terminal ?? FindSafeTerminal(host);
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
                safeTerminal: IsProtectedTerminal(host, terminalPid)
            );
        }
        catch
        {
            executions.Remove(id);
            CloseOwnedTerminal(execution);
            throw;
        }
        ContextManager.ContextProcess process = ContextManager.GetContext(id);
        if (process == null)
        {
            // A very short program can finish on the script thread before CreateContext returns. Its completion message
            // is still handled below and resolves the execution; there is no live PID left to record.
            return;
        }

        execution.Pid = process.Pid;
    }

    /// <summary>
    /// Finds the protected terminal that a restart in safe mode starts on <paramref name="host"/>
    /// (<c>PlayerHelperServer.CheckIniErrorServerRpc</c>), or <see cref="NoTerminal"/>.
    /// </summary>
    private static int FindSafeTerminal(Computer host)
    {
        Computer.Proceso terminal = host.GetProcesos()
            .FirstOrDefault(process =>
                process.isProtected
                && process.isTerminal
                && process.nombreProceso == "Terminal-safemode"
            );

        return terminal?.PID ?? NoTerminal;
    }

    /// <summary>
    /// Whether the program typed into terminal <paramref name="terminalPid"/> of <paramref name="host"/> runs in a safe
    /// terminal: the terminal process is protected (<c>GreyScriptHelperServer.PrepareTerminalServerRpc</c> passes
    /// <c>Proceso.isProtected</c> as <c>GreyInterpreter.isSafeTerminal</c>).
    /// </summary>
    private static bool IsProtectedTerminal(Computer host, int terminalPid)
    {
        return terminalPid != NoTerminal && host.GetProceso(terminalPid)?.isProtected == true;
    }

    /// <summary>Ends the terminal process that the execution's request added, which has no program left to type into.</summary>
    private void CloseOwnedTerminal(Execution execution)
    {
        if (execution.OwnedTerminal == NoTerminal)
        {
            return;
        }

        player.Computer.ClosePrograms(new List<int> { execution.OwnedTerminal });
        execution.OwnedTerminal = NoTerminal;
    }

    private void Finish(Execution execution, ExecutionResultReply result)
    {
        CloseOwnedTerminal(execution);

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
