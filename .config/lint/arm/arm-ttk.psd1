# ==============================================================================
# Ego Hygiene - Universal ARM TTK Configuration
# ==============================================================================
#
# Purpose:
#   Apply the complete ARM Template Test Toolkit suite to first-party Azure
#   Resource Manager templates detected by MegaLinter.
#
# Principles:
#   - Run all applicable built-in tests by default.
#   - Avoid maintaining a duplicated allowlist of upstream test names.
#   - Skip a test only for a documented, reproducible incompatibility.
#   - Prefer targeted exceptions over broad suppression.
#
# ARM TTK:
# https://github.com/Azure/arm-ttk
#
# MegaLinter integration:
# https://megalinter.io/latest/descriptors/arm_arm_ttk/

@{
    # Run every applicable ARM TTK test except the documented exception
    # below. Omitting Test allows newly added upstream checks to become part
    # of the universal quality baseline automatically.
    Skip = @(
        # ARM TTK can report ambiguity for valid nested-template resource
        # references that are resolved outside the nested template.
        #
        # Reevaluate this exception periodically and remove it once affected
        # templates pass the upstream rule without false positives.
        'Resources Should Not Be Ambiguous'
    )
}
