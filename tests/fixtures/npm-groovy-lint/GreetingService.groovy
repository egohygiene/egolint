import groovy.transform.CompileStatic

@CompileStatic
class GreetingService {

    private final String prefix

    GreetingService(String prefix) {
        this.prefix = prefix
    }

    String formatGreeting(String name) {
        if (!name) {
            return prefix
        }

        return "${prefix}, ${name}!"
    }

}
