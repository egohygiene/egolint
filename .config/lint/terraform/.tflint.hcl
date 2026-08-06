# ==============================================================================
# Ego Hygiene - Universal TFLint Configuration
# ==============================================================================
#
# Purpose:
#   Enforce Terraform language correctness and maintainability without assuming
#   a specific cloud provider.
#
# Provider-specific plugins belong in EgoLint profiles such as:
#   terraform-aws
#   terraform-azure
#   terraform-gcp
#
# References:
#   https://github.com/terraform-linters/tflint
#   https://github.com/terraform-linters/tflint/blob/master/docs/user-guide/config.md
# ==============================================================================

config {
  # Analyze local modules referenced by the current module.
  call_module_type = "local"

  # Findings must produce a failing exit status.
  force = false

  # Keep the universal baseline enabled rather than requiring every rule to be
  # opted into manually.
  disabled_by_default = false
}

plugin "terraform" {
  enabled = true
  preset  = "recommended"
}

# ------------------------------------------------------------------------------
# Documentation
# ------------------------------------------------------------------------------

rule "terraform_documented_variables" {
  enabled = true
}

rule "terraform_documented_outputs" {
  enabled = true
}

# ------------------------------------------------------------------------------
# Type safety
# ------------------------------------------------------------------------------

rule "terraform_typed_variables" {
  enabled = true
}

# ------------------------------------------------------------------------------
# Module hygiene
# ------------------------------------------------------------------------------

rule "terraform_module_pinned_source" {
  enabled = true
}

rule "terraform_naming_convention" {
  enabled = true

  format = "snake_case"
}
