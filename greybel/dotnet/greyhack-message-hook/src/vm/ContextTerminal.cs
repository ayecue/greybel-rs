using System.Collections;
using System.Collections.Generic;
using BepInEx;
using GreyHackMessageHook.Game;
using GreyHackMessageHook.Protocol;
using NetworkMessages;
using UnityEngine;
#if BEPINEX6
using BepInEx.Unity.Mono;
#endif

namespace GreyHackMessageHook
{
    /// <summary>
    /// Implements the game <c>Terminal</c> behavior for a context program without a window or drawing. It waits for
    /// transfers, deciphers, and aireplay captures while the terminal reports them
    /// (<c>Terminal.DownloadProgress</c>, <c>Decipher</c>, <c>CollectData</c>), sends the same server responses as the
    /// terminal, and writes terminal failure lines to program output.
    /// </summary>
    public class ContextTerminal
    {
        /// <summary>Bytes transferred per second for one disk-speed unit, using terminal accounting.</summary>
        private const int BytesPerSpeed = 125000;

        private readonly int pid;
        private string currentFolder;
        private string activeUser;
        private Coroutine download;
        private Coroutine decipher;
        private Aireplay aireplay;

        public ContextTerminal(int pid, string currentFolder, string activeUser)
        {
            this.pid = pid;
            this.currentFolder = currentFolder;
            this.activeUser = activeUser;
        }

        /// <summary>Reports a server-sized transfer through <c>Terminal.ResumeConnectionWindow</c>.</summary>
        public void ResumeDownload(long size, int multSpeedDisk, int processes)
        {
            if (download == null)
            {
                download = Start(
                    Download(size, multSpeedDisk * BytesPerSpeed / processes, failed: false)
                );
            }
        }

        /// <summary>Reports a disk-failed transfer through <c>Terminal.ResumeConnectionDiskFailed</c>.</summary>
        public void ResumeFailedDownload(long size, int multSpeedDisk, int processes)
        {
            if (download == null)
            {
                download = Start(
                    Download(size, multSpeedDisk * BytesPerSpeed / (processes + 1), failed: true)
                );
            }
        }

        /// <summary>Reports a decipher lasting <paramref name="seconds"/> through <c>Terminal.ResumeConnectionDecipher</c>.</summary>
        public void ResumeDecipher(float seconds)
        {
            if (decipher == null)
            {
                decipher = Start(RunDecipher(seconds, typeError: 0));
            }
        }

        /// <summary>Reports a hardware-failed process through <c>Terminal.ResumeConnectionProcessFailed</c>.</summary>
        public void ResumeFailedProcess(float seconds, int typeError)
        {
            decipher = Start(RunDecipher(seconds, typeError));
        }

        /// <summary>Reports a router packet capture through <c>Terminal.ResumeAireplay</c>.</summary>
        public void ResumeAireplay(
            List<ServerMap.HelperRoutersCercanos> routers,
            bool isLan,
            string origPublicIp,
            string bssid,
            string essid,
            int maxAcks
        )
        {
            Aireplay capture = new Aireplay(origPublicIp, bssid, essid);

            if (!isLan)
            {
                ServerMap.HelperRoutersCercanos router = routers.Find(item =>
                    item.bssid.Equals(bssid)
                );

                if (router == null)
                {
                    return;
                }

                capture.Power = 100 - (int)(router.distancia * 100f / 0.5f);
            }

            aireplay = capture;
            capture.Routine = Start(CollectData(capture, maxAcks));
        }

        /// <summary>Reports the program's new folder through <c>Terminal.ResumeConnection</c>.</summary>
        public void SetFolder(string folder)
        {
            currentFolder = folder;
        }

        /// <summary>Reports the new connection user and folder through <c>Terminal.UpdateTerminalRemote</c>.</summary>
        public void SetConnection(string user, string folder)
        {
            activeUser = user;
            currentFolder = folder;
        }

        /// <summary>Applies terminal Ctrl+C cancellation to pending program work; does nothing if no work is pending.</summary>
        public void Cancel()
        {
            if (download != null || decipher != null || aireplay != null)
            {
                CancelCommand();
            }
        }

        private IEnumerator Download(long size, int speed, bool failed)
        {
            int target = failed ? UnityEngine.Random.Range(0, 50) : 100;
            float percent = 0f;

            while (percent < target)
            {
                percent = size > 0 ? percent + speed * 100f / size * Time.deltaTime : 100f;

                yield return null;
            }

            if (failed)
            {
                Print("<color=red>" + TranslationManager.GetText("IO_ERROR") + "</color>");
                CancelCommand();
                yield break;
            }

            download = null;

            MessageServer end = new MessageServer(IdServer.EndDownloadServerRpc, pid, true);
            Send(end);
        }

        private IEnumerator RunDecipher(float seconds, int typeError)
        {
            yield return new WaitForSeconds(0.25f);
            yield return new WaitForEndOfFrame();

            int target = typeError == 0 ? 100 : UnityEngine.Random.Range(1, 20);
            float percent = 0f;

            while (percent < target)
            {
                percent += 100f / seconds * Time.deltaTime;

                yield return null;
            }

            decipher = null;

            if (target < 100)
            {
                // A forced restart reboots after its screen closes; a kernel panic displays only a window.
                if (typeError == 3)
                {
                    InternalBash.Singleton.RebootSystem(goToMainMenu: false, waitTime: false);
                }

                Print(
                    "<color=red>Process crashed: Fatal error 0x0000F0"
                        + UnityEngine.Random.Range(0, 10)
                        + "\nWarning: Unstable system.</color>"
                );
                CancelCommand();
                yield break;
            }

            MessageServer end = new MessageServer(IdServer.EndDecipherServerRpc, pid);
            Send(end);
        }

        private IEnumerator CollectData(Aireplay capture, int maxAcks)
        {
            yield return new WaitForSeconds(0.75f);

            while (capture.Packets < int.MaxValue)
            {
                capture.Acks += UnityEngine.Random.Range(0, capture.Power * 150 / 100 + 1);
                capture.Line =
                    "Read " + capture.Packets + " packets (got " + capture.Acks + " ACKs)";

                yield return new WaitForSeconds(0.5f);

                if (maxAcks != -1 && capture.Acks > maxAcks)
                {
                    aireplay = null;
                    FinishAireplay(capture, cancelCommand: false);
                    yield break;
                }

                capture.Packets += UnityEngine.Random.Range(25, 250);
            }
        }

        private void FinishAireplay(Aireplay capture, bool cancelCommand)
        {
            // The terminal replaces this line while reading; program output retains only its final position.
            if (capture.Line != null)
            {
                Print(capture.Line);
            }

            MessageServer fileCap = new MessageServer(
                IdServer.FileCapServerRpc,
                capture.OrigPublicIp,
                currentFolder,
                capture.Power,
                capture.Acks,
                capture.Bssid,
                capture.Essid,
                activeUser,
                pid,
                cancelCommand
            );
            Send(fileCap);
        }

        /// <summary>Implements <c>Terminal.CancelCommand</c>.</summary>
        private void CancelCommand()
        {
            MessageServer cancelDecipher = new MessageServer(IdServer.CancelDecipherServerRpc, pid);
            Send(cancelDecipher);

            if (download != null)
            {
                Stop(download);
                download = null;

                MessageServer cancelDownload = new MessageServer(
                    IdServer.CancelDownloadServerRpc,
                    pid
                );
                Send(cancelDownload);
            }
            else if (decipher != null)
            {
                Stop(decipher);
                decipher = null;
            }
            else if (aireplay != null)
            {
                Aireplay capture = aireplay;

                aireplay = null;
                FinishAireplay(capture, cancelCommand: true);
                Stop(capture.Routine);
            }
            else
            {
                MessageServer cancelScript = new MessageServer(IdServer.CancelScriptServerRpc, pid);
                Send(cancelScript);
            }
        }

        /// <summary>Writes <paramref name="line"/> to program output.</summary>
        private void Print(string line)
        {
            Proxy.Deliver(new PrintSentClientRpc(StringCompressor.Zip(line), false, pid));
        }

        private static void Send(MessageServer message)
        {
            Proxy.GetPlayerClient()?.SendData(message);
        }

        private static Coroutine Start(IEnumerator routine)
        {
            return ThreadingHelper.Instance.StartCoroutine(routine);
        }

        private static void Stop(Coroutine routine)
        {
            if (routine != null)
            {
                ThreadingHelper.Instance.StopCoroutine(routine);
            }
        }

        /// <summary>Reports an active capture through <c>Terminal.HelperCommandAireplay</c>.</summary>
        private sealed class Aireplay
        {
            public readonly string OrigPublicIp;
            public readonly string Bssid;
            public readonly string Essid;
            public int Power = 100;
            public int Packets;
            public int Acks;

            /// <summary>Terminal progress text captured after the first read.</summary>
            public string Line;
            public Coroutine Routine;

            public Aireplay(string origPublicIp, string bssid, string essid)
            {
                OrigPublicIp = origPublicIp;
                Bssid = bssid;
                Essid = essid;
            }
        }
    }
}
