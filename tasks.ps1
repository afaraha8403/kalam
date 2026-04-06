# ==============================================================================
# Kalam Tasks Helper
# ==============================================================================
#
# A PowerShell task runner for common development operations.
#
# USAGE:
#   ./tasks.ps1 <command> [args]
#
# EXAMPLES:
#   ./tasks.ps1 help              # Show all available commands
#   ./tasks.ps1 dev               # Run in development mode
#   ./tasks.ps1 build             # Build the app (Tauri)
#   ./tasks.ps1 test              # Run all tests
#   ./tasks.ps1 release 1.0.0     # Create a stable release
#
# ==============================================================================

param (
    [Parameter(Position=0)]
    [string]$Command = "help",

    [Parameter(Position=1)]
    [string]$Arg1 = "",

    [Parameter(Position=2)]
    [string]$Arg2 = ""
)

$Version = $Arg1

# Ensure UTF-8 output for better visibility of icons
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

$RootDir = Get-Location

# Rust/Cargo default install location is often missing from the interactive PowerShell PATH; align with package.json scripts.
function Ensure-CargoInPath {
    if ($env:OS -eq "Windows_NT") {
        $cargoBin = Join-Path $env:USERPROFILE ".cargo\bin"
        if (Test-Path $cargoBin) {
            $env:PATH = "$cargoBin;$env:PATH"
        }
    }
}

# Tauri needs cargo on PATH; fail fast with a clear message (avoids opaque "program not found" from npx).
function Assert-CargoAvailable {
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        Write-Host ""
        Write-Host "cargo was not found on PATH." -ForegroundColor Red
        Write-Host "Install Rust: https://rustup.rs/  Then open a new terminal and run this task again." -ForegroundColor Yellow
        Write-Host ""
        exit 1
    }
}

# ==============================================================================
# HELPER FUNCTIONS
# ==============================================================================

# ------------------------------------------------------------------------------
# Update-Version
# ------------------------------------------------------------------------------
# Updates the version number in all configuration files that need to stay in sync.
#
# FILES UPDATED:
#   - src-tauri/tauri.conf.json  (Tauri app version)
#   - package.json               (npm package version)
#   - src-tauri/Cargo.toml       (Rust crate version)
# ------------------------------------------------------------------------------
function Update-Version {
    param([string]$NewVersion)
    
    Write-Host "Updating version to $NewVersion..." -ForegroundColor Cyan
    
    # Update tauri.conf.json
    $tauriConf = Get-Content "src-tauri/tauri.conf.json" -Raw | ConvertFrom-Json
    $tauriConf.version = $NewVersion
    $tauriConf | ConvertTo-Json -Depth 10 | Set-Content "src-tauri/tauri.conf.json"
    
    # Update package.json
    $packageJson = Get-Content "package.json" -Raw | ConvertFrom-Json
    $packageJson.version = $NewVersion
    $packageJson | ConvertTo-Json -Depth 10 | Set-Content "package.json"
    
    # Update Cargo.toml (simple regex replace for the package version only)
    $cargoToml = Get-Content "src-tauri/Cargo.toml" -Raw
    $cargoToml = $cargoToml -replace '(^\[package\][\s\S]*?version = ")[^"]*(")', "`${1}$NewVersion`${2}"
    Set-Content "src-tauri/Cargo.toml" $cargoToml -NoNewline
    
    Write-Host "Version updated to $NewVersion" -ForegroundColor Green
}

# ------------------------------------------------------------------------------
# Confirm-ChangelogUpdated
# ------------------------------------------------------------------------------
# Checks if CHANGELOG.md contains a section for the target version.
# Prompts the user to confirm/update the changelog before releasing.
# The GitHub release workflow extracts release notes from this section.
# ------------------------------------------------------------------------------
function Confirm-ChangelogUpdated {
    param([string]$Version)
    
    # Check if CHANGELOG.md exists and has the version section
    $changelogPath = "CHANGELOG.md"
    $hasSection = $false
    $hasGroupedFormat = $false
    
    if (Test-Path $changelogPath) {
        $content = Get-Content $changelogPath -Raw
        # Check for section with or without 'v' prefix (e.g., [0.1.0-beta.1] or [v0.1.0-beta.1])
        $sectionPattern = "## \[$Version\]"
        $sectionPatternWithV = "## \[v$Version\]"
        $hasSection = $content -match $sectionPattern -or $content -match $sectionPatternWithV
        
        # Check if the section uses the new grouped format (Features/Fixes/Changes)
        if ($hasSection) {
            # Find the section and check for grouped subsections
            $sectionMatch = [regex]::Match($content, "(?s)$sectionPattern.*?((?=## \[)|$)")
            if (-not $sectionMatch.Success) {
                $sectionMatch = [regex]::Match($content, "(?s)$sectionPatternWithV.*?((?=## \[)|$)")
            }
            if ($sectionMatch.Success) {
                $sectionContent = $sectionMatch.Value
                $hasGroupedFormat = $sectionContent -match "### Features" -or 
                                    $sectionContent -match "### Fixes" -or 
                                    $sectionContent -match "### Changes"
            }
        }
    }
    
    if (-not $hasSection) {
        Write-Host ""
        Write-Host ("  [!] Warning: CHANGELOG.md does not contain section ## [{0}]" -f $Version) -ForegroundColor Yellow
        Write-Host ""
        Write-Host "  How to update CHANGELOG.md before releasing:" -ForegroundColor Cyan
        Write-Host "  -----------------------------------------------" -ForegroundColor Cyan
        Write-Host ""
        Write-Host "  1. Open CHANGELOG.md" -ForegroundColor White
        Write-Host ""
        Write-Host "  2. Under ## [Unreleased], add a new version section:" -ForegroundColor White
        Write-Host "     ## [$Version]" -ForegroundColor Yellow
        Write-Host ""
        Write-Host "  3. Organize entries into three subsections:" -ForegroundColor White
        Write-Host "     ### Features" -ForegroundColor Yellow
        Write-Host "     - Description of new feature..." -ForegroundColor Gray
        Write-Host "     - Another feature..." -ForegroundColor Gray
        Write-Host ""
        Write-Host "     ### Fixes" -ForegroundColor Yellow
        Write-Host "     - Bug fix description..." -ForegroundColor Gray
        Write-Host "     - Another fix..." -ForegroundColor Gray
        Write-Host ""
        Write-Host "     ### Changes" -ForegroundColor Yellow
        Write-Host "     - Refactor or UX change..." -ForegroundColor Gray
        Write-Host "     - Another change..." -ForegroundColor Gray
        Write-Host ""
        Write-Host "  4. Save and commit the changelog changes" -ForegroundColor White
        Write-Host ""
        Write-Host "  Note: The GitHub release workflow extracts release notes" -ForegroundColor Gray
        Write-Host "  from this section to populate the release description." -ForegroundColor Gray
        Write-Host ""
        
        $response = Read-Host ("  Have you updated CHANGELOG.md for v{0}? (y/N)" -f $Version)
        if ($response -ne "y" -and $response -ne "Y") {
            Write-Host ""
            Write-Host "  [X] Release aborted. Update CHANGELOG.md and run again." -ForegroundColor Red
            Write-Host ""
            exit 1
        }
    } else {
        Write-Host ""
        Write-Host ("  [OK] Found changelog section [{0}]" -f $Version) -ForegroundColor Green
        if (-not $hasGroupedFormat) {
            # Single-quoted: avoids PS 5.1 misparsing (e.g., commas in "(e.g., ...)" or "###" near "#" comments when encoding is wrong).
            Write-Host '  [!] Warning: Section does not use grouped format (### Features / Fixes / Changes)' -ForegroundColor Yellow
        }
        Write-Host ""
    }
}

# ------------------------------------------------------------------------------
# Show-Help
# ------------------------------------------------------------------------------
function Show-Help {
    Write-Host ""
    Write-Host "  Kalam Task Runner" -ForegroundColor Green
    Write-Host "  =================" -ForegroundColor Green
    Write-Host ""
    Write-Host "  Usage: ./tasks.ps1 <command> [args]"
    Write-Host ""
  Write-Host "  BUILD & RUN" -ForegroundColor Yellow
  Write-Host "  -----------"
  Write-Host "    dev               - Run in development mode (Tauri)"
  Write-Host "    build             - Build the Tauri app (unsigned; no signing key needed)"
  Write-Host "    build-signed      - Build with updater signing (requires TAURI_SIGNING_PRIVATE_KEY)"
  Write-Host "    deps              - Install npm dependencies"
    Write-Host ""
    Write-Host "  TESTING & QUALITY" -ForegroundColor Yellow
    Write-Host "  -----------------"
    Write-Host "    test              - Run Rust tests and Svelte checks"
    Write-Host "    fmt               - Format Rust code"
    Write-Host "    lint              - Lint Rust code (clippy)"
    Write-Host "    ci                - Run standard CI checks (fmt, lint, test, build)"
    Write-Host ""
    Write-Host "  RELEASE" -ForegroundColor Yellow
    Write-Host "  -------"
    Write-Host "    release [version]      - Create a stable release (v1.0.0)"
    Write-Host "    release-beta [version] - Create a beta/RC release (e.g. v1.0.0-beta.1 or v1.0.0-rc.1)"
    Write-Host "    release-rc [version]   - Create a release candidate (e.g. v1.0.0-rc.1)"
    Write-Host "    set-version <version>  - Update version only (no commit/tag)"
    Write-Host ""
    Write-Host "  SIGNING KEYS (for auto-updater)" -ForegroundColor Yellow
    Write-Host "  --------------------------------"
    Write-Host "    generate-keys            - Generate Tauri signing keys for updates"
    Write-Host "    show-pubkey              - Show public key + base64 for tauri.conf.json"
    Write-Host "    verify-updater-signing   - Confirm ~/.tauri/kalam.key.pub matches tauri.conf pubkey"
    Write-Host ""
}

# ==============================================================================
# COMMAND ROUTER
# ==============================================================================

switch ($Command) {
    # ==========================================================================
    # BUILD & RUN COMMANDS
    # ==========================================================================
    
    "dev" {
        Write-Host "Starting development server..." -ForegroundColor Cyan
        # Use npm "dev" script so PATH, dev-bridge feature, and Vite match package.json (not plain `tauri dev`).
        npm run dev
    }

    "build" {
        Write-Host "Building Tauri app (unsigned)..." -ForegroundColor Cyan
        Ensure-CargoInPath
        Assert-CargoAvailable
        # Windows: use NSIS only to avoid MSI prerelease version error (aligns with release.yml)
        if ($env:OS -eq "Windows_NT") {
            npx tauri build --no-sign --bundles nsis
        } else {
            npm run tauri:build:unsigned
        }
    }

    "build-signed" {
        Write-Host "Building Tauri app (signed; requires TAURI_SIGNING_PRIVATE_KEY)..." -ForegroundColor Cyan
        Ensure-CargoInPath
        Assert-CargoAvailable
        # Windows: use NSIS only to avoid MSI prerelease version error (aligns with release.yml)
        if ($env:OS -eq "Windows_NT") {
            npx tauri build --bundles nsis
        } else {
            npx tauri build
        }
    }

    "deps" {
        Write-Host "Installing dependencies..." -ForegroundColor Cyan
        npm install
    }

    # ==========================================================================
    # TESTING & QUALITY COMMANDS
    # ==========================================================================

    "test" {
        Write-Host "--- Running Tests ---" -ForegroundColor Cyan
        Ensure-CargoInPath
        Assert-CargoAvailable

        Write-Host "`n[1/3] Checking Svelte/TypeScript..." -ForegroundColor Gray
        npm run check
        $tsResult = $LASTEXITCODE

        Write-Host "`n[2/3] Running frontend unit tests (Vitest)..." -ForegroundColor Gray
        npm run test:unit
        $unitResult = $LASTEXITCODE

        Write-Host "`n[3/3] Running Rust tests..." -ForegroundColor Gray
        Set-Location src-tauri
        cargo test
        $rustResult = $LASTEXITCODE
        Set-Location $RootDir

        if ($tsResult -eq 0 -and $unitResult -eq 0 -and $rustResult -eq 0) {
            Write-Host "`n[OK] SUCCESS: All checks passed." -ForegroundColor Green
        } else {
            Write-Host "`n[X] FAILURE: Some checks failed." -ForegroundColor Red
            exit 1
        }
    }

    "fmt" {
        Write-Host "Formatting Rust code..." -ForegroundColor Cyan
        Ensure-CargoInPath
        Assert-CargoAvailable
        Set-Location src-tauri
        cargo fmt
        Set-Location $RootDir
    }

    "lint" {
        Write-Host "Linting Rust code..." -ForegroundColor Cyan
        Ensure-CargoInPath
        Assert-CargoAvailable
        Set-Location src-tauri
        cargo clippy -- -D warnings
        Set-Location $RootDir
    }

    "ci" {
        Write-Host "Running CI checks..." -ForegroundColor Cyan
        & ./tasks.ps1 fmt
        & ./tasks.ps1 lint
        & ./tasks.ps1 test
        & ./tasks.ps1 build
        Write-Host "CI checks passed!" -ForegroundColor Green
    }

    # ==========================================================================
    # RELEASE COMMANDS
    # ==========================================================================

    "release" {
        if (-not $Version) {
            $Version = Read-Host 'Enter version (e.g. 1.0.0)'
        }
        
        if (-not ($Version -match '^\d+\.\d+\.\d+$')) {
            Write-Host 'Error: Invalid version format. Use semantic versioning (e.g. 1.0.0)' -ForegroundColor Red
            exit 1
        }
        
        Confirm-ChangelogUpdated $Version
        Update-Version $Version
        
        git add src-tauri/tauri.conf.json package.json src-tauri/Cargo.toml
        git commit -m "chore: bump version to $Version"
        # Push version bump first so the tag points to a commit that exists on remote; only then push tag (which triggers the release workflow).
        git push origin main
        if ($LASTEXITCODE -ne 0) {
            Write-Host "Error: Push to main failed. Fix and push manually, then run: git tag -a 'v$Version' -m 'Release v$Version'; git push origin 'v$Version'" -ForegroundColor Red
            exit 1
        }
        
        git tag -a "v$Version" -m "Release v$Version"
        git push origin "v$Version"
        if ($LASTEXITCODE -ne 0) { exit 1 }
        
        Write-Host ""
        Write-Host ("[OK] Release v{0} initiated!" -f $Version) -ForegroundColor Green
        Write-Host "GitHub Actions will now build and draft the release." -ForegroundColor Gray
    }

    "release-beta" {
        if (-not $Version) {
            $Version = Read-Host 'Enter prerelease version (e.g. 1.0.0-beta.1 or 1.0.0-rc.1)'
        }
        
        if (-not ($Version -match '^\d+\.\d+\.\d+-(alpha|beta|rc)\.\d+$')) {
            Write-Host "Error: Invalid prerelease version format." -ForegroundColor Red
            Write-Host "Use: X.Y.Z-beta.N, X.Y.Z-rc.N, or X.Y.Z-alpha.N" -ForegroundColor Yellow
            exit 1
        }
        
        Confirm-ChangelogUpdated $Version
        Update-Version $Version
        
        git add src-tauri/tauri.conf.json package.json src-tauri/Cargo.toml
        git commit -m "chore: bump version to $Version"
        # Push version bump first so the tag exists on remote and the release workflow builds the bumped version.
        git push origin main
        if ($LASTEXITCODE -ne 0) {
            Write-Host "Error: Push to main failed. Fix and push manually, then run: git tag -a 'v$Version' -m 'Pre-release v$Version'; git push origin 'v$Version'" -ForegroundColor Red
            exit 1
        }
        
        git tag -a "v$Version" -m "Pre-release v$Version"
        git push origin "v$Version"
        if ($LASTEXITCODE -ne 0) { exit 1 }
        
        Write-Host ""
        Write-Host ("[OK] Pre-release v{0} initiated!" -f $Version) -ForegroundColor Green
        Write-Host "GitHub Actions will build and draft the release." -ForegroundColor Gray
    }

    "release-rc" {
        if (-not $Version) {
            $Version = Read-Host 'Enter RC version (e.g. 1.0.0-rc.1)'
        }
        
        if (-not ($Version -match '^\d+\.\d+\.\d+-rc\.\d+$')) {
            Write-Host 'Error: Invalid RC version format. Use X.Y.Z-rc.N (e.g. 1.0.0-rc.1)' -ForegroundColor Red
            exit 1
        }
        
        Confirm-ChangelogUpdated $Version
        Update-Version $Version
        
        git add src-tauri/tauri.conf.json package.json src-tauri/Cargo.toml
        git commit -m "chore: bump version to $Version"
        # Push version bump first so the release workflow builds the bumped version.
        git push origin main
        if ($LASTEXITCODE -ne 0) {
            Write-Host "Error: Push to main failed. Fix and push manually, then run: git tag -a 'v$Version' -m 'Release candidate v$Version'; git push origin 'v$Version'" -ForegroundColor Red
            exit 1
        }
        
        git tag -a "v$Version" -m "Release candidate v$Version"
        git push origin "v$Version"
        if ($LASTEXITCODE -ne 0) { exit 1 }
        
        Write-Host ""
        Write-Host ("[OK] Release candidate v{0} initiated!" -f $Version) -ForegroundColor Green
        Write-Host "GitHub Actions will build and draft the release." -ForegroundColor Gray
    }

    "set-version" {
        if (-not $Version) {
            $Version = Read-Host 'Enter version (e.g. 1.0.0)'
        }
        Update-Version $Version
        Write-Host ""
        Write-Host ("[OK] Version updated to {0} in all config files." -f $Version) -ForegroundColor Green
        Write-Host "Note: Changes are NOT committed. Run 'git add' and 'git commit' manually." -ForegroundColor Yellow
    }

    # ==========================================================================
    # SIGNING KEY COMMANDS
    # ==========================================================================

    "generate-keys" {
        Write-Host ""
        Write-Host "  Generating Tauri Signing Keys" -ForegroundColor Green
        Write-Host "  =============================" -ForegroundColor Green
        Write-Host ""
        
        $tauriDir = "$env:USERPROFILE\.tauri"
        if (-not (Test-Path $tauriDir)) {
            New-Item -ItemType Directory -Path $tauriDir -Force | Out-Null
        }
        
        $keyPath = "$tauriDir\kalam.key"
        if (Test-Path $keyPath) {
            Write-Host "  WARNING: Keys already exist at $keyPath" -ForegroundColor Yellow
            $confirm = Read-Host '  Overwrite existing keys? (y/N)'
            if ($confirm -ne "y" -and $confirm -ne "Y") {
                Write-Host "  Aborted." -ForegroundColor Gray
                exit 0
            }
        }
        
        npx tauri signer generate -w "$keyPath"
        $result = $LASTEXITCODE
        
        if ($result -eq 0) {
            Write-Host ""
            Write-Host "  [OK] Keys generated successfully!" -ForegroundColor Green
            Write-Host "  Key files:" -ForegroundColor Cyan
            Write-Host "    Private: $keyPath" -ForegroundColor Gray
            Write-Host "    Public:  $keyPath.pub" -ForegroundColor Gray
            Write-Host ""
            Write-Host "  NEXT STEPS:" -ForegroundColor Yellow
            Write-Host "  1. Run: ./tasks.ps1 show-pubkey - copy the BASE64 line into tauri.conf.json -> plugins.updater.pubkey" -ForegroundColor White
            Write-Host "  2. Add GitHub Secrets: TAURI_SIGNING_PRIVATE_KEY and TAURI_SIGNING_PRIVATE_KEY_PASSWORD" -ForegroundColor White
            Write-Host "  3. Run: ./tasks.ps1 verify-updater-signing before tagging a release" -ForegroundColor White
        } else {
            Write-Host "  [X] Key generation failed" -ForegroundColor Red
            exit 1
        }
    }

    "show-pubkey" {
        $keyPath = "$env:USERPROFILE\.tauri\kalam.key.pub"
        
        if (-not (Test-Path $keyPath)) {
            Write-Host ""
            Write-Host "  [X] Public key not found at: $keyPath" -ForegroundColor Red
            Write-Host "  Run './tasks.ps1 generate-keys' first to create a keypair." -ForegroundColor Yellow
            Write-Host ""
            exit 1
        }
        
        # Tauri expects plugins.updater.pubkey = base64(UTF-8 bytes of the full .pub file), not raw multiline text.
        $pubkeyRaw = Get-Content $keyPath -Raw
        $pubkeyNormalized = $pubkeyRaw.TrimEnd() + "`n"
        $pubkeyB64 = [Convert]::ToBase64String([System.Text.Encoding]::UTF8.GetBytes($pubkeyNormalized))
        
        Write-Host ""
        Write-Host "  Your Public Key (minisign .pub - for reference)" -ForegroundColor Green
        Write-Host "  ==============================================" -ForegroundColor Green
        Write-Host ""
        Write-Host "  $pubkeyRaw" -ForegroundColor White
        Write-Host ""
        Write-Host "  Paste this SINGLE-LINE base64 into tauri.conf.json -> plugins -> updater -> pubkey:" -ForegroundColor Gray
        Write-Host ""
        Write-Host "  ----------------------------------------------------------------" -ForegroundColor Cyan
        Write-Host "  $pubkeyB64" -ForegroundColor White
        Write-Host "  ----------------------------------------------------------------" -ForegroundColor Cyan
        Write-Host ""
        
        try {
            $pubkeyB64 | Set-Clipboard
            Write-Host "  [OK] Base64 value copied to clipboard (use this in JSON)." -ForegroundColor Green
        } catch {
            Write-Host "  (Could not copy to clipboard automatically)" -ForegroundColor Gray
        }
        Write-Host ""
    }

    "verify-updater-signing" {
        $confPath = Join-Path $RootDir "src-tauri\tauri.conf.json"
        if (-not (Test-Path $confPath)) {
            Write-Host ""
            Write-Host "  [X] Not found: $confPath" -ForegroundColor Red
            Write-Host ""
            exit 1
        }
        $conf = Get-Content $confPath -Raw | ConvertFrom-Json
        $embeddedB64 = $conf.plugins.updater.pubkey
        if (-not $embeddedB64) {
            Write-Host ""
            Write-Host "  [X] tauri.conf.json missing plugins.updater.pubkey" -ForegroundColor Red
            Write-Host ""
            exit 1
        }
        $embeddedBytes = [Convert]::FromBase64String($embeddedB64)
        $embeddedText = [System.Text.Encoding]::UTF8.GetString($embeddedBytes)
        $embeddedText = (($embeddedText -replace "`r`n", "`n") -replace "`r", "`n").TrimEnd() + "`n"

        $keyPath = "$env:USERPROFILE\.tauri\kalam.key.pub"
        if (-not (Test-Path $keyPath)) {
            Write-Host ""
            Write-Host "  [X] Local public key not found: $keyPath" -ForegroundColor Red
            Write-Host "  Run './tasks.ps1 generate-keys' or copy your CI keypair's .pub here, then re-run." -ForegroundColor Yellow
            Write-Host ""
            exit 1
        }
        $diskText = Get-Content $keyPath -Raw
        $diskText = (($diskText -replace "`r`n", "`n") -replace "`r", "`n").TrimEnd() + "`n"

        if ($embeddedText -ceq $diskText) {
            Write-Host ""
            Write-Host "  [OK] tauri.conf.json pubkey matches $keyPath" -ForegroundColor Green
            Write-Host "  Ensure GitHub secret TAURI_SIGNING_PRIVATE_KEY is the private key for this .pub file." -ForegroundColor Gray
            Write-Host ""
            exit 0
        }

        Write-Host ""
        Write-Host "  [X] Mismatch: tauri.conf.json plugins.updater.pubkey does not match $keyPath" -ForegroundColor Red
        Write-Host "  Run './tasks.ps1 show-pubkey' and update tauri.conf.json, or replace the local .pub with the keypair you use in CI." -ForegroundColor Yellow
        Write-Host ""
        exit 1
    }

    # ==========================================================================
    # HELP
    # ==========================================================================

    "help" {
        Show-Help
    }

    default {
        Write-Host ""
        Write-Host "  Unknown command: $Command" -ForegroundColor Red
        Write-Host "  Run './tasks.ps1 help' for available commands." -ForegroundColor Gray
        Write-Host ""
        exit 1
    }
}
