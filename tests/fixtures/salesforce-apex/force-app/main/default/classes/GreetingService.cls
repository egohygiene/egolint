public with sharing class GreetingService {
    public static String formatGreeting(String name) {
        String normalizedName = String.isBlank(name)
            ? ''
            : name.trim();

        if (normalizedName == '') {
            return 'Hello.';
        }

        return 'Hello, ' + normalizedName + '.';
    }
}

