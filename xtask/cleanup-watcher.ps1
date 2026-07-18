param([Parameter(Mandatory = $true)][string]$Dir)
$ErrorActionPreference = 'SilentlyContinue'
Add-Type -TypeDefinition @'
using System;
using System.Text;
using System.Runtime.InteropServices;
public static class WsNative {
  [DllImport("kernel32.dll", CharSet=CharSet.Unicode, SetLastError=true)]
  static extern IntPtr CreateFileW(string n, uint acc, uint share, IntPtr sec, uint disp, uint flags, IntPtr tmpl);
  [DllImport("kernel32.dll", CharSet=CharSet.Unicode, SetLastError=true)]
  static extern int GetFinalPathNameByHandleW(IntPtr h, StringBuilder buf, int len, int flags);
  [DllImport("kernel32.dll")] static extern bool CloseHandle(IntPtr h);
  public static IntPtr Open(string p){ return CreateFileW(p, 0x80000000, 0x7, IntPtr.Zero, 3, 0x02000000, IntPtr.Zero); }
  public static string Final(IntPtr h){ var sb=new StringBuilder(1024); int n=GetFinalPathNameByHandleW(h,sb,sb.Capacity,0); if(n<=0) return ""; var s=sb.ToString(); if(s.StartsWith(@"\\?\")) s=s.Substring(4); return s; }
  public static void Close(IntPtr h){ CloseHandle(h); }
}
'@
$h = [WsNative]::Open($Dir)
if ($h.ToInt64() -eq -1) { exit }
$sh = New-Object -ComObject Shell.Application
$target = [IO.Path]::GetFullPath($Dir).TrimEnd('\')
Invoke-Item -LiteralPath $Dir
$hwnd = $null
for ($i = 0; $i -lt 50 -and $null -eq $hwnd; $i++) {
    foreach ($w in @($sh.Windows())) {
        $p = $null; try { $p = $w.Document.Folder.Self.Path }catch {}
        if ($p -and ([IO.Path]::GetFullPath($p).TrimEnd('\') -ieq $target)) { $hwnd = $w.HWND; break } 
    }
    if ($null -eq $hwnd) { Start-Sleep -Milliseconds 100 }
}
if ($null -eq $hwnd) { [WsNative]::Close($h); exit }
while ($true) {
    $open = $false
    foreach ($w in @($sh.Windows())) { try { if ($w.HWND -eq $hwnd) { $open = $true; break } }catch {} }
    if (-not $open) { break }
    Start-Sleep -Milliseconds 500
}
$final = [WsNative]::Final($h)
[WsNative]::Close($h)
if ($final -and (Test-Path -LiteralPath $final)) { Remove-Item -LiteralPath $final -Recurse -Force }
