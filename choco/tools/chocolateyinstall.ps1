$ErrorActionPreference = 'Stop'

$packageName = 'bntui'
$version = '0.1.1'
$url = "https://github.com/obselate/bntui/releases/download/v${version}/bntui-windows-x86_64.exe"

$toolsDir = "$(Split-Path -Parent $MyInvocation.MyCommand.Definition)"
$exePath = Join-Path $toolsDir 'bntui.exe'

Get-ChocolateyWebFile -PackageName $packageName `
  -FileFullPath $exePath `
  -Url64bit $url `
  -Checksum64 '' `
  -ChecksumType64 'sha256'
