namespace GreyHackMessageHook.Game
{
    public static class GameMessageConversion
    {
        public static T ToGameMessage<T>(this RawMessage message)
            where T : NetworkMessages.RpcMessage
        {
            return message.ToObject<T>();
        }
    }
}
