package egohygiene.fixture

import rego.v1

default allowed := false

allowed if input.enabled
