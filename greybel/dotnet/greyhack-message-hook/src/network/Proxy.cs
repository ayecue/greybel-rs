using System;
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.Net;
using System.Threading;
using System.Threading.Tasks;
using Duplex;
using GreyHackMessageHook.Game;
using GreyHackMessageHook.Protocol;

namespace GreyHackMessageHook
{
    public delegate bool MessageFilter(Message message);

    public delegate void MessageReceivedEventHandler(Message message);

    internal class PendingWait
    {
        public MessageFilter Filter { get; set; }
        public TaskCompletionSource<Message> TaskCompletionSource { get; set; }
        public CancellationToken CancellationToken { get; set; }
    }

    public static class Proxy
    {
        public static readonly Converter Converter = new Converter();

        static readonly object gate = new object();
        static MessageProxy proxy = null;
        static PlayerClient playerClient = null;

        static ConcurrentQueue<PendingWait> pendingWaits = new ConcurrentQueue<PendingWait>();

        public static event MessageReceivedEventHandler ReceiveMessage;

        static Proxy()
        {
            ReceiveMessage += ProcessPendingWaits;
        }

        public static void SetPlayerClient(PlayerClient client)
        {
            playerClient = client;
        }

        public static void ResetPlayerClient()
        {
            playerClient = null;
        }

        public static PlayerClient GetPlayerClient()
        {
            return playerClient;
        }

        public static void RegisterMessageListener(MessageReceivedEventHandler handler)
        {
            ReceiveMessage += handler;
        }

        public static void UnregisterMessageListener(MessageReceivedEventHandler handler)
        {
            ReceiveMessage -= handler;
        }

        public static Task<Message> WaitForReceivedMessage(
            MessageFilter filter,
            CancellationToken cancellationToken = default
        )
        {
            var tcs = new TaskCompletionSource<Message>();
            var pendingWait = new PendingWait
            {
                Filter = filter,
                TaskCompletionSource = tcs,
                CancellationToken = cancellationToken,
            };

            pendingWaits.Enqueue(pendingWait);

            if (cancellationToken != default)
            {
                cancellationToken.Register(() =>
                {
                    tcs.TrySetCanceled();
                });
            }

            return tcs.Task;
        }

        public static async Task<Message> WaitForReceivedMessage(
            MessageFilter filter,
            int timeoutMs
        )
        {
            using (var cts = new CancellationTokenSource(timeoutMs))
            {
                try
                {
                    return await WaitForReceivedMessage(filter, cts.Token);
                }
                catch (OperationCanceledException)
                {
                    return null;
                }
            }
        }

        public static void ProcessReceivedData(RawMessage raw)
        {
            Message message;
            try
            {
                message = Converter.Decode(raw, Role.Provider);
            }
            catch (Exception error)
            {
                Logger.Log("Proxy - game message " + raw.Id + " doesn't convert");
                Logger.LogError(error);
                return;
            }
            if (message == null)
            {
                return;
            }
            ReceiveMessage?.Invoke(message);
            Publish(message);
        }

        private static void ProcessPendingWaits(Message message)
        {
            List<PendingWait> remainingWaits = new List<PendingWait>();

            while (pendingWaits.TryDequeue(out var pendingWait))
            {
                if (pendingWait.CancellationToken.IsCancellationRequested)
                {
                    pendingWait.TaskCompletionSource.TrySetCanceled();
                    continue;
                }

                try
                {
                    if (pendingWait.Filter(message))
                    {
                        pendingWait.TaskCompletionSource.TrySetResult(message);
                    }
                    else
                    {
                        remainingWaits.Add(pendingWait);
                    }
                }
                catch (Exception ex)
                {
                    pendingWait.TaskCompletionSource.TrySetException(ex);
                }
            }

            foreach (var wait in remainingWaits)
            {
                pendingWaits.Enqueue(wait);
            }
        }

        /// <summary>
        /// Delivers <paramref name="message"/> as a game-originated message to the plugin listeners and the
        /// extension.
        /// </summary>
        internal static void Deliver(Message message)
        {
            ReceiveMessage?.Invoke(message);
            Publish(message);
        }

        internal static void Publish(Message message)
        {
            proxy?.Publish(message);
        }

        public static void Start()
        {
            lock (gate)
            {
                if (proxy != null)
                {
                    Logger.Log("Proxy - Proxy is already running...");
                    return;
                }
                proxy = new MessageProxy(new PlayerClientGame(), new ProxyLog());
                proxy.Start(new IPEndPoint(IPAddress.Loopback, Plugin.Port.Value));
            }
        }

        public static void Stop()
        {
            lock (gate)
            {
                proxy?.Dispose();
                proxy = null;
            }
        }

        private sealed class ProxyLog : IProxyLog
        {
            public void LogInfo(string message)
            {
                Logger.Log(message);
            }

            public void LogError(Exception error)
            {
                Logger.LogError(error);
            }
        }
    }
}
