



using System.Threading;

namespace MonoCorefx.Text.RegularExpressions
{
    internal sealed class ExclusiveReference
    {
        private RegexRunner _ref;
        private RegexRunner _obj;
        private volatile int _locked;

        public RegexRunner Get()
        {


            if (0 == Interlocked.Exchange(ref _locked, 1))
            {

                RegexRunner obj = _ref;


                if (obj == null)
                {
                    _locked = 0;

                    return null;
                }


                _obj = obj;

                return obj;
            }

            return null;
        }

        public void Release(RegexRunner obj)
        {
            if (obj == null)
                throw new ArgumentNullException(nameof(obj));


            if (_obj == obj)
            {
                _obj = null;
                _locked = 0;

                return;
            }


            if (_obj == null)
            {

                if (0 == Interlocked.Exchange(ref _locked, 1))
                {

                    if (_ref == null)
                        _ref = obj;


                    _locked = 0;

                    return;
                }
            }
        }
    }
}
