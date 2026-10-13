#nullable enable
using Duplex;

namespace GreyHackMessageHook;

public interface IGame
{
    void Send(Message message);
}
