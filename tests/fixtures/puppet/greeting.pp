# @summary
#   Declares a greeting notification for validation purposes.
#
# @param name
#   Name included in the rendered greeting.
class egohygiene::greeting (
  String[1] $name = 'World',
) {
  notify { 'egohygiene_greeting':
    message => "Hello, ${name}.",
  }
}
