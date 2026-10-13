



using System.Runtime.CompilerServices;

namespace System.Collections.Generic
{
    internal ref partial struct ValueListBuilder<T>
    {
        [MethodImpl(MethodImplOptions.AggressiveInlining)]
        public T Pop()
        {
            _pos--;
            return _span[_pos];
        }
    }
}
