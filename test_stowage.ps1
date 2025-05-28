$ErrorActionPreference = 'Stop'
Write-Host "Starting Stowage server..."
$server = Start-Process cargo -ArgumentList 'run', '--release' -NoNewWindow -PassThru
$maxAttempts = 20
$attempt = 0
while ($attempt -lt $maxAttempts) {
    Start-Sleep -Seconds 1
    try {
        $resp = Invoke-WebRequest -Uri http://localhost:8080/about -UseBasicParsing -ErrorAction Stop
        if ($resp.StatusCode -eq 200) {
            Write-Host "Server is up!"
            break
        }
    } catch {}
    $attempt++
}
if ($attempt -eq $maxAttempts) {
    Write-Host "ERROR: Server did not start in time."
    Stop-Process -Id $server.Id
    exit 1
}

$files = @(
    'example.json',
    'example.mp3',
    'example.png',
    'example.xml',
    'example.exe',
    'disguised.mp3',
    'disguised.png',
    'disguised.xml'
)

$shouldSucceed = @{
    'example.json' = $true
    'example.mp3' = $true
    'example.png' = $true
    'example.xml' = $true
    'example.exe' = $false
    'disguised.mp3' = $false
    'disguised.png' = $false
    'disguised.xml' = $false
}

$datadir = Join-Path $PSScriptRoot '.\.data'

foreach ($file in $files) {
    $path = Join-Path $datadir $file
    Write-Host "Uploading $file..."
    $resp = curl.exe -s -w "`n%{http_code}" -F "file=@$path" http://localhost:8080/upload
    $lines = $resp -split "`n"
    $body = $lines[0]
    $status = $lines[1]
    if ($shouldSucceed[$file]) {
        if ($status -eq '201') {
            Write-Host "  Success: $file uploaded."
            $json = $body | ConvertFrom-Json
            $file_id = $json.file_id
            write-host "  File ID: $file_id"
            $download_url = "http://localhost:8080/files/$file_id"
            $tempDownload = [System.IO.Path]::GetTempFileName()
            $download_status = curl.exe -s -w "%{http_code}" -o $tempDownload $download_url
            if ($download_status -eq '200') {
                Write-Host "  Download succeeded for $file. Comparing contents..."
                $origHash = (Get-FileHash -Algorithm SHA256 $path).Hash
                $downHash = (Get-FileHash -Algorithm SHA256 $tempDownload).Hash
                if ($origHash -eq $downHash) {
                    Write-Host "  File contents match."
                } else {
                    Write-Host "  ERROR: File contents do not match for $file."
                }
            } else {
                Write-Host "  ERROR: Download failed for $file (status $download_status)"
            }
            Remove-Item $tempDownload -ErrorAction SilentlyContinue
        } else {
            Write-Host "  ERROR: $file shoud upload (got status $status)"
        }
    } else {
        if ($status -eq '400') {
            Write-Host "  Correctly rejected $file."
        } else {
            Write-Host "  ERROR: $file should be rejected (got status $status)"
        }
    }
}
Write-Host "\nTesting /about endpoint..."
$about = Invoke-WebRequest -Uri http://localhost:8080/about -UseBasicParsing
Write-Host $about.Content

# Test download endpoint
Write-Host "\nTesting /download endpoint..."
$downloadUrl = "http://kneecap.2wu.me/media/transcripts/052ef596-51d5-4133-bc27-8f1a84bd179b.m4a.json"
$body = @{
    download_url = $downloadUrl
} | ConvertTo-Json

Write-Host "  Creating download job for $downloadUrl..."
$resp = Invoke-WebRequest -Uri "http://localhost:8080/download" -Method Post -Body $body -ContentType "application/json" -UseBasicParsing
if ($resp.StatusCode -ne 202) {
    Write-Host "  ERROR: Failed to create download job (status $($resp.StatusCode))"
} else {
    $job = $resp.Content | ConvertFrom-Json
    $jobId = $job.job_id
    $statusUrl = $job.status_url
    Write-Host "  Job created with ID: $jobId"
    
    # Poll the status until completion or timeout
    $maxPolls = 60  # 60 seconds max
    $pollCount = 0
    $completed = $false
    
    while ($pollCount -lt $maxPolls -and -not $completed) {
        $pollCount++
        Start-Sleep -Seconds 1
        
        try {
            $statusResp = Invoke-WebRequest -Uri $statusUrl -UseBasicParsing -ErrorAction Stop
            $status = $statusResp.Content | ConvertFrom-Json
            
            Write-Host "  Poll $pollCount - Status: $($status.status)"
            
            if ($status.status -eq "Completed") {
                $completed = $true
                Write-Host "  Download completed successfully!"
                
                # Download the file
                if ($status.file_id) {
                    $fileId = $status.file_id
                    $downloadUrl = "http://localhost:8080/files/$fileId"
                    $outputFile = "downloaded_file_$(Get-Date -Format 'yyyyMMddHHmmss').json"
                    
                    Write-Host "  Downloading file to $outputFile..."
                    try {
                        Invoke-WebRequest -Uri $downloadUrl -OutFile $outputFile -UseBasicParsing -ErrorAction Stop
                        Write-Host "  File downloaded successfully to $outputFile"
                        Write-Host "  File size: $((Get-Item $outputFile).Length) bytes"
                    } catch {
                        Write-Host "  ERROR: Failed to download file: $_"
                    }
                }
            } elseif ($status.status -eq "Failed") {
                Write-Host "  ERROR: Download failed: $($status.error)"
                break
            }
        } catch {
            Write-Host "  ERROR: Failed to get job status: $_"
            break
        }
    }
    
    if (-not $completed) {
        Write-Host "  WARNING: Download did not complete within $maxPolls seconds"
    }
}

Write-Host "\nStopping Stowage server..."
# Stop-Process -Id $server.Id
