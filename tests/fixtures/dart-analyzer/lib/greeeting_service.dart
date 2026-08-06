final class GreetingService {
  const GreetingService({
    required this.prefix,
  });

  final String prefix;

  String createGreeting(String name) {
    if (name.isEmpty) {
      return prefix;
    }

    return "$prefix, $name!";
  }
}

