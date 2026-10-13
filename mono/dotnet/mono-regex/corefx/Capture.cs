








namespace MonoCorefx.Text.RegularExpressions
{
    public class Capture
    {
        internal Capture(string text, int index, int length)
        {
            Text = text;
            Index = index;
            Length = length;
        }

        public int Index { get; private protected set; }

        public int Length { get; private protected set; }

        internal string Text { get; private protected set; }

        public string Value => Text.Substring(Index, Length);

        public override string ToString() => Value;

        internal ReadOnlySpan<char> GetLeftSubstring() => Text.AsSpan(0, Index);

        internal ReadOnlySpan<char> GetRightSubstring() => Text.AsSpan(Index + Length, Text.Length - Index - Length);
    }
}
