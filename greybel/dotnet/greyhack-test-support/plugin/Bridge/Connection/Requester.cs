using GreyHackTestSupport.Protocol;

namespace GreyHackTestSupport.Plugin.Bridge.Connection;

/// <summary>Identifies a test request by connection and request ID for reply routing.</summary>
internal sealed class Requester(TestConnection connection, uint request)
{
    public void Reply(Reply reply)
    {
        connection.Send(new ReplyRpc(request, reply));
    }

    /// <summary>Returns the error and stack trace so test output includes fixture failure diagnostics.</summary>
    public void Fail(System.Exception error)
    {
        Reply(new Reply.TestFailure(new TestFailureReply(error.ToString())));
    }
}
