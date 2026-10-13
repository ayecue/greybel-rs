using GreyHackUtil.Protocol;

namespace GreyHackUtil.Tests;

public class NamesTests
{
    [Theory]
    [InlineData("windowPID", "window_pid")]
    [InlineData("zipHelperGO", "zip_helper_go")]
    [InlineData("PID", "pid")]
    [InlineData("PIDs", "pids")]
    [InlineData("IPsList", "ips_list")]
    [InlineData("HTTPServer", "http_server")]
    [InlineData("isOS", "is_os")]
    [InlineData("isAdmin", "is_admin")]
    [InlineData("propActiveUser", "prop_active_user")]
    [InlineData("ip2", "ip_2")]
    [InlineData("x", "x")]
    public void ShouldSnakeCaseNames(string name, string expected)
    {
        string input = name;

        string snake = ProtocolNameTable.ToSnakeCase(input);

        Assert.Equal(expected, snake);
    }
}
