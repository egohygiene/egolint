// Copyright 2026 Ego Hygiene
// SPDX-License-Identifier: MIT

const allowedTypes = Object.freeze([
    "build",
    "chore",
    "ci",
    "docs",
    "feat",
    "fix",
    "perf",
    "refactor",
    "revert",
    "security",
    "style",
    "test",
]);

const emojiToken = String.raw`(?::[a-z0-9_+-]+:|\p{Extended_Pictographic}(?:\uFE0F|\p{Emoji_Modifier})?(?:\u200D\p{Extended_Pictographic}(?:\uFE0F|\p{Emoji_Modifier})?)*)`;
const conventionalEmojiHeaderPattern = new RegExp(
    String.raw`^([a-z][a-z0-9-]*)\(([a-z0-9][a-z0-9._/-]*)\):\s+(${emojiToken})\s+(.+)$`,
    "u",
);

function normalizeCommitHeader(rawHeader) {
    return rawHeader.normalize("NFC").replace(/\s+/g, " ").trim();
}

function parseCommitHeader(rawHeader) {
    const match = conventionalEmojiHeaderPattern.exec(normalizeCommitHeader(rawHeader));
    if (!match) {
        return null;
    }

    const [, type, scope, emoji, subject] = match;
    return { emoji, scope, subject, type };
}

function subjectStartsWithLowercase(subject) {
    const [firstCharacter] = Array.from(subject.trim());
    if (!firstCharacter) {
        return false;
    }

    return (
        firstCharacter === firstCharacter.toLocaleLowerCase() &&
        firstCharacter !== firstCharacter.toLocaleUpperCase()
    );
}

function validateCommitHeader(rawHeader) {
    const parsedHeader = parseCommitHeader(rawHeader);
    if (!parsedHeader) {
        return [false, "Expected format: <type>(<scope>): <emoji> <subject>"];
    }

    if (!allowedTypes.includes(parsedHeader.type)) {
        return [false, `Unsupported commit type: ${parsedHeader.type}`];
    }

    if (!subjectStartsWithLowercase(parsedHeader.subject)) {
        return [false, "Commit subjects must begin with a lowercase letter"];
    }

    return [true, "Commit header follows the canonical emoji policy"];
}

module.exports = {
    allowedTypes,
    conventionalEmojiHeaderPattern,
    parseCommitHeader,
    subjectStartsWithLowercase,
    validateCommitHeader,
};
