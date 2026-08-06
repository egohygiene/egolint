package fixtures.checkstyle;

import java.util.Objects;

/**
 * Creates personalized greeting messages.
 */
public final class GreetingService {

    private final String prefix;

    /**
     * Creates a greeting service.
     *
     * @param prefix greeting prefix
     */
    public GreetingService(String prefix) {
        this.prefix = Objects.requireNonNull(prefix);
    }

    /**
     * Creates a greeting for a supplied name.
     *
     * @param name greeting recipient
     * @return formatted greeting
     */
    public String createGreeting(String name) {
        if (name == null || name.isBlank()) {
            return prefix;
        }

        return "%s, %s!".formatted(prefix, name.trim());
    }
}

