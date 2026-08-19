package main

import rego.v1

deny contains message if {
    input.kind == "Deployment"
    input.spec.replicas < 2
    message := "Deployments require at least two replicas."
}
