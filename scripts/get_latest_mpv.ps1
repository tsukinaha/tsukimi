function Get-Latest-Mpv() {
    $api_gh = "https://api.github.com/repos/shinchiro/mpv-winbuild-cmake/releases/latest"
    $json = Invoke-WebRequest $api_gh -MaximumRedirection 0 -ErrorAction Ignore -UseBasicParsing | ConvertFrom-Json
    $filename = $json.assets | where { $_.name -Match "mpv-dev-x86_64-v3" } | Select-Object -ExpandProperty name
    $download_link = $json.assets | where { $_.name -Match "mpv-dev-x86_64-v3" } | Select-Object -ExpandProperty browser_download_url

    Write-Host "Downloading" $filename -ForegroundColor Green
    Invoke-WebRequest -Uri $download_link -UserAgent ([Microsoft.PowerShell.Commands.PSUserAgent]::FireFox) -OutFile $filename

    7z.exe x $filename
}