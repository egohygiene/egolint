package egohygiene.fixtures

object GreetingService {
  def formatGreeting(name: String): String = {
    val normalizedName = name.trim

    if (normalizedName.isEmpty) {
      "Hello."
    } else {
      s"Hello, $normalizedName."
    }
  }
}
