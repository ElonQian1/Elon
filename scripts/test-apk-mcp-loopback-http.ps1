$ErrorActionPreference = 'Stop'
$tokens = $null
$errors = $null
$sourcePath = Join-Path $PSScriptRoot 'invoke-apk-mcp.ps1'
$ast = [System.Management.Automation.Language.Parser]::ParseFile(
    $sourcePath, [ref]$tokens, [ref]$errors)
if ($errors.Count) { throw 'APK MCP helper must parse before testing.' }
$function = $ast.Find({ param($node)
    $node -is [System.Management.Automation.Language.FunctionDefinitionAst] -and
        $node.Name -eq 'Invoke-ApkMcpLoopbackJson'
}, $true)
if (-not $function) { throw 'Missing direct loopback request owner.' }
. ([scriptblock]::Create($function.Extent.Text))
$requests = @($ast.FindAll({ param($node)
    $node -is [System.Management.Automation.Language.CommandAst] -and
        $node.GetCommandName() -eq 'Invoke-ApkMcpLoopbackJson'
}, $true))
if ($requests.Count -ne 4 -or (Get-Content $sourcePath -Raw).Contains('Invoke-RestMethod')) {
    throw 'Every health probe and final MCP request must use the direct request owner.'
}

Add-Type -AssemblyName System.Net.Http
Add-Type -TypeDefinition @'
using System;
using System.IO;
using System.Net;
using System.Net.Sockets;
using System.Text;
using System.Threading;
using System.Threading.Tasks;

public sealed class ApkMcpForcedProxy : IWebProxy {
    readonly Uri address;
    public ApkMcpForcedProxy(int port) { address = new Uri("http://127.0.0.1:" + port); }
    public ICredentials Credentials { get; set; }
    public Uri GetProxy(Uri destination) { return address; }
    public bool IsBypassed(Uri host) { return false; }
}

public sealed class ApkMcpHttpFixture : IDisposable {
    readonly TcpListener listener = new TcpListener(IPAddress.Loopback, 0);
    readonly Task worker;
    volatile bool disposed;
    public volatile string Mode = "ok";
    public string LastBody = "";
    public string LastRequest = "";
    public int Requests;
    public int RedirectPort;
    public int Port { get { return ((IPEndPoint)listener.LocalEndpoint).Port; } }
    public ApkMcpHttpFixture() {
        listener.Start();
        worker = Task.Run((Action)Run);
    }
    void Run() {
        while (!disposed) {
            try {
                using (var socket = listener.AcceptTcpClient())
                using (var stream = socket.GetStream()) {
                    socket.ReceiveTimeout = 3000;
                    socket.SendTimeout = 3000;
                    var header = new StringBuilder();
                    while (!header.ToString().EndsWith("\r\n\r\n")) {
                        int next = stream.ReadByte();
                        if (next < 0 || header.Length >= 65536) throw new IOException();
                        header.Append((char)next);
                    }
                    string[] lines = header.ToString().Split(new[] { "\r\n" }, StringSplitOptions.None);
                    LastRequest = lines[0];
                    int length = 0;
                    foreach (string line in lines) {
                        if (line.StartsWith("Content-Length:", StringComparison.OrdinalIgnoreCase))
                            length = int.Parse(line.Substring(15).Trim());
                    }
                    if (length > 65536) throw new IOException();
                    var data = new byte[length];
                    int offset = 0;
                    while (offset < length) {
                        int read = stream.Read(data, offset, length - offset);
                        if (read == 0) throw new IOException();
                        offset += read;
                    }
                    LastBody = Encoding.UTF8.GetString(data);
                    Interlocked.Increment(ref Requests);
                    string mode = Mode;
                    if (mode == "redirect") {
                        Send(stream, "HTTP/1.1 307 Temporary Redirect\r\nLocation: http://127.0.0.1:" +
                            RedirectPort + "/mcp\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
                    } else if (mode == "stall") {
                        Send(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n" +
                            "Content-Length: 100\r\nConnection: close\r\n\r\n{");
                        for (int i = 0; i < 50 && !disposed; i++) Thread.Sleep(100);
                    } else {
                        string body = mode == "proxy" ? "{\"via_proxy\":true}" : "{\"ok\":true}";
                        Send(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: " +
                            Encoding.UTF8.GetByteCount(body) + "\r\nConnection: close\r\n\r\n" + body);
                    }
                }
            } catch { if (disposed) return; }
        }
    }
    static void Send(Stream stream, string text) {
        byte[] bytes = Encoding.UTF8.GetBytes(text);
        stream.Write(bytes, 0, bytes.Length);
        stream.Flush();
    }
    public void Dispose() { disposed = true; listener.Stop(); worker.Wait(3500); }
}
'@
$origin = [ApkMcpHttpFixture]::new()
$proxy = [ApkMcpHttpFixture]::new()
$proxy.Mode = 'proxy'
$originalProxy = [System.Net.WebRequest]::DefaultWebProxy
$httpProxyProperty = [System.Net.Http.HttpClient].GetProperty('DefaultProxy')
$originalHttpProxy = if ($httpProxyProperty) { $httpProxyProperty.GetValue($null) } else { $null }
try {
    $fakeProxy = [ApkMcpForcedProxy]::new($proxy.Port)
    [System.Net.WebRequest]::DefaultWebProxy = $fakeProxy
    if ($httpProxyProperty) { $httpProxyProperty.SetValue($null, $fakeProxy) }
    $controlHandler = [System.Net.Http.HttpClientHandler]::new()
    $controlHandler.Proxy = $fakeProxy
    $control = [System.Net.Http.HttpClient]::new($controlHandler)
    try {
        $control.Timeout = [TimeSpan]::FromSeconds(3)
        # Framework HttpClient bypasses loopback even with an explicit proxy.
        $controlHost = if ($httpProxyProperty) { '127.0.0.1' } else { 'apk-mcp-fixture.invalid' }
        $reply = $control.GetStringAsync("http://${controlHost}:$($origin.Port)/health").GetAwaiter().GetResult()
        if (-not ($reply | ConvertFrom-Json).via_proxy) { throw 'Proxy trap was not exercised.' }
    } finally { $control.Dispose() }
    $proxyRequests = $proxy.Requests
    $reply = Invoke-ApkMcpLoopbackJson -Port $origin.Port -Path /health -TimeoutSec 3
    if (-not $reply.ok -or $origin.LastRequest -notmatch '^GET /health ') { throw 'Direct GET failed.' }
    $unicodeText = [string][char]0x4f60 + [char]0x597d
    $body = @{ jsonrpc='2.0'; params=@{ auth_token='synthetic-only'; text=$unicodeText } } |
        ConvertTo-Json -Depth 4 -Compress
    $reply = Invoke-ApkMcpLoopbackJson -Port $origin.Port -Path /mcp -Method Post -Body $body -TimeoutSec 3
    if (-not $reply.ok -or $origin.LastBody -cne $body -or $proxy.Requests -ne $proxyRequests) {
        throw 'POST body changed or a loopback request reached the proxy.'
    }
    $origin.RedirectPort = $proxy.Port
    $origin.Mode = 'redirect'
    $rejected = $false
    try { Invoke-ApkMcpLoopbackJson -Port $origin.Port -Path /mcp -Method Post -Body $body -TimeoutSec 3 | Out-Null }
    catch { $rejected = $_.Exception.Message -match '307' }
    if (-not $rejected -or $proxy.Requests -ne $proxyRequests) { throw 'Credential-bearing redirect was followed.' }
    $origin.Mode = 'stall'
    $timer = [Diagnostics.Stopwatch]::StartNew()
    $failed = $false
    try { Invoke-ApkMcpLoopbackJson -Port $origin.Port -Path /health -TimeoutSec 1 | Out-Null }
    catch { $failed = $true }
    if (-not $failed -or $timer.Elapsed.TotalSeconds -gt 4) { throw 'Response body escaped the deadline.' }
    Write-Output 'APK_MCP_LOOPBACK_HTTP=passed cases=4 (direct GET, UTF8 POST, redirect rejection, stalled body)'
} finally {
    [System.Net.WebRequest]::DefaultWebProxy = $originalProxy
    if ($httpProxyProperty) { $httpProxyProperty.SetValue($null, $originalHttpProxy) }
    $origin.Dispose()
    $proxy.Dispose()
}
