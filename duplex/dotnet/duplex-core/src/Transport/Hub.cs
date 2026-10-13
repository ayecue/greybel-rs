#nullable enable
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Threading;

namespace Duplex;

public sealed class Hub
{
    private readonly object gate = new object();
    private readonly Queue<Action> tasks = new Queue<Action>();
    private readonly List<Timer> timers = new List<Timer>();
    private readonly List<Transport> toFlush = new List<Transport>();
    private readonly Func<long> clock;
    private Thread? thread;
    private bool stopping;

    public Hub(Func<long>? clock = null)
    {
        if (clock == null)
        {
            var stopwatch = Stopwatch.StartNew();
            clock = () => stopwatch.ElapsedMilliseconds;
        }
        this.clock = clock;
    }

    public long Now => clock();

    public bool IsCurrent => thread == null || Thread.CurrentThread == thread;

    public void Post(Action task)
    {
        lock (gate)
        {
            tasks.Enqueue(task);
            Monitor.Pulse(gate);
        }
    }

    public IDisposable Schedule(long delay, Action task) => AddTimer(delay, null, task);

    public IDisposable ScheduleInterval(long period, Action task) => AddTimer(period, period, task);

    private Timer AddTimer(long delay, long? period, Action task)
    {
        var timer = new Timer(this, Now + Math.Max(delay, 0), period, task);
        lock (gate)
        {
            timers.Add(timer);
            Monitor.Pulse(gate);
        }
        return timer;
    }

    internal void OnWrite(Transport transport)
    {
        if (!toFlush.Contains(transport))
        {
            toFlush.Add(transport);
        }
    }

    public bool RunReady()
    {
        var ran = false;
        for (; ; )
        {
            Action? task = null;
            lock (gate)
            {
                if (tasks.Count > 0)
                {
                    task = tasks.Dequeue();
                }
                else
                {
                    var now = Now;
                    var due = timers.Find(timer => timer.Due <= now);
                    if (due != null)
                    {
                        task = due.Fire();
                        if (due.Period == null)
                        {
                            timers.Remove(due);
                        }
                    }
                }
            }
            if (task == null)
            {
                if (toFlush.Count == 0)
                {
                    return ran;
                }
                Flush();
                ran = true;
                continue;
            }
            task();
            Flush();
            ran = true;
        }
    }

    private void Flush()
    {
        var transports = toFlush.ToArray();
        toFlush.Clear();
        foreach (var transport in transports)
        {
            transport.Flush();
            if (!transport.IsIdle)
            {
                OnWrite(transport);
            }
        }
    }

    public Thread Start(string name = "duplex hub")
    {
        var runner = new Thread(Run) { IsBackground = true, Name = name };
        thread = runner;
        runner.Start();
        return runner;
    }

    public void Stop()
    {
        lock (gate)
        {
            stopping = true;
            Monitor.Pulse(gate);
        }
    }

    private void Run()
    {
        for (; ; )
        {
            RunReady();
            lock (gate)
            {
                if (stopping)
                {
                    return;
                }
                if (tasks.Count > 0 || toFlush.Count > 0)
                {
                    continue;
                }
                var wait = Timeout.Infinite;
                foreach (var timer in timers)
                {
                    var left = (int)Math.Min(Math.Max(timer.Due - Now, 0), int.MaxValue);
                    wait = wait == Timeout.Infinite ? left : Math.Min(wait, left);
                }
                if (wait != 0)
                {
                    Monitor.Wait(gate, wait);
                }
            }
        }
    }

    private sealed class Timer : IDisposable
    {
        private readonly Hub hub;
        private readonly Action task;

        public Timer(Hub hub, long due, long? period, Action task)
        {
            this.hub = hub;
            Due = due;
            Period = period;
            this.task = task;
        }

        public long Due { get; private set; }

        public long? Period { get; }

        public Action Fire()
        {
            if (Period != null)
            {
                Due += Math.Max(Period.Value, 1);
            }
            return task;
        }

        public void Dispose()
        {
            lock (hub.gate)
            {
                hub.timers.Remove(this);
            }
        }
    }
}
