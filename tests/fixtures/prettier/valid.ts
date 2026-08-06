interface GreetingOptions {
  readonly excited?: boolean;
}

export function formatGreeting(
  name: string,
  options: GreetingOptions = {},
): string {
  const normalizedName = name.trim();
  const punctuation = options.excited ? "!" : ".";

  return normalizedName === ""
    ? `Hello${punctuation}`
    : `Hello, ${normalizedName}${punctuation}`;
}
