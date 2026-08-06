package fixtures.pmd;

import java.util.Objects;

public final class GreetingService {

    private final String prefix;

    public GreetingService(String prefix) {
        this.prefix = Objects.requireNonNull(prefix);
    }

    public String createGreeting(String name) {
        if (name == null || name.isBlank()) {
            return prefix;
        }

        return "%s, %s!".formatted(prefix, name.trim());
    }
}

