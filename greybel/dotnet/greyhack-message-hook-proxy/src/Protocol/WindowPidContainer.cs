#nullable enable
using System;
using GreyHackMessageHook.Game;
using Newtonsoft.Json;

namespace GreyHackMessageHook.Protocol;

public readonly struct WindowPidContainer
{
    private readonly int? singlePid;
    private readonly int[]? pidList;
    private readonly byte[]? payload;

    private WindowPidContainer(int? singlePid, int[]? pidList, byte[]? payload)
    {
        this.singlePid = singlePid;
        this.pidList = pidList;
        this.payload = payload;
    }

    public static WindowPidContainer None => default;

    public static WindowPidContainer FromPid(int pid) => new WindowPidContainer(pid, null, null);

    public static WindowPidContainer FromPids(int[]? pids) =>
        new WindowPidContainer(null, pids ?? new int[0], null);

    public static WindowPidContainer FromPayload(byte[]? payload) =>
        new WindowPidContainer(null, null, payload ?? new byte[0]);

    public int[] ToArray()
    {
        if (singlePid.HasValue)
        {
            return new[] { singlePid.Value };
        }
        if (pidList != null)
        {
            return pidList;
        }
        if (payload != null && payload.Length > 0)
        {
            try
            {
                return JsonConvert.DeserializeObject<int[]>(StringCompressor.Unzip(payload))
                    ?? new int[0];
            }
            catch (Exception)
            {
                return new int[0];
            }
        }
        return new int[0];
    }

    public bool Contains(int pid)
    {
        return singlePid.HasValue ? singlePid.Value == pid : Array.IndexOf(ToArray(), pid) >= 0;
    }
}
