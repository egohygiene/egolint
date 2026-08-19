@{
    # ==========================================================================
    # Ego Hygiene — Universal PSScriptAnalyzer Configuration
    # ==========================================================================
    #
    # Purpose:
    #   Enforce PowerShell correctness, security, portability, and maintainability.
    #
    # Formatting rules are intentionally handled by:
    #   .powershell-formatter.psd1
    #
    # References:
    #   https://github.com/PowerShell/PSScriptAnalyzer
    #   https://learn.microsoft.com/powershell/utility-modules/psscriptanalyzer/
    # ==========================================================================

    # Analyze all built-in rules except the explicitly excluded policies below.
    ExcludeRules = @(
        # Write-Host is acceptable for intentional CLI presentation.
        # Libraries and reusable modules should still prefer pipeline output.
        'PSAvoidUsingWriteHost'

        # State-changing helper scripts do not universally need SupportsShouldProcess.
        # Enable this at the application or module level when dry-run semantics matter.
        'PSUseShouldProcessForStateChangingFunctions'
    )

    # Keep all meaningful diagnostic severities.
    Severity = @(
        'Error'
        'Warning'
        'Information'
    )

    Rules = @{
        # ----------------------------------------------------------------------
        # Compatibility
        # ----------------------------------------------------------------------

        PSUseCompatibleSyntax = @{
            Enable         = $true
            TargetVersions = @(
                '7.2'
                '7.4'
            )
        }

        # ----------------------------------------------------------------------
        # Security
        # ----------------------------------------------------------------------

        PSAvoidUsingConvertToSecureStringWithPlainText = @{
            Enable = $true
        }

        PSAvoidUsingPlainTextForPassword = @{
            Enable = $true
        }

        # ----------------------------------------------------------------------
        # Maintainability
        # ----------------------------------------------------------------------

        PSAvoidLongLines = @{
            Enable            = $true
            MaximumLineLength = 120
        }

        PSAvoidUsingPositionalParameters = @{
            Enable           = $true
            CommandAllowList = @(
                'Join-Path'
                'Split-Path'
            )
        }
    }
}
