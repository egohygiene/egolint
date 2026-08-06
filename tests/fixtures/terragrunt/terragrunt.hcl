locals {
  environment = "fixture"
  region      = "us-east-1"
}

terraform {
  source = "./modules/example"
}

inputs = {
  environment = local.environment
  region      = local.region
}
