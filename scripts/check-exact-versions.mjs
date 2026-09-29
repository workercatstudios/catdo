// Fails on any dependency (npm, Cargo, Gradle) that is not pinned to an exact version.
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";

const EXACT = /^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?(\+[0-9A-Za-z.-]+)?$/;
const SECTIONS = [
  "dependencies",
  "devDependencies",
  "optionalDependencies",
  "peerDependencies",
  "overrides",
  "resolutions",
];

function isExact(spec) {
  if (/^(workspace|link|file):/.test(spec)) return true;
  return EXACT.test(spec.replace(/^npm:.+@(?=[^@]+$)/, ""));
}

function* entries(section, path) {
  for (const [name, spec] of Object.entries(section ?? {})) {
    if (Object(spec) === spec) yield* entries(spec, `${path}.${name}`);
    else yield [`${path}.${name}`, spec];
  }
}

const files = execFileSync(
  "git",
  ["ls-files", "--cached", "--others", "--exclude-standard", "-z"],
  { encoding: "utf8" },
)
  .split("\0")
  .filter((file) =>
    /(^|\/)(package\.json|Cargo\.toml|[^/]+\.gradle(\.kts)?)$/.test(file),
  );

const CARGO_EXACT = /^=\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?(\+[0-9A-Za-z.-]+)?$/;
const GRADLE_EXACT = /^\d[0-9A-Za-z._-]*$/;

function checkCargo(file, text, failures) {
  let inDeps = false;
  let inDepTable = false;
  text.split("\n").forEach((line, index) => {
    const header = line.match(/^\s*\[(.+?)\]\s*$/);
    if (header) {
      const name = header[1];
      inDeps = /(^|\.)(dev-|build-)?dependencies$/.test(name);
      inDepTable = /(^|\.)(dev-|build-)?dependencies\./.test(name);
      return;
    }
    const match = inDepTable
      ? line.match(/^\s*version\s*=\s*"([^"]*)"/)
      : inDeps
        ? line.match(
            /^\s*[\w-]+\s*=\s*(?:"([^"]*)"|\{.*?\bversion\s*=\s*"([^"]*)")/,
          )
        : null;
    const spec = match?.[1] ?? match?.[2];
    if (spec !== undefined && !CARGO_EXACT.test(spec)) {
      failures.push(
        `${file}:${index + 1}: ${JSON.stringify(spec)} (use "=x.y.z")`,
      );
    }
  });
}

function checkGradle(file, text, failures) {
  text.split("\n").forEach((line, index) => {
    const specs = [
      ...[...line.matchAll(/"[\w.-]+:[\w.-]+:([^":]*)"/g)].map(
        (match) => match[1],
      ),
      ...[...line.matchAll(/\bversion\s*\(?\s*"([^"]*)"/g)].map(
        (match) => match[1],
      ),
    ];
    for (const spec of specs) {
      if (!GRADLE_EXACT.test(spec) || spec.startsWith("latest")) {
        failures.push(`${file}:${index + 1}: ${JSON.stringify(spec)}`);
      }
    }
  });
}

const failures = [];
for (const file of files) {
  if (file.endsWith("Cargo.toml")) {
    checkCargo(file, readFileSync(file, "utf8"), failures);
    continue;
  }
  if (/\.gradle(\.kts)?$/.test(file)) {
    checkGradle(file, readFileSync(file, "utf8"), failures);
    continue;
  }
  const manifest = JSON.parse(readFileSync(file, "utf8"));
  for (const [path, section] of [
    ...SECTIONS.map((name) => [name, manifest[name]]),
    ["pnpm.overrides", manifest.pnpm?.overrides],
  ]) {
    for (const [key, spec] of entries(section, path)) {
      if (!isExact(spec))
        failures.push(`${file}: ${key} = ${JSON.stringify(spec)}`);
    }
  }
}

if (failures.length > 0) {
  console.error(
    `Ranged versions found. Pin each to the exact version in its lockfile:\n`,
  );
  console.error(failures.map((line) => `  ${line}`).join("\n"));
  process.exit(1);
}
console.log(`Checked ${files.length} manifest(s): all versions are exact.`);
