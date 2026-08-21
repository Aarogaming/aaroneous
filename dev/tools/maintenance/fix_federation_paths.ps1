# fix_federation_paths.ps1

$files = Get-ChildItem -Path "d:\Aaroneous\core\hypervisor\src" -Include "*.rs" -Recurse
foreach ($f in $files) {
    $content = [System.IO.File]::ReadAllText($f.FullName)
    $orig = $content
    $content = $content.Replace("Arccrate::hive", "Archive")
    $content = $content.Replace("arccrate::hive", "archive")
    $content = $content.Replace("crate::federation::crate::hive", "crate::federation::hive")
    $content = $content.Replace("crate::federation::multi_crate::hive", "crate::federation::multi_hive")
    $content = $content.Replace("federation::crate::hive", "federation::hive")
    $content = $content.Replace("federation::multi_crate::hive", "federation::multi_hive")
    $content = $content.Replace("multi_crate::hive", "multi_hive")
    $content = $content.Replace("self_crate::digestion", "self_digestion")
    $content = $content.Replace("crate::digestion::self_digestion::", "crate::digestion::")
    $content = $content.Replace("crate::crate::", "crate::")
    if ($content -ne $orig) {
        [System.IO.File]::WriteAllText($f.FullName, $content)
        Write-Host "Fixed: $($f.Name)" -ForegroundColor Green
    }
}
