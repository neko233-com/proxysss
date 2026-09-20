#requires -Version 7.2
param()

$ErrorActionPreference = 'Stop'
$repository = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$utf8 = [Text.UTF8Encoding]::new($false)

function Get-ContentHash([string]$Value) {
    $normalized = $Value.Replace("`r`n", "`n").Trim()
    $bytes = [Security.Cryptography.SHA256]::HashData([Text.Encoding]::UTF8.GetBytes($normalized))
    return [Convert]::ToHexString($bytes).ToLowerInvariant()
}

$links = @{
    'docs/CONFIGURATION.md' = 'configuration.html'
    'docs/CDN-ORIGIN.md' = 'cdn-origin.html'
    'docs/ARCHITECTURE.md' = 'architecture.html'
    'docs/SECURITY-PERFORMANCE.md' = 'security-performance.html'
    'docs/VALIDATION.md' = 'validation.html'
    'ts-how-to-use.md' = 'ts-how-to-use.html'
}

foreach ($name in @('nginx-to-proxysss', 'caddy-to-proxysss')) {
    $sourcePath = Join-Path $repository "$name.md"
    $targetPath = Join-Path $repository "docs/$name.html"
    $source = [IO.File]::ReadAllText($sourcePath).Replace("`r`n", "`n")
    $markdown = [regex]::Replace($source, '\A# [^\n]+\n', '')
    foreach ($link in $links.GetEnumerator()) {
        $markdown = $markdown.Replace("]($($link.Key))", "]($($link.Value))")
    }
    $sections = [regex]::Split($markdown.Trim(), '(?m)(?=^## )')
    $ids = @('intro', 'mapping', 'beginner', 'advanced', 'validation', 'read-next')
    if ($sections.Count -ne $ids.Count) {
        throw "$name must have an introduction and five ordered tutorial sections"
    }
    $parts = for ($index = 0; $index -lt $sections.Count; $index++) {
        $html = (ConvertFrom-Markdown -InputObject $sections[$index]).Html.Trim()
        "<section class=`"panel`" id=`"$($ids[$index])`">`n$html`n</section>"
    }
    $body = $parts -join "`n"
    $sourceHash = Get-ContentHash $source
    $bodyHash = Get-ContentHash $body
    $fragment = "<!-- migration-source: $name.md sha256=$sourceHash body-sha256=$bodyHash -->`n$body`n<!-- /migration-source -->"
    $page = [IO.File]::ReadAllText($targetPath).Replace("`r`n", "`n")
    $main = [regex]::Match($page, '(?s)(<main class="content">).*?(</main>)')
    if (-not $main.Success) { throw "Missing content main in $targetPath" }
    $replacement = $main.Groups[1].Value + "`n$fragment`n" + $main.Groups[2].Value
    $page = $page.Substring(0, $main.Index) + $replacement + $page.Substring($main.Index + $main.Length)
    [IO.File]::WriteAllText($targetPath, $page, $utf8)
    Write-Output "Rendered docs/$name.html from $name.md"
}
