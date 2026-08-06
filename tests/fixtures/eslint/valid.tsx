interface GreetingCardProperties {
    readonly name: string;
}

export function GreetingCard({
    name,
}: GreetingCardProperties): React.JSX.Element {
    return <p>Hello, {name}.</p>;
}
