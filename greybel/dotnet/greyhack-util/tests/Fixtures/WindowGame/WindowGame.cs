public class Ventana
{
    protected int PID = -1;

    public virtual int GetPID()
    {
        return PID;
    }
}

public class Chat : Ventana { }

public class Panel
{
    private int chatPID;

    private int count;

    public void Configure(Chat chat, int number)
    {
        chatPID = chat.GetPID();
        count = number;
    }

    public void SendsAPidOfAWindowFromAField() => Sink.Send(chatPID);

    public void SendsANumberFromAField() => Sink.Send(count);
}

public class Finder : Ventana
{
    private Chat chat = new Chat();

    public void SendsItsOwnPid() => Sink.Send(GetPID());

    public void SendsItsOwnPidField() => Sink.Send(PID);

    public void SendsItsOwnPidThroughALocal()
    {
        int pid = GetPID();
        Sink.Send(pid);
    }

    public void SendsAnotherWindowPid() => Sink.Send(chat.GetPID());

    public void SendsANumber() => Sink.Send(3);

    public void Opens()
    {
        SendsAPassedPid(GetPID());
        SendsAPassedPid(chat.GetPID());
        SendsAPassedNumber(GetPID());
        SendsAPassedNumber(7);
        Sink.Register(SendsADelegatesPid);
        SendsADelegatesPid(GetPID());
    }

    private void SendsAPassedPid(int pid) => Sink.Send(pid);

    private void SendsAPassedNumber(int pid) => Sink.Send(pid);

    private void SendsADelegatesPid(int pid) => Sink.Send(pid);
}

public static class Sink
{
    public static void Send(int value) { }

    public static void Register(System.Action<int> callback) { }
}
