// Copyright 2026 Ego Hygiene
// SPDX-License-Identifier: MIT

export function greeting(name) {
    return `Hello, ${name}.`;
}

if (import.meta.url === `file://${process.argv[1]}`) {
    process.stdout.write(`${greeting("developer")}\n`);
}
