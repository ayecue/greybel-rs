using System.Runtime.InteropServices;
using MonoCorefx.Text.RegularExpressions;

namespace MonoRegex;

public static unsafe class Exports
{
    [UnmanagedCallersOnly(EntryPoint = "mono_regex_is_match")]
    public static void IsMatch(
        char* input,
        int inputLength,
        char* pattern,
        int patternLength,
        int options,
        int timeoutMilliseconds,
        Result* result
    )
    {
        var respond = new Call(
            input,
            inputLength,
            pattern,
            patternLength,
            options,
            timeoutMilliseconds
        );
        Respond(
            result,
            () =>
                Response.CreateMatch(
                    Regex.IsMatch(respond.Input, respond.Pattern, respond.Options, respond.Timeout)
                )
        );
    }

    [UnmanagedCallersOnly(EntryPoint = "mono_regex_matches")]
    public static void Matches(
        char* input,
        int inputLength,
        char* pattern,
        int patternLength,
        int options,
        int timeoutMilliseconds,
        Result* result
    )
    {
        var respond = new Call(
            input,
            inputLength,
            pattern,
            patternLength,
            options,
            timeoutMilliseconds
        );
        Respond(
            result,
            () =>
                Response.CreateMatches(
                    Regex.Matches(respond.Input, respond.Pattern, respond.Options, respond.Timeout)
                )
        );
    }

    [UnmanagedCallersOnly(EntryPoint = "mono_regex_split")]
    public static void Split(
        char* input,
        int inputLength,
        char* pattern,
        int patternLength,
        int options,
        int timeoutMilliseconds,
        Result* result
    )
    {
        var respond = new Call(
            input,
            inputLength,
            pattern,
            patternLength,
            options,
            timeoutMilliseconds
        );
        Respond(
            result,
            () =>
                Response.CreateSplit(
                    Regex.Split(respond.Input, respond.Pattern, respond.Options, respond.Timeout)
                )
        );
    }

    [UnmanagedCallersOnly(EntryPoint = "mono_regex_replace")]
    public static void Replace(
        char* input,
        int inputLength,
        char* pattern,
        int patternLength,
        char* replacement,
        int replacementLength,
        int options,
        int timeoutMilliseconds,
        Result* result
    )
    {
        var respond = new Call(
            input,
            inputLength,
            pattern,
            patternLength,
            options,
            timeoutMilliseconds
        );
        string replacementText = new(replacement, 0, replacementLength);
        Respond(
            result,
            () =>
                Response.CreateReplacement(
                    Regex.Replace(
                        respond.Input,
                        respond.Pattern,
                        replacementText,
                        respond.Options,
                        respond.Timeout
                    )
                )
        );
    }

    [UnmanagedCallersOnly(EntryPoint = "mono_regex_free")]
    public static void Release(Result* result) => result->Release();

    private static void Respond(Result* result, Func<Response> respond)
    {
        Response response;
        try
        {
            response = respond();
        }
        catch (RegexMatchTimeoutException)
        {
            response = Response.CreateTimeout();
        }
        catch (Exception exception)
        {
            response = Response.CreateFailure(exception.Message);
        }
        *result = Result.From(response);
    }
}
