export interface GreetingOptions {
    readonly prefix: string;
}

export function createGreeting(
    name: string,
    options: GreetingOptions,
): string {
    const normalizedName = name.trim();

    if (normalizedName === "") {
        return options.prefix;
    }

    return `${options.prefix}, ${normalizedName}!`;
}

