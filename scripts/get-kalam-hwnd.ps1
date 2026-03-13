# Get HWND(s) for Kalam windows (main and overlay). Run with: .\scripts\get-kalam-hwnd.ps1
Add-Type -TypeDefinition @"
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;
public class KalamHwndHelper {
    [DllImport("user32.dll")]
    public static extern bool EnumWindows(EnumWindowsProc lpEnumFunc, IntPtr lParam);
    public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr lParam);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    public static extern int GetWindowText(IntPtr hWnd, StringBuilder lpString, int nMaxCount);
    [DllImport("user32.dll")]
    public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint lpdwProcessId);
    [DllImport("user32.dll")]
    public static extern bool IsWindowVisible(IntPtr hWnd);
    public static List<int> TargetPids;
    public static List<string> Results = new List<string>();
    public static bool Callback(IntPtr hWnd, IntPtr lParam) {
        uint pid = 0;
        GetWindowThreadProcessId(hWnd, out pid);
        if (!TargetPids.Contains((int)pid)) return true;
        var sb = new StringBuilder(256);
        GetWindowText(hWnd, sb, sb.Capacity);
        bool vis = IsWindowVisible(hWnd);
        long h = hWnd.ToInt64();
        Results.Add(string.Format("{0}|{1}|{2}", h, sb.ToString(), vis ? "visible" : "hidden"));
        return true;
    }
}
"@

$kalamProcess = Get-Process -Name "kalam-voice" -ErrorAction SilentlyContinue
if (-not $kalamProcess) {
    Write-Host "kalam-voice.exe is not running. Start the app (e.g. npm run tauri dev) and run this script again."
    exit 1
}
[KalamHwndHelper]::TargetPids = [System.Collections.Generic.List[int]]::new()
$kalamProcess.Id | ForEach-Object { [KalamHwndHelper]::TargetPids.Add($_) }
[KalamHwndHelper]::Results = [System.Collections.Generic.List[string]]::new()
$callback = [System.Delegate]::CreateDelegate([KalamHwndHelper+EnumWindowsProc], [KalamHwndHelper].GetMethod("Callback"))
$null = [KalamHwndHelper]::EnumWindows($callback, [IntPtr]::Zero)

Write-Host "Kalam windows (kalam-voice.exe):"
foreach ($line in [KalamHwndHelper]::Results) {
    $parts = $line -split '\|', 3
    $hwnd = [long]$parts[0]
    $title = $parts[1]
    $vis = $parts[2]
    Write-Host ("  HWND 0x{0:X} ({1})  Title: ""{2}""  [{3}]" -f $hwnd, $hwnd, $title, $vis)
}
if ([KalamHwndHelper]::Results.Count -eq 0) {
    Write-Host "  (no top-level windows found for this process)"
}
