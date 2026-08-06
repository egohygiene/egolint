export function GreetingCard() {
    const name = "Ego Hygiene";

    return (
        <article className="greeting-card">
            <h1>Hello!</h1>
            <p className="greeting-card__message">Welcome, {name}.</p>
        </article>
    );
}
