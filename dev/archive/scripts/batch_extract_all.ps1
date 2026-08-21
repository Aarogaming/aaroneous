$rootDir = "C:\Users\aarog\OneDrive - St. Clair County Community College\Documents\College"
$url = "http://localhost:1234/v1/chat/completions"
$logFile = Join-Path $rootDir "extraction_log.txt"

function Write-Log($message) {
    $timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
    $logMessage = "[$timestamp] $message"
    Write-Output $logMessage
    Add-Content -Path $logFile -Value $logMessage
}

Write-Log "Starting batch extraction across all folders in $rootDir"

$files = Get-ChildItem -Path $rootDir -Filter "*.jpg" -Recurse

Write-Log "Found $($files.Count) JPG files total."

$count = 1
foreach ($file in $files) {
    $outFile = Join-Path $file.DirectoryName ($file.BaseName + "_extracted.txt")
    
    if (Test-Path $outFile) {
        Write-Log "[$count/$($files.Count)] Skipping $($file.Name) - already extracted."
        $count++
        continue
    }
    
    Write-Log "[$count/$($files.Count)] Extracting $($file.Name) in folder $($file.Directory.Name)..."
    
    try {
        $bytes = [System.IO.File]::ReadAllBytes($file.FullName)
        $base64 = [System.Convert]::ToBase64String($bytes)
        
        $payload = @{
            model = "local-model"
            messages = @(
                @{
                    role = "user"
                    content = @(
                        @{
                            type = "text"
                            text = "You are a technical document extractor. Extract all mathematical formulas, equations, circuit details, circuit values, and theoretical concepts from this whiteboard image. Format formulas cleanly using markdown math notation where appropriate. Do NOT describe the image visually (e.g. 'The image shows...'). Just output the raw technical information, rules, concepts, and formulas."
                        },
                        @{
                            type = "image_url"
                            image_url = @{
                                url = "data:image/jpeg;base64,$base64"
                            }
                        }
                    )
                }
            )
            temperature = 0.1
            max_tokens = 2000
        }
        
        $jsonPayload = $payload | ConvertTo-Json -Depth 10
        
        $response = Invoke-RestMethod -Uri $url -Method Post -Body $jsonPayload -ContentType "application/json" -TimeoutSec 300
        $extractedText = $response.choices[0].message.content
        
        [System.IO.File]::WriteAllText($outFile, $extractedText)
        Write-Log "  -> Saved to $($file.BaseName)_extracted.txt"
    } catch {
        Write-Log "  -> Error processing $($file.Name): $_"
    }
    $count++
}

Write-Log "Batch extraction complete! All images have been processed."
