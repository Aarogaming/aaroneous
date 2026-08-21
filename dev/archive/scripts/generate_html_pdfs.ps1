$files = @(
    "C:\Users\aarog\OneDrive - St. Clair County Community College\Documents\College\ETE 120\ETE120_Study_Guide_3Q.md",
    "C:\Users\aarog\OneDrive - St. Clair County Community College\Documents\College\ETE 120\ETE120_Formula_Sheet_3Q.md",
    "C:\Users\aarog\OneDrive - St. Clair County Community College\Documents\College\ETM 110\ETM110_Study_Guide_3Q.md",
    "C:\Users\aarog\OneDrive - St. Clair County Community College\Documents\College\ETM 110\ETM110_Formula_Sheet_3Q.md"
)

$htmlTemplate1 = @"
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>Study Material</title>
    <script src="https://cdn.jsdelivr.net/npm/marked/marked.min.js"></script>
    <script>
        MathJax = {
            tex: {
                inlineMath: [['`$', '`$'], ['\\(', '\\)']],
                displayMath: [['`$`$', '`$`$'], ['\\[', '\\]']],
                processEscapes: true
            }
        };
    </script>
    <script src="https://cdn.jsdelivr.net/npm/mathjax@3/es5/tex-mml-chtml.js"></script>
    <style>
        body { font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif; max-width: 900px; margin: 0 auto; padding: 40px 20px; line-height: 1.5; color: #000; }
        h1, h2, h3 { color: #111; border-bottom: 2px solid #eee; padding-bottom: 5px; margin-top: 30px; }
        h1 { text-align: center; }
        code { background: #f8f9fa; padding: 2px 4px; border-radius: 3px; font-family: Consolas, monospace; }
        pre { background: #f8f9fa; padding: 15px; border-radius: 5px; overflow-x: auto; border: 1px solid #ddd; }
        ul, ol { margin-bottom: 20px; }
        li { margin-bottom: 8px; }
        .math { font-size: 1.1em; }
        @media print {
            body { max-width: 100%; margin: 0; padding: 10mm; font-size: 12pt; }
            h1, h2, h3 { page-break-after: avoid; }
            ul, ol, p { page-break-inside: avoid; }
        }
    </style>
</head>
<body>
    <div id="content"></div>
    <textarea id="markdown-source" style="display:none;">
"@

$htmlTemplate2 = @"
    </textarea>
    <script>
        // Parse markdown to HTML
        document.getElementById('content').innerHTML = marked.parse(document.getElementById('markdown-source').value);
        // Tell MathJax to render the math equations
        MathJax.typesetPromise();
    </script>
</body>
</html>
"@

foreach ($file in $files) {
    if (Test-Path $file) {
        $content = Get-Content $file -Raw
        $htmlFile = $file.Replace(".md", ".html")
        
        # Replace template placeholders to prevent powershell variable expansion issues in the template
        $template1 = $htmlTemplate1.Replace('`$', '$')

        $finalHtml = $template1 + "`n" + $content + "`n" + $htmlTemplate2
        Set-Content -Path $htmlFile -Value $finalHtml -Encoding UTF8
        Write-Host "Generated: $htmlFile"
    } else {
        Write-Host "File not found: $file"
    }
}
