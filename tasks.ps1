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
    Write-Host "    release-beta [version] - Create a beta release (v1.0.0-beta.1)"
    Write-Host "    set-version <version>  - Update version only (no commit/tag)"
    Write-Host ""
    Write-Host "  SIGNING KEYS (for auto-updater)" -ForegroundColor Yellow
    Write-Host "  --------------------------------"
    Write-Host "    generate-keys     - Generate Tauri signing keys for updates"
    Write-Host "    show-pubkey       - Display the public key to add to config"
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
        npm run tauri dev
    }

    "build" {
        Write-Host "Building Tauri app (unsigned)..." -ForegroundColor Cyan
        npm run tauri:build:unsigned
    }

    "build-signed" {
        Write-Host "Building Tauri app (signed; requires TAURI_SIGNING_PRIVATE_KEY)..." -ForegroundColor Cyan
        npm run tauri build
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
            Write-Host "`n✓ SUCCESS: All checks passed." -ForegroundColor Green
        } else {
            Write-Host "`n✗ FAILURE: Some checks failed." -ForegroundColor Red
            exit 1
        }
    }

    "fmt" {
        Write-Host "Formatting Rust code..." -ForegroundColor Cyan
        Set-Location src-tauri
        cargo fmt
        Set-Location $RootDir
    }

    "lint" {
        Write-Host "Linting Rust code..." -ForegroundColor Cyan
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
            $Version = Read-Host "Enter version (e.g., 1.0.0)"
        }
        
        if (-not ($Version -match '^\d+\.\d+\.\d+$')) {
            Write-Host "Error: Invalid version format. Use semantic versioning (e.g., 1.0.0)" -ForegroundColor Red
            exit 1
        }
        
        Update-Version $Version
        
        git add src-tauri/tauri.conf.json package.json src-tauri/Cargo.toml
        git commit -m "chore: bump version to $Version"
        git push origin main
        
        git tag -a "v$Version" -m "Release v$Version"
        git push origin "v$Version"
        
        Write-Host ""
        Write-Host "✓ Release v$Version initiated!" -ForegroundColor Green
        Write-Host "GitHub Actions will now build and draft the release." -ForegroundColor Gray
    }

    "release-beta" {
        if (-not $Version) {
            $Version = Read-Host "Enter beta version (e.g., 1.0.0-beta.1)"
        }
        
        if (-not ($Version -match '^\d+\.\d+\.\d+-(alpha|beta|rc)\.\d+$')) {
            Write-Host "Error: Invalid beta version format." -ForegroundColor Red
            Write-Host "Use: X.Y.Z-beta.N, X.Y.Z-alpha.N, or X.Y.Z-rc.N" -ForegroundColor Yellow
            exit 1
        }
        
        $BaseVersion = $Version -replace '-.*$', ''
        Update-Version $BaseVersion
        
        git add src-tauri/tauri.conf.json package.json src-tauri/Cargo.toml
        git commit -m "chore: bump version to $BaseVersion for $Version release"
        git push origin main
        
        git tag -a "v$Version" -m "Beta release v$Version"
        git push origin "v$Version"
        
        Write-Host ""
        Write-Host "✓ Beta release v$Version initiated!" -ForegroundColor Green
    }

    "set-version" {
        if (-not $Version) {
            $Version = Read-Host "Enter version (e.g., 1.0.0)"
        }
        Update-Version $Version
        Write-Host ""
        Write-Host "✓ Version updated to $Version in all config files." -ForegroundColor Green
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
            $confirm = Read-Host "  Overwrite existing keys? (y/N)"
            if ($confirm -ne "y" -and $confirm -ne "Y") {
                Write-Host "  Aborted." -ForegroundColor Gray
                exit 0
            }
        }
        
        npx tauri signer generate -w "$keyPath"
        $result = $LASTEXITCODE
        
        if ($result -eq 0) {
            Write-Host ""
            Write-Host "  ✓ Keys generated successfully!" -ForegroundColor Green
            Write-Host "  Key files:" -ForegroundColor Cyan
            Write-Host "    Private: $keyPath" -ForegroundColor Gray
            Write-Host "    Public:  $keyPath.pub" -ForegroundColor Gray
            Write-Host ""
            Write-Host "  NEXT STEPS:" -ForegroundColor Yellow
            Write-Host "  1. Run: ./tasks.ps1 show-pubkey" -ForegroundColor White
            Write-Host "  2. Add GitHub Secrets: TAURI_SIGNING_PRIVATE_KEY and TAURI_SIGNING_PRIVATE_KEY_PASSWORD" -ForegroundColor White
        } else {
            Write-Host "  ✗ Key generation failed" -ForegroundColor Red
            exit 1
        }
    }

    "show-pubkey" {
        $keyPath = "$env:USERPROFILE\.tauri\kalam.key.pub"
        
        if (-not (Test-Path $keyPath)) {
            Write-Host ""
            Write-Host "  ✗ Public key not found at: $keyPath" -ForegroundColor Red
            Write-Host "  Run './tasks.ps1 generate-keys' first to create a keypair." -ForegroundColor Yellow
            Write-Host ""
            exit 1
        }
        
        $pubkey = Get-Content $keyPath -Raw
        $pubkey = $pubkey.Trim()
        
        Write-Host ""
        Write-Host "  Your Public Key" -ForegroundColor Green
        Write-Host "  ===============" -ForegroundColor Green
        Write-Host ""
        Write-Host "  Copy this key and paste it into:" -ForegroundColor Gray
        Write-Host "  src-tauri/tauri.conf.json > plugins > updater > pubkey" -ForegroundColor Gray
        Write-Host ""
        Write-Host "  ┌──────────────────────────────────────────────────────────────┐" -ForegroundColor Cyan
        Write-Host "  │ $pubkey" -ForegroundColor White
        Write-Host "  └──────────────────────────────────────────────────────────────┘" -ForegroundColor Cyan
        Write-Host ""
        
        try {
            $pubkey | Set-Clipboard
            Write-Host "  ✓ Copied to clipboard!" -ForegroundColor Green
        } catch {
            Write-Host "  (Could not copy to clipboard automatically)" -ForegroundColor Gray
        }
        Write-Host ""
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
