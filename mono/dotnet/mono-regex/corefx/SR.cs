

namespace System
{
    internal static partial class SR
    {
        internal const string AlternationCantCapture = "Alternation conditions do not capture and cannot be named.";
        internal const string AlternationCantHaveComment = "Alternation conditions cannot be comments.";
        internal const string Arg_ArrayPlusOffTooSmall = "Destination array is not long enough to copy all the items in the collection. Check array index and length.";
        internal const string ArgumentNull_ArrayWithNullElements = "The array cannot contain null elements.";
        internal const string BadClassInCharRange = "Cannot include class \\{0} in character range.";
        internal const string BeginIndexNotNegative = "Start index cannot be less than 0 or greater than input length.";
        internal const string CaptureGroupOutOfRange = "Capture group numbers must be less than or equal to Int32.MaxValue.";
        internal const string CapnumNotZero = "Capture number cannot be zero.";
        internal const string CountTooSmall = "Count cannot be less than -1.";
        internal const string EnumNotStarted = "Enumeration has either not started or has already finished.";
        internal const string IllegalCondition = "Illegal conditional (?(...)) expression.";
        internal const string IllegalDefaultRegexMatchTimeoutInAppDomain = "AppDomain data '{0}' contains an invalid value or object for specifying a default matching timeout for System.Text.RegularExpressions.Regex.";
        internal const string IllegalEndEscape = "Illegal \\ at end of pattern.";
        internal const string IllegalRange = "Illegal {x,y} with x > y.";
        internal const string IncompleteSlashP = "Incomplete \\p{X} character escape.";
        internal const string InternalError = "Internal error in ScanRegex.";
        internal const string InvalidGroupName = "Invalid group name: Group names must begin with a word character.";
        internal const string InvalidEmptyArgument = "Argument {0} cannot be zero-length.";
        internal const string LengthNotNegative = "Length cannot be less than 0 or exceed input length.";
        internal const string MalformedNameRef = "Malformed \\k<...> named back reference.";
        internal const string MalformedReference = "(?({0}) ) malformed.";
        internal const string MalformedSlashP = "Malformed \\p{X} character escape.";
        internal const string MakeException = "parsing \"{0}\" - {1}";
        internal const string MissingControl = "Missing control character.";
        internal const string NestedQuantify = "Nested quantifier {0}.";
        internal const string NoResultOnFailed = "Result cannot be called on a failed Match.";
        internal const string NotEnoughParens = "Not enough )'s.";
        internal const string NotSupported_ReadOnlyCollection = "Collection is read-only.";
        internal const string OnlyAllowedOnce = "This operation is only allowed once per object.";
        internal const string PlatformNotSupported_CompileToAssembly = "This platform does not support writing compiled regular expressions to an assembly.";
        internal const string QuantifyAfterNothing = "Quantifier {x,y} following nothing.";
        internal const string RegexMatchTimeoutException_Occurred = "The RegEx engine has timed out while trying to match a pattern to an input string. This can occur for many reasons, including very large inputs or excessive backtracking caused by nested quantifiers, back-references and other factors.";
        internal const string ReplacementError = "Replacement pattern error.";
        internal const string ReversedCharRange = "[x-y] range in reverse order.";
        internal const string SubtractionMustBeLast = "A subtraction must be the last element in a character class.";
        internal const string TooFewHex = "Insufficient hexadecimal digits.";
        internal const string TooManyAlternates = "Too many | in (?()|).";
        internal const string TooManyParens = "Too many )'s.";
        internal const string UndefinedBackref = "Reference to undefined group number {0}.";
        internal const string UndefinedNameRef = "Reference to undefined group name {0}.";
        internal const string UndefinedReference = "(?({0}) ) reference to undefined group.";
        internal const string UnexpectedOpcode = "Unexpected opcode in regular expression generation: {0}.";
        internal const string UnimplementedState = "Unimplemented state.";
        internal const string UnknownProperty = "Unknown property '{0}'.";
        internal const string UnrecognizedControl = "Unrecognized control character.";
        internal const string UnrecognizedEscape = "Unrecognized escape sequence \\{0}.";
        internal const string UnrecognizedGrouping = "Unrecognized grouping construct.";
        internal const string UnterminatedBracket = "Unterminated [] set.";
        internal const string UnterminatedComment = "Unterminated (?#...) comment.";

        internal static string Format(string format, object arg0) => string.Format(format, arg0);
        internal static string Format(string format, object arg0, object arg1) => string.Format(format, arg0, arg1);
        internal static string Format(string format, object arg0, object arg1, object arg2) => string.Format(format, arg0, arg1, arg2);
        internal static string Format(string format, params object[] args) => string.Format(format, args);
    }
}
