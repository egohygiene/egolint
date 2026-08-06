terraform {
  required_version = ">= 1.0.0"
}

locals {
  normalized_greeting_name = trimspace(var.greeting_name)
}
