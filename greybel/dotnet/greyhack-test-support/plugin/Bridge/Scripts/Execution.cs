using System.Collections.Generic;
using GreyHackTestSupport.Plugin.Bridge.Connection;
using GreyHackTestSupport.Protocol;
using GameProtocol = GreyHackMessageHook.Protocol;

namespace GreyHackTestSupport.Plugin.Bridge.Scripts;

/// <summary>Tracks a bridge script running in a message-hook context until game completion.</summary>
internal sealed class Execution
{
    /// <summary>
    /// Destination for the next reply: initially the starting request, then the request that supplied awaited input.
    /// </summary>
    public Requester Requester;

    /// <summary>Context program assigned after <c>ContextManager.CreateContext</c> returns.</summary>
    public int Pid;

    public readonly Output Output = new Output();

    /// <summary>Owning script pair and whether this script is its first member.</summary>
    public string PairId;
    public bool First;

    /// <summary>Ordered inputs supplied whenever the script requests input.</summary>
    public Queue<string> Inputs;

    /// <summary>True when input requests wait for test input instead of consuming queued input.</summary>
    public bool Pauses;

    /// <summary>Pending input request while the context is waiting.</summary>
    public GameProtocol.InputSentClientRpc InputRequest;
}

/// <summary>Combines two script results into one reply after both scripts finish.</summary>
internal sealed class PairExecution
{
    public Requester Requester;
    public ExecutionResultReply First;
    public ExecutionResultReply Second;
}
