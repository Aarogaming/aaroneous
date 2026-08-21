# fix_hypervisor_crate_imports.ps1

$files = Get-ChildItem -Path "d:\Aaroneous\core\hypervisor" -Include "*.rs" -Recurse
$replacements = @{
    "use digestion::" = "use crate::digestion::"
    "use agents::" = "use crate::agents::"
    "use control::" = "use crate::control::"
    "use hive::" = "use crate::hive::"
    "use intelligence::" = "use crate::intelligence::"
    "use sabs::" = "use crate::sabs::"
    "use constellation::" = "use crate::constellation::"
    "use skills::" = "use crate::skills::"
    "use genetics::" = "use crate::genetics::"
    "use scientific_analyzer::" = "use crate::scientific_analyzer::"
    "digestion::self_digestion::" = "crate::digestion::"
    "digestion::" = "crate::digestion::"
    "agents::" = "crate::agents::"
    "control::" = "crate::control::"
    "hive::" = "crate::hive::"
    "intelligence::" = "crate::intelligence::"
    "sabs::" = "crate::sabs::"
    "constellation::" = "crate::constellation::"
    "skills::" = "crate::skills::"
    "genetics::" = "crate::genetics::"
    "scientific_analyzer::" = "crate::scientific_analyzer::"
}

foreach ($f in $files) {
    $content = [System.IO.File]::ReadAllText($f.FullName)
    $orig = $content
    foreach ($k in $replacements.Keys) {
        $content = $content.Replace($k, $replacements[$k])
    }
    $content = $content.Replace("crate::federation::crate::hive", "crate::federation::hive")
    $content = $content.Replace("crate::federation::multi_crate::hive", "crate::federation::multi_hive")
    $content = $content.Replace("multi_crate::hive", "multi_hive")
    $content = $content.Replace("self_crate::digestion", "self_digestion")
    $content = $content.Replace("crate::digestion::self_digestion::", "crate::digestion::")
    $content = $content.Replace("crate::crate::", "crate::")
    if ($content -ne $orig) {
        [System.IO.File]::WriteAllText($f.FullName, $content)
    }
}

Write-Host "Updated hypervisor imports to crate:: bridges" -ForegroundColor Green
