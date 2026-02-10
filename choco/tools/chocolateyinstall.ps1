$ErrorActionPreference = 'Stop'

$packageName = 'bntui'
$version = '0.1.3'
$url64 = "https://github.com/obselate/bntui/releases/download/v${version}/bntui-windows-x86_64.exe"

$toolsDir = "$(Split-Path -Parent $MyInvocation.MyCommand.Definition)"
$exePath = Join-Path $toolsDir 'bntui.exe'

Get-ChocolateyWebFile -PackageName $packageName `
  -FileFullPath $exePath `
  -Url64bit $url64 `
  -Checksum64 'd69febbd522aa07ffde277a287fb15b04444ac7faf510eaf37319a2bde376871' `
  -ChecksumType64 'sha256'
