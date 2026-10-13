





namespace MonoCorefx.Text.RegularExpressions
{
    public abstract class RegexRunnerFactory
    {
        protected RegexRunnerFactory() { }
        protected internal abstract RegexRunner CreateInstance();
    }
}
