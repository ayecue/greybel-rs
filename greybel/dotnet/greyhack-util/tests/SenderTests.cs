using GreyHackUtil.Extraction;

namespace GreyHackUtil.Tests;

public class SenderTests
{
    [Theory]
    [InlineData("GetActiveUser", "activeUser")]
    [InlineData("GetPID", "pid")]
    [InlineData("GetIPAddress", "ipAddress")]
    [InlineData("GetRuta", "ruta")]
    public void ShouldNameAValueAfterTheGetterItCameFrom(string getter, string expected)
    {
        string name = SenderReader.GetGetterName(getter);

        Assert.Equal(expected, name);
    }

    [Theory]
    [InlineData("GetPID", true)]
    [InlineData("Getaway", false)]
    [InlineData("Get", false)]
    public void ShouldTakeOnlyGettersForGetters(string method, bool expected)
    {
        bool getter = SenderReader.IsGetter(method);

        Assert.Equal(expected, getter);
    }

    [Theory]
    [InlineData("System.Int32", "int")]
    [InlineData("System.Byte[]", "bytes")]
    public void ShouldNameAValueWithoutANameAfterItsFieldType(string fieldType, string expected)
    {
        string name = SenderReader.GetStem(fieldType);

        Assert.Equal(expected, name);
    }
}
