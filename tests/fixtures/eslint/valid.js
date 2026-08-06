export function createGreeting(name) {
    const normalizedName = name.trim();

    if (normalizedName === "") {
        return "Hello";
    }

    return `Hello, ${normalizedName}!`;
}
