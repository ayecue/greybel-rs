#nullable enable
using System;
using System.Collections.Generic;
using System.Linq;

namespace Duplex;

internal sealed class OutboundAliasTable
{
    private const uint FirstAlias = 1;
    private const uint MaxAliases = 8191;

    private readonly Dictionary<ulong, Dictionary<uint, uint>> aliasesByPeer =
        new Dictionary<ulong, Dictionary<uint, uint>>();
    private readonly List<uint> vacantAliases = new List<uint>();
    private readonly List<SliceRoute> releasedRoutes = new List<SliceRoute>();
    private uint nextAlias = FirstAlias;

    public SliceKey GetKey(SliceRoute route)
    {
        if (route.Connection == 0)
        {
            return SliceKey.FromRoute(route);
        }
        aliasesByPeer.TryGetValue(route.PeerKey, out var connections);
        if (connections != null && connections.TryGetValue(route.Connection, out var current))
        {
            return SliceKey.FromAlias(current);
        }
        var alias = Allocate();
        if (alias == null)
        {
            return SliceKey.FromRoute(route);
        }
        if (connections == null)
        {
            connections = new Dictionary<uint, uint>();
            aliasesByPeer[route.PeerKey] = connections;
        }
        connections[route.Connection] = alias.Value;
        return SliceKey.CreateAnnouncement(alias.Value, route);
    }

    public void Release(SliceRoute route) => releasedRoutes.Add(route);

    public void Reclaim()
    {
        foreach (var route in releasedRoutes)
        {
            if (
                route.Connection == 0
                || !aliasesByPeer.TryGetValue(route.PeerKey, out var connections)
            )
            {
                continue;
            }
            if (connections.TryGetValue(route.Connection, out var alias))
            {
                connections.Remove(route.Connection);
                vacantAliases.Add(alias);
            }
        }
        releasedRoutes.Clear();
    }

    public void Clear()
    {
        aliasesByPeer.Clear();
        vacantAliases.Clear();
        releasedRoutes.Clear();
        nextAlias = FirstAlias;
    }

    private uint? Allocate()
    {
        if (vacantAliases.Count > 0)
        {
            var smallest = vacantAliases.Min();
            vacantAliases.Remove(smallest);
            return smallest;
        }
        if (nextAlias > MaxAliases)
        {
            return null;
        }
        return nextAlias++;
    }
}

internal sealed class InboundAliasTable
{
    private const uint MaxAliases = 8191;

    private readonly Dictionary<uint, SliceRoute> routes = new Dictionary<uint, SliceRoute>();

    public SliceRoute? GetRoute(SliceKey key)
    {
        switch (key.Kind)
        {
            case SliceKeyKind.NoAlias:
                return key.Route;
            case SliceKeyKind.Announcement:
                if (key.Alias <= MaxAliases)
                {
                    routes[key.Alias] = key.Route;
                }
                return key.Route;
            default:
                return routes.TryGetValue(key.Alias, out var route) ? route : null;
        }
    }

    public void Clear() => routes.Clear();
}

internal sealed class Egress
{
    public const int FlushBytes = 64 * 1024;
    private const int MaxBufferedBytes = 16 * 1024 * 1024;
    private const int MinFragmentBytes = 16 * 1024;

    private readonly Dictionary<ulong, Outbox> outboxes = new Dictionary<ulong, Outbox>();
    private readonly List<Outbox> readyOutboxes = new List<Outbox>();
    private readonly DuplexWriter batchWriter = new DuplexWriter();
    private readonly Planner planner = new Planner();
    private int pendingBytes;
    private Outbox? currentOutbox;
    private SliceRoute currentRoute;
    private int frameStart;
    private long maxFrameLength = long.MaxValue;

    public OutboundAliasTable Aliases { get; } = new OutboundAliasTable();

    public bool IsFull => pendingBytes >= FlushBytes;

    public bool IsEmpty => pendingBytes == 0;

    public FrameEncoder Begin(SliceRoute route, ushort minor, long maxFrame)
    {
        if (pendingBytes >= MaxBufferedBytes)
        {
            throw ChannelException.CreateRejection(ErrorCode.Overload);
        }
        if (!outboxes.TryGetValue(route.PeerKey, out var outbox))
        {
            outbox = new Outbox();
            outboxes[route.PeerKey] = outbox;
        }
        currentOutbox = outbox;
        currentRoute = route;
        frameStart = outbox.Frames.Size;
        maxFrameLength = maxFrame;
        outbox.Encoder.Minor = minor;
        outbox.Encoder.LastFrameStart = null;
        outbox.Encoder.Validate(false, long.MaxValue);
        return outbox.Encoder;
    }

    public bool CommitFrame()
    {
        var outbox = currentOutbox!;
        var frames = outbox.Frames;
        var start = frameStart;
        var frameLength = frames.Size - start;
        if (frameLength > maxFrameLength)
        {
            frames.Truncate(start);
            throw ChannelException.CreateRejection(ErrorCode.Overload);
        }
        var last = outbox.Encoder.LastFrameStart;
        if (last == null || last < start || frameLength <= 0)
        {
            frames.Truncate(start);
            return false;
        }
        if (!outbox.IsReady)
        {
            outbox.IsReady = true;
            readyOutboxes.Add(outbox);
        }
        outbox.Append(currentRoute, frames.Size, last.Value);
        pendingBytes += frameLength;
        return true;
    }

    public void Flush(Action<byte[]> send)
    {
        if (readyOutboxes.Count == 0)
        {
            return;
        }
        if (readyOutboxes.Count != 1 || !SendOutboxInPlace(readyOutboxes[0], send))
        {
            SendBatch(readyOutboxes, send);
        }
        readyOutboxes.RemoveAll(outbox =>
        {
            outbox.IsReady = !outbox.IsEmpty;
            return !outbox.IsReady;
        });
    }

    public void Clear()
    {
        outboxes.Clear();
        readyOutboxes.Clear();
        pendingBytes = 0;
        Aliases.Clear();
    }

    private void SendBatch(List<Outbox> sharers, Action<byte[]> send)
    {
        planner.StartBatch(FlushBytes / sharers.Count);
        foreach (var outbox in sharers)
        {
            outbox.Plan(planner);
        }
        var slices = planner.Slices;
        var batch = new BatchWriter(batchWriter, BatchWriter.CoreMajor, slices.Count);
        foreach (var slice in slices)
        {
            var key = Aliases.GetKey(slice.Route);
            var bytes = slice.Outbox.Frames.Buffer;
            if (slice.Piece == FragmentPiece.None)
            {
                batch.WriteSlice(key, bytes, slice.Start, slice.End, slice.LastFrameStart);
            }
            else
            {
                batch.WriteFragment(
                    key,
                    bytes,
                    slice.Start,
                    slice.End,
                    slice.Piece == FragmentPiece.Ends
                );
            }
        }
        foreach (var slice in slices)
        {
            pendingBytes -= slice.End - slice.Start;
            if (slice.Piece == FragmentPiece.None)
            {
                slice.Outbox.AdvanceSendPosition(slice.End);
            }
            else
            {
                slice.Outbox.RecordSentPiece(slice.End, slice.Piece == FragmentPiece.Ends);
            }
        }
        slices.Clear();
        Aliases.Reclaim();
        send(batchWriter.ToArray());
    }

    private bool SendOutboxInPlace(Outbox outbox, Action<byte[]> send)
    {
        var run = outbox.FrontRun!;
        var start = outbox.UnsentStart;
        if (
            outbox.Runs.Count - outbox.FirstRunIndex != 1
            || run.End - start > FlushBytes
            || outbox.NextPieceStart != null
        )
        {
            return false;
        }
        new BatchWriter(batchWriter, BatchWriter.CoreMajor, 1).WriteSlice(
            Aliases.GetKey(run.Route),
            outbox.Frames.Buffer,
            start,
            run.End,
            run.LastFrameStart
        );
        pendingBytes -= run.End - start;
        outbox.AdvanceSendPosition(run.End);
        Aliases.Reclaim();
        send(batchWriter.ToArray());
        return true;
    }

    private sealed class Run
    {
        public Run(SliceRoute route, int end, int lastFrameStart)
        {
            Route = route;
            End = end;
            LastFrameStart = lastFrameStart;
        }

        public SliceRoute Route { get; }

        public int End { get; set; }

        public int LastFrameStart { get; set; }
    }

    private enum FragmentPiece
    {
        None,
        Continues,
        Ends,
    }

    private readonly struct PlannedSlice
    {
        public PlannedSlice(
            Outbox outbox,
            SliceRoute route,
            int start,
            int end,
            int lastFrameStart,
            FragmentPiece piece
        )
        {
            Outbox = outbox;
            Route = route;
            Start = start;
            End = end;
            LastFrameStart = lastFrameStart;
            Piece = piece;
        }

        public Outbox Outbox { get; }

        public SliceRoute Route { get; }

        public int Start { get; }

        public int End { get; }

        public int LastFrameStart { get; }

        public FragmentPiece Piece { get; }
    }

    private sealed class Outbox
    {
        public Outbox()
        {
            Encoder = new FrameEncoder(Frames);
        }

        public DuplexWriter Frames { get; } = new DuplexWriter();

        public FrameEncoder Encoder { get; }

        public List<Run> Runs { get; } = new List<Run>();

        public int FirstRunIndex { get; private set; }

        public int UnsentStart { get; private set; }

        public int? NextPieceStart { get; private set; }

        public bool IsReady { get; set; }

        public bool IsEmpty => FirstRunIndex == Runs.Count;

        public Run? FrontRun => FirstRunIndex < Runs.Count ? Runs[FirstRunIndex] : null;

        public void Append(SliceRoute route, int end, int last)
        {
            var run = Runs.Count > FirstRunIndex ? Runs[Runs.Count - 1] : null;
            if (run != null && run.Route.Equals(route))
            {
                run.End = end;
                run.LastFrameStart = last;
                return;
            }
            Runs.Add(new Run(route, end, last));
        }

        public void Plan(Planner planner)
        {
            var front = FrontRun;
            if (front == null)
            {
                return;
            }
            planner.StartOutbox(this);
            if (NextPieceStart != null)
            {
                planner.PlanNextPiece(front, UnsentStart, NextPieceStart.Value);
                return;
            }
            var start = UnsentStart;
            for (var index = FirstRunIndex; index < Runs.Count; index++)
            {
                var run = Runs[index];
                if (!planner.PlanRun(run, start))
                {
                    return;
                }
                start = run.End;
            }
        }

        public void RecordSentPiece(int end, bool isLastPiece)
        {
            if (!isLastPiece)
            {
                NextPieceStart = end;
                return;
            }
            NextPieceStart = null;
            AdvanceSendPosition(end);
        }

        public void AdvanceSendPosition(int end)
        {
            UnsentStart = end;
            while (FirstRunIndex < Runs.Count && Runs[FirstRunIndex].End <= end)
            {
                FirstRunIndex++;
            }
            if (FirstRunIndex == Runs.Count)
            {
                Runs.Clear();
                FirstRunIndex = 0;
                UnsentStart = 0;
                Frames.Reset();
                return;
            }
            if (UnsentStart < FlushBytes || UnsentStart * 2 < Frames.Size)
            {
                return;
            }
            var shift = UnsentStart;
            var size = Frames.Size;
            System.Buffer.BlockCopy(Frames.Buffer, shift, Frames.Buffer, 0, size - shift);
            Frames.Truncate(size - shift);
            Runs.RemoveRange(0, FirstRunIndex);
            FirstRunIndex = 0;
            foreach (var run in Runs)
            {
                run.End -= shift;
                run.LastFrameStart -= shift;
            }
            if (NextPieceStart != null)
            {
                NextPieceStart -= shift;
            }
            UnsentStart = 0;
        }
    }

    private sealed class Planner
    {
        private int sharePerOutbox;
        private int pieceSize;
        private Outbox? outbox;
        private byte[] bytes = new byte[0];
        private int size;
        private int shareLeft;
        private bool hasFrames;

        public List<PlannedSlice> Slices { get; } = new List<PlannedSlice>();

        public void StartBatch(int share)
        {
            Slices.Clear();
            sharePerOutbox = share;
            pieceSize = Math.Max(share, MinFragmentBytes);
        }

        public void StartOutbox(Outbox outbox)
        {
            this.outbox = outbox;
            bytes = outbox.Frames.Buffer;
            size = outbox.Frames.Size;
            shareLeft = sharePerOutbox;
            hasFrames = false;
        }

        public bool PlanRun(Run run, int start)
        {
            if (run.End - start <= shareLeft)
            {
                shareLeft -= run.End - start;
                AddFrames(run, start, run.End, run.LastFrameStart);
                return shareLeft > 0;
            }
            var end = start;
            var lastFrameStart = -1;
            while (end < run.End)
            {
                var frameEnd = Varuint.FindFrameEnd(bytes, end, size);
                if (frameEnd < 0 || frameEnd > run.End)
                {
                    return false;
                }
                var frameSize = frameEnd - end;
                if (frameSize <= shareLeft)
                {
                    shareLeft -= frameSize;
                    lastFrameStart = end;
                    end = frameEnd;
                    continue;
                }
                var isFirst = !hasFrames && lastFrameStart < 0;
                if (frameSize > pieceSize && (isFirst || shareLeft >= MinFragmentBytes))
                {
                    if (lastFrameStart >= 0)
                    {
                        AddFrames(run, start, end, lastFrameStart);
                    }
                    AddPiece(run, end, end + (isFirst ? pieceSize : shareLeft), false);
                    return false;
                }
                if (isFirst && frameSize <= pieceSize)
                {
                    shareLeft = 0;
                    lastFrameStart = end;
                    end = frameEnd;
                }
                break;
            }
            if (lastFrameStart >= 0)
            {
                AddFrames(run, start, end, lastFrameStart);
            }
            return end == run.End && shareLeft > 0;
        }

        public void PlanNextPiece(Run run, int frameStart, int pieceStart)
        {
            var frameEnd = Varuint.FindFrameEnd(bytes, frameStart, size);
            if (frameEnd < 0 || frameEnd > run.End)
            {
                return;
            }
            var pieceEnd = Math.Min(pieceStart + pieceSize, frameEnd);
            AddPiece(run, pieceStart, pieceEnd, pieceEnd == frameEnd);
        }

        private void AddFrames(Run run, int start, int end, int last)
        {
            Slices.Add(new PlannedSlice(outbox!, run.Route, start, end, last, FragmentPiece.None));
            hasFrames = true;
        }

        private void AddPiece(Run run, int start, int end, bool ends)
        {
            Slices.Add(
                new PlannedSlice(
                    outbox!,
                    run.Route,
                    start,
                    end,
                    start,
                    ends ? FragmentPiece.Ends : FragmentPiece.Continues
                )
            );
        }
    }
}
