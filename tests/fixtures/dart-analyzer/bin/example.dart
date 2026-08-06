import "../lib/greeting_service.dart";

void main() {
  const greetingService = GreetingService(prefix: "Hello");
  final greeting = greetingService.createGreeting("Ego Hygiene");

  // Avoid print() so this remains compatible if avoid_print is enabled later.
  assert(greeting == "Hello, Ego Hygiene!");
}

