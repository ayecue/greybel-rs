using MonoFixtures.Output;

namespace MonoFixtures.Fixtures;

internal sealed class NumberFixtureBuilder : IFixtureBuilder
{
    private static readonly CultureInfo Invariant = CultureInfo.InvariantCulture;

    private static readonly string[] DoubleFormats =
    [
        "",
        "G",
        "g",
        "G5",
        "G15",
        "G16",
        "G17",
        "R",
        "E",
        "E0",
        "E6",
        "e3",
        "E20",
        "F",
        "F0",
        "F2",
        "F6",
        "N",
        "N0",
        "N1",
        "N3",
        "P",
        "P1",
        "C",
        "C3",
        "0",
        "0.0#####",
        "0.00",
        "#,##0.00",
        "#,##0.00;(#,##0.00);zero",
        "0.###E+0",
        "00.00E-00",
        "#.##",
        "0%",
        "0.0\u2030",
        "'lit'0;\"neg\"0",
        "\\#0.0",
        "#,#,,",
        "0,0.0",
        "0;-0;",
        ";;",
        "#",
        "X",
        "D",
        "Z9",
        "F123",
        "E0000",
    ];

    private static readonly string[] FloatFormats =
    [
        "",
        "R",
        "G",
        "G9",
        "E",
        "E8",
        "F2",
        "N1",
        "0.0#####",
        "0.00",
        "#,##0.0",
    ];

    private static readonly NumberStyles[] ParseStyles =
    [
        NumberStyles.Any,
        NumberStyles.Float | NumberStyles.AllowThousands,
        NumberStyles.Number | NumberStyles.AllowExponent,
        NumberStyles.Integer,
        NumberStyles.Number,
        NumberStyles.HexNumber,
        NumberStyles.None,
    ];

    private static readonly string[] ParseTexts =
    [
        "42",
        " 42 ",
        "-1.5",
        "+3",
        "1e3",
        "1E-2",
        "1,000",
        "1,2,3",
        ",123,4.56",
        "1234,.56",
        "1,.56",
        ",.56",
        "(5)",
        "5-",
        "$5",
        "\u00a45",
        "5\u00a4",
        "-\u00a41",
        "\u00a4-1",
        "(\u00a41)",
        "0x10",
        "Infinity",
        "-Infinity",
        "infinity",
        " Infinity ",
        "NaN",
        "nan",
        "\u221e",
        ".5",
        "5.",
        "1_000",
        "abc",
        "",
        "  ",
        "1.2.3",
        "--1",
        "+-1",
        "- 1",
        "1e",
        "1e+",
        "1e-5",
        "\u0661\u0662",
        "\t12\n",
        "12\0",
        "12\0\0",
        "1\0 2",
        "0",
        "-0",
        "-0.0",
        "000123",
        "123456789012345678901234567890",
        "0.1234567890123456789",
        "1e309",
        "-1e309",
        "1e-400",
        "4.9e-324",
        "2.4703282292062328e-324",
        "2.4703282292062327e-324",
        "1.7976931348623157e308",
        "1.7976931348623158e308",
        "2147483647",
        "2147483648",
        "-2147483648",
        "-2147483649",
        "99999999999",
        " +7 ",
        "1 000",
        "1\u00a0000",
        "1e1000",
        "1e-1000",
        "0e5",
        "0.0e-5",
        "00",
        "0.",
        ".",
        "e5",
        "+",
        "-",
        "1.5e3.2",
        "12a",
        "a12",
        "1,",
        "1,,2",
        "0,5",
        "\uff11\uff12",
        "7FFFFFFF",
        "ffffffff",
        "100000000",
        "  1f  ",
        "0001",
        "0.1",
        "0.2",
        "0.3",
        "9007199254740993",
        "123456789.123456789",
        "1.0000000000000002",
        "0.000000000000000000000000000001",
        "5e-324",
        "1.5\u00a0",
        "\u00a01.5",
        "1e01",
        "1E+0010",
        "\u3000" + "1",
    ];

    private static readonly int[] RoundingDigits = [-1, 0, 1, 2, 3, 6, 15, 16];

    private static readonly double[] RoundingValues =
    [
        0.49999999999999994,
        -0.49999999999999994,
        4503599627370497.0,
        0.5,
        1.5,
        2.5,
        -2.5,
        3.5,
        1e300,
        2.675,
        1.005,
        0.125,
        1e16 + 1,
    ];

    public string CrateName => "mono-number";

    public void Write(Crate crate)
    {
        List<double> doubles = GetDoubles().ToList();
        WriteFormat(crate.GetFixturePath("format.json"), doubles);
        WriteParse(crate.GetFixturePath("parse.json"));
        WriteMath(crate.GetFixturePath("math.json"), doubles);
    }

    private static void WriteFormat(string path, List<double> doubles)
    {
        var json = new JsonWriter();
        json.WriteStartObject();
        json.WriteProperty(
            "double_formats",
            Json.FormatArray(DoubleFormats.Select(Json.FormatString))
        );
        json.WriteArray(
            "doubles",
            doubles.Select(value =>
                FormatToStringRow(
                    Json.FormatBits(value),
                    DoubleFormats,
                    format => value.ToString(format, Invariant)
                )
            )
        );
        json.WriteProperty(
            "float_formats",
            Json.FormatArray(FloatFormats.Select(Json.FormatString))
        );
        json.WriteArray(
            "floats",
            GetFloats()
                .Select(value =>
                    FormatToStringRow(
                        Json.FormatBits(value),
                        FloatFormats,
                        format => value.ToString(format, Invariant)
                    )
                )
        );
        json.WriteEndObject();
        json.Save(path);
    }

    private static string FormatToStringRow(
        string bits,
        string[] formats,
        Func<string, string> toString
    ) =>
        Json.FormatArray(
            new[] { bits }.Concat(
                formats.Select(format =>
                    FormatOrNull<FormatException>(() => Json.FormatString(toString(format)))
                )
            )
        );

    private static void WriteParse(string path)
    {
        var json = new JsonWriter();
        json.WriteStartObject();
        json.WriteProperty(
            "styles",
            Json.FormatArray(ParseStyles.Select(style => Json.FormatInt((int)style)))
        );
        json.WriteArray("texts", ParseTexts.Concat(GetNumericTexts()).Select(FormatParseRow));
        json.WriteEndObject();
        json.Save(path);
    }

    private static string FormatParseRow(string text)
    {
        var results = new List<string> { Json.FormatString(text) };
        foreach (NumberStyles style in ParseStyles)
        {
            string number =
                (style & NumberStyles.AllowHexSpecifier) == 0
                && double.TryParse(text, style, Invariant, out double d)
                    ? Json.FormatBits(d)
                    : Json.Null;
            string integer = int.TryParse(text, style, Invariant, out int i)
                ? Json.FormatInt(i)
                : Json.Null;
            results.Add(Json.FormatArray([number, integer]));
        }
        return Json.FormatArray(results);
    }

    private static void WriteMath(string path, List<double> doubles)
    {
        var json = new JsonWriter();
        json.WriteStartObject();
        json.WriteProperty(
            "digits",
            Json.FormatArray(RoundingDigits.Select(digits => Json.FormatInt(digits)))
        );
        json.WriteArray("values", doubles.Concat(RoundingValues).Select(FormatMathRow));
        json.WriteEndObject();
        json.Save(path);
    }

    private static string FormatMathRow(double value)
    {
        var results = new List<string>
        {
            Json.FormatBits(value),
            Json.FormatBits(Math.Round(value)),
            Json.FormatInt(value.GetHashCode()),
        };
        results.AddRange(
            RoundingDigits.Select(digits =>
                FormatOrNull<ArgumentOutOfRangeException>(() =>
                    Json.FormatBits(Math.Round(value, digits))
                )
            )
        );
        return Json.FormatArray(results);
    }

    private static string FormatOrNull<TException>(Func<string> format)
        where TException : Exception
    {
        try
        {
            return format();
        }
        catch (TException)
        {
            return Json.Null;
        }
    }

    private const ulong DoubleSeed = 0x5eed;
    private const ulong FloatSeed = 0xf10a7;

    private static IEnumerable<double> GetDoubles()
    {
        double[] curatedValues =
        [
            0.0,
            -0.0,
            1,
            -1,
            0.5,
            1.5,
            2.5,
            -2.5,
            0.1,
            0.2,
            0.3,
            1.0 / 3,
            2.0 / 3,
            0.1234565,
            1.0000005,
            2.5e-7,
            123456.7890125,
            0.30000000000000004,
            1234567890.1234567,
            9999999999.9999995,
            1e15 + 1,
            123456789012345678,
            1e22,
            1e-6,
            5.0000005e-7,
            1e10,
            1e10 + 0.5,
            -1e10 - 0.5,
            9999999999.5,
            -1e-7,
            2.675,
            0.125,
            0.375,
            1.005,
            1e-5,
            1e-4,
            0.00001234,
            12345.6789,
            -12345.6789,
            1e15,
            999999999999999,
            1e16,
            9007199254740993,
            double.Epsilon,
            double.MaxValue,
            double.MinValue,
            2.2250738585072014E-308,
            double.NaN,
            double.PositiveInfinity,
            double.NegativeInfinity,
            1e300,
            1e-300,
            0.05,
            0.005,
            0.0005,
            0.95,
            0.995,
            9.5,
            99.5,
            999.5,
            0.45,
            1.45,
            1234.5,
            1e21,
            1e-21,
            123.456,
            -0.000001,
            1e-7,
            0.49999999999999994,
            4503599627370497,
            1e100,
            1345,
            1000,
            1000000,
            1234567,
            -0.4,
            -0.5,
            0.0000004,
            -1e-300,
            7.0,
            42.42,
            0.999999,
            0.9999995,
            99999.99995,
        ];
        foreach (double value in curatedValues)
        {
            yield return value;
        }
        var random = new SplitMix(DoubleSeed);
        for (int i = 0; i < 40; i++)
        {
            double value = BitConverter.Int64BitsToDouble((long)random.Next());
            if (!double.IsNaN(value) && !double.IsInfinity(value))
            {
                yield return value;
            }
        }
        for (int i = 0; i < 40; i++)
        {
            yield return random.ApplyRandomSign(
                random.NextDigits(1 + random.NextInt(17)) / Math.Pow(10, random.NextInt(21))
            );
        }
        for (int i = 0; i < 30; i++)
        {
            double halves = random.NextDigits(1 + random.NextInt(14)) * 10 + 5;
            yield return random.ApplyRandomSign(halves / Math.Pow(10, 1 + random.NextInt(16)));
        }
        for (int i = 0; i < 20; i++)
        {
            yield return random.ApplyRandomSign(random.NextDigits(1 + random.NextInt(18)));
        }
    }

    private static IEnumerable<float> GetFloats()
    {
        float[] curatedValues =
        [
            0f,
            -0f,
            1f,
            0.1f,
            0.2f,
            0.3f,
            1f / 3,
            2.5f,
            0.05f,
            1234.5f,
            12345.67f,
            1e10f,
            1e-10f,
            16777217f,
            float.Epsilon,
            float.MaxValue,
            float.MinValue,
            float.NaN,
            float.PositiveInfinity,
            float.NegativeInfinity,
            3.14159265f,
            0.15f,
            0.25f,
            0.35f,
            99.95f,
            1.05f,
        ];
        foreach (float value in curatedValues)
        {
            yield return value;
        }
        var random = new SplitMix(FloatSeed);
        for (int i = 0; i < 30; i++)
        {
            float value = BitConverter.ToSingle(BitConverter.GetBytes((uint)random.Next()), 0);
            if (!float.IsNaN(value) && !float.IsInfinity(value))
            {
                yield return value;
            }
        }
        for (int i = 0; i < 30; i++)
        {
            yield return (float)
                random.ApplyRandomSign(
                    random.NextDigits(1 + random.NextInt(8)) / Math.Pow(10, random.NextInt(8))
                );
        }
    }

    private const ulong ParseTextSeed = 0x7e47;

    private static IEnumerable<string> GetNumericTexts()
    {
        var random = new SplitMix(ParseTextSeed);
        for (int i = 0; i < 300; i++)
        {
            int count = 1 + random.NextInt(25);
            var digits = new char[count];
            for (int j = 0; j < count; j++)
            {
                digits[j] = (char)('0' + random.NextInt(10));
            }
            string text = new(digits);
            if (random.NextInt(2) == 0)
            {
                int point = random.NextInt(count + 1);
                text = text.Substring(0, point) + "." + text.Substring(point);
            }
            if (random.NextInt(2) == 0)
            {
                text += "e" + (random.NextInt(661) - 330).ToString(Invariant);
            }
            if (random.NextInt(4) == 0)
            {
                text = "-" + text;
            }
            yield return text;
        }
    }
}
