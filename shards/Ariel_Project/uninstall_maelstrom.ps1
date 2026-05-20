
$ErrorActionPreference = 'SilentlyContinue'
Remove-Item -Path 'C:\Users\aarog\Desktop\Project Maelstrom.lnk' -ErrorAction SilentlyContinue
Remove-Item -Path 'C:\Users\aarog\AppData\Roaming\Microsoft\Windows\Start Menu\Programs\Project Maelstrom\Project Maelstrom.lnk' -ErrorAction SilentlyContinue
Remove-Item -Path 'C:\Users\aarog\AppData\Roaming\Microsoft\Windows\Start Menu\Programs\Project Maelstrom\Uninstall Project Maelstrom.lnk' -ErrorAction SilentlyContinue
if (Test-Path 'C:\Users\aarog\AppData\Local\ProjectMaelstrom') {
    Remove-Item -Recurse -Force 'C:\Users\aarog\AppData\Local\ProjectMaelstrom'
}
Write-Host 'Project Maelstrom removed.'
