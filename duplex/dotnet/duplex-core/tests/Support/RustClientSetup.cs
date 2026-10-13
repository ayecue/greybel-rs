using System.Collections.Concurrent;
using System.Diagnostics;
using System.Net;
using Duplex.Tests.Calculator;

namespace Duplex.Tests.Support;

internal static class RustClientSetup
{
    internal static readonly TimeSpan WaitLimit = TimeSpan.FromMinutes(5);

    internal sealed class Responder(
        Hub hub,
        BlockingCollection<string> logTexts,
        Func<CalculatorChannel> channel
    ) : CalculatorRouter
    {
        public override void OnLog(Log message)
        {
            logTexts.Add(message.Text);
            var calculator = channel();
            calculator.Send(new Progress(Phase.Active, null, [], new Dictionary<string, uint>()));

            hub.Post(() => calculator.Close());
        }
    }
}
