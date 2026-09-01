#!/usr/bin/env node

// Copyright 2026 Ego Hygiene
// SPDX-License-Identifier: MIT

import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdir, readFile, rename, rm, writeFile } from "node:fs/promises";
import { dirname, isAbsolute, join, relative, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";

import { publint } from "publint";
import { formatMessage } from "publint/utils";

const CONTRACT_VERSION = 1;
const POLICY_PATH = ".config/rules/javascript-package-quality.v1.json";
const DEFAULT_MANIFEST = "egolint.javascript-package-quality.json";
const DEFAULT_JSON_REPORT = ".reports/egolint/javascript-package-quality.json";
const DEFAULT_SARIF_REPORT = ".reports/egolint/javascript-package-quality.sarif";
const OWNER = "egohygiene/egolint";
const DEFAULT_IGNORE_PATTERNS = [
    ".git/**",
    ".reports/**",
    ".egolint-biome-*.json",
    "build/**",
    "coverage/**",
    "dist/**",
    "generated/**",
    "node_modules/**",
    "target/**",
];
const ANSI_ESCAPE_PATTERN = new RegExp(`${String.fromCodePoint(27)}\\[[0-9;]*m`, "g");
const CONTROL_CHARACTER_PATTERN = /\p{Cc}/gu;

const scriptDirectory = dirname(fileURLToPath(import.meta.url));
const repositoryRoot = resolve(scriptDirectory, "..");

class EgolintJavascriptError extends Error {
    constructor(message, exitCode) {
        super(message);
        this.exitCode = exitCode;
    }
}

function configurationError(message) {
    return new EgolintJavascriptError(`configuration error: ${message}`, 2);
}

function runtimeError(message) {
    return new EgolintJavascriptError(`adapter execution failed: ${message}`, 3);
}

function portablePath(value) {
    return value.split(sep).join("/");
}

function boundedText(value, maximum = 4096) {
    return String(value)
        .replaceAll(ANSI_ESCAPE_PATTERN, "")
        .replaceAll(CONTROL_CHARACTER_PATTERN, " ")
        .replaceAll(/\s+/g, " ")
        .trim()
        .slice(0, maximum);
}

function containedPath(root, requested, label) {
    const resolved = resolve(root, requested);
    const relativePath = relative(root, resolved);
    if (relativePath === ".." || relativePath.startsWith(`..${sep}`) || isAbsolute(relativePath)) {
        throw configurationError(`${label} must remain inside the workspace: ${requested}`);
    }
    return resolved;
}

function validateReportPath(workspace, requested, label) {
    const resolved = containedPath(workspace, requested, label);
    const relativePath = portablePath(relative(workspace, resolved));
    if (relativePath !== ".reports/egolint" && !relativePath.startsWith(".reports/egolint/")) {
        throw configurationError(`${label} must remain under .reports/egolint`);
    }
    return resolved;
}

async function readJson(path, label) {
    let raw;
    try {
        raw = await readFile(path, "utf8");
    } catch (error) {
        throw configurationError(`${label} is not readable at ${path}: ${error.message}`);
    }
    try {
        return JSON.parse(raw);
    } catch (error) {
        throw configurationError(`${label} is invalid JSON at ${path}: ${error.message}`);
    }
}

function validateManifest(manifest) {
    if (manifest?.schema_version !== CONTRACT_VERSION) {
        throw configurationError(`manifest schema_version must equal ${CONTRACT_VERSION}`);
    }
    if (!new Set(["base", "react-library"]).has(manifest.profile)) {
        throw configurationError("manifest profile must be base or react-library");
    }
    if (typeof manifest.package_path !== "string" || manifest.package_path.trim() === "") {
        throw configurationError("manifest package_path must be a nonempty relative path");
    }
    if (manifest.package_path.includes("\\") || isAbsolute(manifest.package_path)) {
        throw configurationError(
            "manifest package_path must use portable workspace-relative syntax",
        );
    }
    if (!new Set(["npm", "private"]).has(manifest.publication)) {
        throw configurationError("manifest publication must be npm or private");
    }
    if (manifest.ignore !== undefined) {
        if (!Array.isArray(manifest.ignore) || manifest.ignore.length > 128) {
            throw configurationError("manifest ignore must contain at most 128 relative patterns");
        }
        for (const pattern of manifest.ignore) {
            if (
                typeof pattern !== "string" ||
                pattern.trim() === "" ||
                pattern.startsWith("!") ||
                pattern.includes("\\") ||
                isAbsolute(pattern) ||
                pattern.split("/").includes("..")
            ) {
                throw configurationError(
                    "manifest ignore patterns must be nonempty, portable, workspace-relative globs",
                );
            }
        }
    }
}

function validateProfile(profile, manifest) {
    if (profile?.schema_version !== CONTRACT_VERSION) {
        throw configurationError(`profile schema_version must equal ${CONTRACT_VERSION}`);
    }
    if (profile.id !== "egolint/javascript-package-quality") {
        throw configurationError("unexpected JavaScript package-quality profile id");
    }
    if (!profile.variants?.[manifest.profile]) {
        throw configurationError(`profile variant is not defined: ${manifest.profile}`);
    }
    if (profile.adapters?.biome?.linter_enabled !== false) {
        throw configurationError("Biome linting must remain disabled in the canonical profile");
    }
}

async function installedPackageVersion(packageName) {
    const packagePath = resolve(
        repositoryRoot,
        "node_modules",
        ...packageName.split("/"),
        "package.json",
    );
    const manifest = await readJson(packagePath, `${packageName} package manifest`);
    if (typeof manifest.version !== "string") {
        throw runtimeError(`${packageName} package manifest does not contain a version`);
    }
    return manifest.version;
}

async function verifyAdapterVersions(profile) {
    const mappings = [
        ["oxlint", "oxlint"],
        ["oxlint_tsgolint", "oxlint-tsgolint"],
        ["biome", "@biomejs/biome"],
        ["publint", "publint"],
    ];
    const observed = {};
    for (const [profileKey, packageName] of mappings) {
        const expected = profile.adapters[profileKey].version;
        const actual = await installedPackageVersion(packageName);
        if (actual !== expected) {
            throw runtimeError(
                `${packageName} version ${actual} does not match reviewed pin ${expected}`,
            );
        }
        observed[profileKey] = actual;
    }
    return observed;
}

function pnpmCommand() {
    return process.platform === "win32" ? "pnpm.cmd" : "pnpm";
}

function runAdapter(arguments_, allowedExitCodes, label) {
    const result = spawnSync(pnpmCommand(), arguments_, {
        cwd: repositoryRoot,
        encoding: "utf8",
        env: { ...process.env, NO_COLOR: "1" },
        maxBuffer: 32 * 1024 * 1024,
    });
    if (result.error) {
        throw runtimeError(`${label} could not start: ${result.error.message}`);
    }
    if (!allowedExitCodes.has(result.status)) {
        throw runtimeError(
            `${label} exited with ${result.status}: ${boundedText(result.stderr || result.stdout)}`,
        );
    }
    return result.stdout;
}

function sourcePath(rawPath, packageDirectory, workspace) {
    if (!rawPath || typeof rawPath !== "string") return null;
    const candidates = isAbsolute(rawPath)
        ? [resolve(rawPath)]
        : [
              resolve(packageDirectory, rawPath),
              resolve(repositoryRoot, rawPath),
              resolve(workspace, rawPath),
          ];
    for (const candidate of candidates) {
        const relativePath = relative(workspace, candidate);
        if (
            relativePath !== ".." &&
            !relativePath.startsWith(`..${sep}`) &&
            !isAbsolute(relativePath)
        ) {
            return portablePath(relativePath || ".");
        }
    }
    return null;
}

function severity(value) {
    switch (String(value).toLowerCase()) {
        case "error":
        case "critical":
            return "error";
        case "warn":
        case "warning":
            return "warning";
        default:
            return "info";
    }
}

function findingId(toolId, ruleId, path, line, message) {
    const digest = createHash("sha256")
        .update([toolId, ruleId, path ?? "", line ?? "", message].join("\u0000"))
        .digest("hex")
        .slice(0, 24);
    return `javascript-quality-${digest}`;
}

function normalizedFinding({
    toolId,
    toolName,
    toolVersion,
    ruleId,
    severity: findingSeverity,
    message,
    path,
    startLine,
    startColumn,
    endLine,
    endColumn,
    configurationPath,
    profileMapping,
    fixSafety,
    applicability = "applicable",
}) {
    const cleanMessage = boundedText(message, 16384);
    const location = path
        ? {
              path,
              start_line: startLine ?? 1,
              start_column: startColumn ?? 1,
              end_line: endLine ?? startLine ?? 1,
              end_column: endColumn ?? startColumn ?? 1,
          }
        : null;
    const id = findingId(toolId, ruleId, path, startLine, cleanMessage);
    return {
        finding: {
            schema_version: CONTRACT_VERSION,
            id,
            rule: { tool_id: toolId, rule_id: ruleId },
            severity: findingSeverity,
            message: cleanMessage,
            location,
            ownership: {
                owner: OWNER,
                policy_source: POLICY_PATH,
                configuration_path: configurationPath,
            },
            fingerprint: id,
            evidence: [],
            suppressed_by: null,
        },
        source_tool: toolName,
        source_tool_version: toolVersion,
        profile_mapping: profileMapping,
        fix_safety: fixSafety,
        applicability,
        suppression_state: "none",
    };
}

function parseJsonOutput(raw, label) {
    try {
        return JSON.parse(raw || "{}");
    } catch (error) {
        throw runtimeError(`${label} emitted invalid JSON: ${error.message}`);
    }
}

function normalizeOxlint(raw, context) {
    const document = parseJsonOutput(raw, "Oxlint");
    const diagnostics = Array.isArray(document.diagnostics) ? document.diagnostics : [];
    return diagnostics.map((diagnostic) => {
        const span = diagnostic.labels?.[0]?.span ?? {};
        const ruleId = String(diagnostic.code ?? "oxlint/unknown");
        const path = sourcePath(diagnostic.filename, context.packageDirectory, context.workspace);
        return normalizedFinding({
            toolId: "OXLINT",
            toolName: "oxlint",
            toolVersion: context.versions.oxlint,
            ruleId,
            severity: severity(diagnostic.severity),
            message: diagnostic.help
                ? `${diagnostic.message} ${diagnostic.help}`
                : (diagnostic.message ?? ruleId),
            path,
            startLine: span.line,
            startColumn: span.column,
            endLine: span.line,
            endColumn: span.column && span.length ? span.column + span.length : span.column,
            configurationPath: context.variant.oxlint_config,
            profileMapping: context.manifest.profile,
            fixSafety: "not_requested",
        });
    });
}

function normalizeBiome(raw, context) {
    const document = parseJsonOutput(raw, "Biome SARIF");
    const results = document.runs?.flatMap((run) => run.results ?? []) ?? [];
    return results.map((result) => {
        const physical = result.locations?.[0]?.physicalLocation ?? {};
        const region = physical.region ?? {};
        const path = sourcePath(
            physical.artifactLocation?.uri,
            context.packageDirectory,
            context.workspace,
        );
        const ruleId = String(result.ruleId ?? "biome/unknown");
        const safe = ruleId === "format" || ruleId.startsWith("assist/source/");
        return normalizedFinding({
            toolId: "BIOME",
            toolName: "biome",
            toolVersion: context.versions.biome,
            ruleId,
            severity: severity(result.level),
            message: result.message?.text ?? ruleId,
            path,
            startLine: region.startLine,
            startColumn: region.startColumn,
            endLine: region.endLine,
            endColumn: region.endColumn,
            configurationPath: context.variant.biome_config,
            profileMapping: context.manifest.profile,
            fixSafety: safe ? "safe" : "not_requested",
        });
    });
}

function publintLocation(message, packageDirectory, workspace) {
    const rawPath =
        message.path ??
        message.file ??
        message.location?.path ??
        message.loc?.file ??
        "package.json";
    const range = message.location?.range ?? message.loc ?? {};
    const start = range.start ?? {};
    const end = range.end ?? {};
    return {
        path: sourcePath(rawPath, packageDirectory, workspace) ?? "package.json",
        startLine: start.line ?? range.line ?? 1,
        startColumn: start.column ?? range.column ?? 1,
        endLine: end.line ?? start.line ?? range.line ?? 1,
        endColumn: end.column ?? start.column ?? range.column ?? 1,
    };
}

async function runPublint(context) {
    if (context.manifest.publication === "private") {
        return { findings: [], status: "not_applicable" };
    }
    let result;
    try {
        result = await publint({
            pkgDir: context.packageDirectory,
            pack: "pnpm",
            strict: false,
            level: "suggestion",
        });
    } catch (error) {
        throw runtimeError(`publint failed: ${error.message}`);
    }
    const findings = (result.messages ?? []).map((message) => {
        const location = publintLocation(message, context.packageDirectory, context.workspace);
        let rendered;
        try {
            rendered = formatMessage(message, result.pkg);
        } catch {
            rendered = message.code ?? "publint package validation finding";
        }
        return normalizedFinding({
            toolId: "PUBLINT",
            toolName: "publint",
            toolVersion: context.versions.publint,
            ruleId: message.code ?? "publint/unknown",
            severity: severity(message.type),
            message: rendered,
            path: location.path,
            startLine: location.startLine,
            startColumn: location.startColumn,
            endLine: location.endLine,
            endColumn: location.endColumn,
            configurationPath: POLICY_PATH,
            profileMapping: context.manifest.profile,
            fixSafety: "none",
        });
    });
    return { findings, status: "applicable" };
}

function toolResult(toolId, findings, statusOverride) {
    if (statusOverride === "not_applicable") {
        return {
            schema_version: CONTRACT_VERSION,
            tool_id: toolId,
            owner: OWNER,
            policy_source: POLICY_PATH,
            status: "not_applicable",
            enforcement: "disabled",
            finding_count: 0,
            warning_count: 0,
            duration_ms: null,
            evidence: [],
        };
    }
    const relevant = findings.filter((item) => item.finding.rule.tool_id === toolId);
    const errors = relevant.filter((item) => item.finding.severity === "error").length;
    const warnings = relevant.filter((item) => item.finding.severity === "warning").length;
    return {
        schema_version: CONTRACT_VERSION,
        tool_id: toolId,
        owner: OWNER,
        policy_source: POLICY_PATH,
        status: errors > 0 ? "failed_findings" : warnings > 0 ? "passed_with_warnings" : "passed",
        enforcement: "blocking",
        finding_count: errors,
        warning_count: warnings,
        duration_ms: null,
        evidence: [],
    };
}

function compareFindings(left, right) {
    const leftFinding = left.finding;
    const rightFinding = right.finding;
    return (
        leftFinding.rule.tool_id.localeCompare(rightFinding.rule.tool_id) ||
        leftFinding.rule.rule_id.localeCompare(rightFinding.rule.rule_id) ||
        (leftFinding.location?.path ?? "").localeCompare(rightFinding.location?.path ?? "") ||
        (leftFinding.location?.start_line ?? 0) - (rightFinding.location?.start_line ?? 0) ||
        leftFinding.id.localeCompare(rightFinding.id)
    );
}

function summarize(findings, toolResults) {
    const count = (value) => findings.filter((item) => item.finding.severity === value).length;
    return {
        errors: count("error"),
        warnings: count("warning"),
        infos: count("info"),
        not_applicable_tools: toolResults
            .filter((result) => result.status === "not_applicable")
            .map((result) => result.tool_id)
            .sort(),
    };
}

function toSarif(report) {
    const rules = new Map();
    const results = [];
    for (const item of report.findings) {
        const finding = item.finding;
        const ruleId = `${finding.rule.tool_id}:${finding.rule.rule_id}`;
        if (!rules.has(ruleId)) {
            rules.set(ruleId, {
                id: ruleId,
                name: finding.rule.rule_id,
                shortDescription: { text: `${finding.rule.tool_id} ${finding.rule.rule_id}` },
                properties: {
                    egolintOwner: finding.ownership.owner,
                    egolintPolicySource: finding.ownership.policy_source,
                    sourceTool: item.source_tool,
                    sourceToolVersion: item.source_tool_version,
                    fixSafety: item.fix_safety,
                },
            });
        }
        const result = {
            ruleId,
            level:
                finding.severity === "error"
                    ? "error"
                    : finding.severity === "warning"
                      ? "warning"
                      : "note",
            message: { text: finding.message },
            partialFingerprints: { "egolint/v1": finding.fingerprint },
            properties: {
                applicability: item.applicability,
                profileMapping: item.profile_mapping,
                suppressionState: item.suppression_state,
            },
        };
        if (finding.location) {
            result.locations = [
                {
                    physicalLocation: {
                        artifactLocation: { uri: finding.location.path },
                        region: {
                            startLine: finding.location.start_line,
                            startColumn: finding.location.start_column,
                            endLine: finding.location.end_line,
                            endColumn: finding.location.end_column,
                        },
                    },
                },
            ];
        }
        results.push(result);
    }
    return {
        $schema: "https://json.schemastore.org/sarif-2.1.0.json",
        version: "2.1.0",
        runs: [
            {
                tool: {
                    driver: {
                        name: "Egolint",
                        informationUri: "https://egolint.egohygiene.io",
                        rules: [...rules.values()].sort((a, b) => a.id.localeCompare(b.id)),
                    },
                },
                results,
                properties: {
                    egolintProfile: report.profile.id,
                    egolintProfileVersion: report.profile.version,
                    egolintVariant: report.profile.variant,
                },
            },
        ],
    };
}

async function writeAtomic(path, value) {
    await mkdir(dirname(path), { recursive: true });
    const temporaryPath = `${path}.tmp-${process.pid}`;
    await writeFile(temporaryPath, `${JSON.stringify(value, null, 2)}\n`, "utf8");
    await rename(temporaryPath, path);
}

async function materializeBiomeConfig(configurationPath, workspace, ignorePatterns) {
    const config = await readJson(configurationPath, "Biome configuration");
    delete config.$schema;
    config.formatter = config.formatter ?? {};
    config.formatter.useEditorconfig = false;
    config.files = config.files ?? {};
    config.files.includes = [
        ...(config.files.includes ?? []),
        ...[...DEFAULT_IGNORE_PATTERNS, ...ignorePatterns].map((pattern) => `!${pattern}`),
    ];
    const path = join(workspace, `.egolint-biome-${process.pid}-${Date.now()}.json`);
    await writeFile(path, `${JSON.stringify(config, null, 2)}\n`, {
        encoding: "utf8",
        flag: "wx",
    });
    return path;
}

function printFindings(report) {
    for (const item of report.findings) {
        const finding = item.finding;
        const location = finding.location
            ? `${finding.location.path}:${finding.location.start_line}:${finding.location.start_column}`
            : "repository";
        console.log(
            `egolint:${location}: ${finding.severity}: ${finding.message} [${finding.rule.tool_id}/${finding.rule.rule_id}]`,
        );
    }
    console.log(
        `Egolint JavaScript package quality: ${report.summary.errors} error(s), ${report.summary.warnings} warning(s), ${report.summary.infos} info(s)`,
    );
}

async function main() {
    const { values } = parseArgs({
        args: process.argv.slice(2),
        strict: true,
        allowPositionals: false,
        options: {
            workspace: { type: "string", default: "." },
            manifest: { type: "string", default: DEFAULT_MANIFEST },
            output: { type: "string", default: DEFAULT_JSON_REPORT },
            sarif: { type: "string", default: DEFAULT_SARIF_REPORT },
        },
    });

    const workspace = resolve(values.workspace);
    const manifestPath = containedPath(workspace, values.manifest, "manifest");
    const jsonReportPath = validateReportPath(workspace, values.output, "JSON report");
    const sarifReportPath = validateReportPath(workspace, values.sarif, "SARIF report");
    const manifest = await readJson(manifestPath, "JavaScript package-quality manifest");
    validateManifest(manifest);

    const profile = await readJson(resolve(repositoryRoot, POLICY_PATH), "canonical profile");
    validateProfile(profile, manifest);
    const variant = profile.variants[manifest.profile];
    const packageDirectory = containedPath(workspace, manifest.package_path, "package_path");
    const packageManifest = await readJson(
        resolve(packageDirectory, "package.json"),
        "package.json",
    );
    if (manifest.publication === "npm" && packageManifest.private === true) {
        throw configurationError("publication npm cannot target a package.json with private=true");
    }
    if (manifest.publication === "private" && packageManifest.private !== true) {
        throw configurationError("publication private requires package.json private=true");
    }

    const versions = await verifyAdapterVersions(profile);
    const context = { workspace, packageDirectory, manifest, profile, variant, versions };
    const oxlintConfig = resolve(repositoryRoot, variant.oxlint_config);
    const biomeConfig = resolve(repositoryRoot, variant.biome_config);
    const ignorePatterns = manifest.ignore ?? [];

    const oxlintArguments = [
        "--dir",
        repositoryRoot,
        "exec",
        "oxlint",
        "--config",
        oxlintConfig,
        "--format",
        "json",
        "--disable-nested-config",
    ];
    for (const pattern of ignorePatterns) {
        oxlintArguments.push("--ignore-pattern", pattern);
    }
    oxlintArguments.push(packageDirectory);

    const oxlintRaw = runAdapter(oxlintArguments, new Set([0, 1]), "Oxlint");
    const temporaryBiomeConfig = await materializeBiomeConfig(
        biomeConfig,
        workspace,
        ignorePatterns,
    );
    let biomeRaw;
    try {
        biomeRaw = runAdapter(
            [
                "--dir",
                repositoryRoot,
                "exec",
                "biome",
                "ci",
                "--colors=off",
                "--config-path",
                temporaryBiomeConfig,
                "--reporter=sarif",
                "--max-diagnostics=none",
                packageDirectory,
            ],
            new Set([0, 1]),
            "Biome",
        );
    } finally {
        await rm(temporaryBiomeConfig, { force: true });
    }
    const publintResult = await runPublint(context);

    const findings = [
        ...normalizeOxlint(oxlintRaw, context),
        ...normalizeBiome(biomeRaw, context),
        ...publintResult.findings,
    ].sort(compareFindings);
    const toolResults = [
        toolResult("OXLINT", findings),
        toolResult("BIOME", findings),
        toolResult("PUBLINT", findings, publintResult.status),
    ].sort((a, b) => a.tool_id.localeCompare(b.tool_id));
    const report = {
        schema_version: CONTRACT_VERSION,
        profile: {
            id: profile.id,
            version: profile.version,
            variant: manifest.profile,
            policy_source: profile.policy_source,
        },
        package: {
            path: portablePath(relative(workspace, packageDirectory) || "."),
            publication: manifest.publication,
            name: packageManifest.name ?? null,
        },
        adapters: {
            oxlint: versions.oxlint,
            oxlint_tsgolint: versions.oxlint_tsgolint,
            biome: versions.biome,
            publint: versions.publint,
        },
        tool_results: toolResults,
        findings,
        summary: summarize(findings, toolResults),
    };

    await writeAtomic(jsonReportPath, report);
    await writeAtomic(sarifReportPath, toSarif(report));
    printFindings(report);
    return report.summary.errors > 0 ? 1 : 0;
}

try {
    process.exitCode = await main();
} catch (error) {
    const exitCode = error instanceof EgolintJavascriptError ? error.exitCode : 4;
    console.error(`egolint-javascript: ${boundedText(error?.stack ?? error)}`);
    process.exitCode = exitCode;
}
