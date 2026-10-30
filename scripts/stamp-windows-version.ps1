param(
    [Parameter(Mandatory)][string]$Binary,
    [Parameter(Mandatory)][ValidatePattern('^\d+\.\d+\.\d+$')][string]$Version,
    [Parameter(Mandatory)][string]$WorkDir
)
$ErrorActionPreference = 'Stop'
New-Item -ItemType Directory -Force -Path $WorkDir | Out-Null
$sdkBin = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits/10/bin'
$rc = Get-ChildItem -LiteralPath $sdkBin -Filter rc.exe -Recurse -ErrorAction Stop |
    Where-Object { $_.Directory.Name -eq 'x64' } | Sort-Object FullName -Descending | Select-Object -First 1
if (-not $rc) { throw 'Windows SDK resource compiler (x64) is required.' }
$tuple = ($Version -replace '\.', ',') + ',0'
$rcPath = Join-Path $WorkDir 'version.rc'
$resPath = Join-Path $WorkDir 'version.res'
$text = @"
1 VERSIONINFO
 FILEVERSION $tuple
 PRODUCTVERSION $tuple
 FILEFLAGSMASK 0x3fL
 FILEFLAGS 0x0L
 FILEOS 0x40004L
 FILETYPE 0x1L
 FILESUBTYPE 0x0L
BEGIN
  BLOCK "StringFileInfo"
  BEGIN
    BLOCK "040904B0"
    BEGIN
      VALUE "FileVersion", "$Version.0"
      VALUE "ProductVersion", "$Version.0"
      VALUE "ProductName", "proxysss"
    END
  END
  BLOCK "VarFileInfo"
  BEGIN
    VALUE "Translation", 0x0409, 1200
  END
END
"@
[IO.File]::WriteAllText($rcPath, $text, [Text.Encoding]::ASCII)
& $rc.FullName /nologo /fo $resPath $rcPath
if ($LASTEXITCODE -ne 0) { throw 'Version resource compilation failed.' }
$data = [IO.File]::ReadAllBytes($resPath)
$payload = $null
for ($offset = 0; $offset + 16 -le $data.Length;) {
    $size = [BitConverter]::ToUInt32($data, $offset)
    $headerSize = [BitConverter]::ToUInt32($data, $offset + 4)
    if ($headerSize -lt 16 -or ($offset + $headerSize + $size) -gt $data.Length) { throw 'Invalid compiled resource.' }
    if ([BitConverter]::ToUInt16($data, $offset + 8) -eq 0xffff -and [BitConverter]::ToUInt16($data, $offset + 10) -eq 16) {
        $payload = [byte[]]::new($size)
        [Array]::Copy($data, $offset + $headerSize, $payload, 0, $size)
        break
    }
    $offset = [int](($offset + $headerSize + $size + 3) -band (-bnot 3))
}
if (-not $payload) { throw 'Compiled version resource was not found.' }
if (-not ('ProxysssResourceEditor' -as [type])) {
    Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class ProxysssResourceEditor {
    [DllImport("kernel32.dll", CharSet=CharSet.Unicode, SetLastError=true)] public static extern IntPtr BeginUpdateResource(string file, bool deleteExisting);
    [DllImport("kernel32.dll", CharSet=CharSet.Unicode, SetLastError=true)] public static extern bool UpdateResource(IntPtr handle, IntPtr type, IntPtr name, ushort language, byte[] data, uint size);
    [DllImport("kernel32.dll", CharSet=CharSet.Unicode, SetLastError=true)] public static extern bool EndUpdateResource(IntPtr handle, bool discard);
}
'@
}
$handle = [ProxysssResourceEditor]::BeginUpdateResource($Binary, $false)
if ($handle -eq [IntPtr]::Zero) { throw 'Cannot open binary resources.' }
$ok = [ProxysssResourceEditor]::UpdateResource($handle, [IntPtr]16, [IntPtr]1, 0x0409, $payload, $payload.Length)
$committed = [ProxysssResourceEditor]::EndUpdateResource($handle, -not $ok)
if (-not $ok -or -not $committed) { throw 'Cannot write binary version resource.' }
$actual = (Get-Item -LiteralPath $Binary).VersionInfo.FileVersion
if ($actual -ne "$Version.0") { throw "Version resource mismatch: $actual" }
