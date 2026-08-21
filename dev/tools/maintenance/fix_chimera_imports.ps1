# fix_chimera_imports.ps1

$files = Get-ChildItem -Path "d:\Aaroneous\crates\chimera\src\analysis\*.rs"
foreach ($f in $files) {
    $content = [System.IO.File]::ReadAllText($f.FullName)
    $content = $content.Replace("crate::ast_parser::", "crate::analysis::ast_parser::")
    $content = $content.Replace("use crate::ast_parser::", "use crate::analysis::ast_parser::")
    $content = $content.Replace("use crate::ast_parser;", "use crate::analysis::ast_parser;")
    $content = $content.Replace("use crate::batch_tensor::", "use crate::analysis::batch_tensor::")
    $content = $content.Replace("use crate::experiment::", "use crate::analysis::experiment::")
    $content = $content.Replace("use crate::hypothesis::", "use crate::analysis::hypothesis::")
    $content = $content.Replace("use crate::verifier::", "use crate::analysis::verifier::")
    $content = $content.Replace("use crate::pipeline::", "use crate::analysis::pipeline::")
    $content = $content.Replace("crate::ast_parser::", "crate::analysis::ast_parser::")
    [System.IO.File]::WriteAllText($f.FullName, $content)
}

Write-Host "Fixed analysis imports in chimera" -ForegroundColor Green
