export interface ButtonProps {
    disabled?: boolean;
    label: string;
}

export function Button({ disabled = false, label }: ButtonProps) {
    return <button disabled={disabled}>{label}</button>;
}
