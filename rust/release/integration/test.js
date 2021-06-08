const path = require("path");
const fs = require("fs");
const { deepStrictEqual: assertEq } = require("assert");

const { execSync } = require("child_process");

const cwd = path.join(__dirname, "..", "test-repo");
const versionsFile = path.join(cwd, "versions");
const versionsIgnoreFile = path.join(cwd, "versions-ignore");
let cr = path.join(__dirname, "..", "target", "release", "cosmos-release");
if (process.platform === "win32") {
  cr += ".exe";
}

const createCwd = () => {
  if (fs.existsSync(cwd)) {
    fs.rmdirSync(cwd, { recursive: true });
  }
  fs.mkdirSync(cwd);
};

const checkBuild = () => {
  if (!fs.existsSync(cr)) {
    console.error(
      `unable to find cosmos-release target at ${cr} (did you \`cargo build --release\`?)`
    );
    process.exit(1);
  }
};

const appendLineToVersionsFile = (line) =>
  fs.appendFileSync(versionsFile, line + "\n");
const appendLineToVersionsIgnoreFile = (line) =>
  fs.appendFileSync(versionsIgnoreFile, line + "\n");
const getHeadHash = () =>
  execSync(`git rev-parse --short HEAD`, { encoding: "utf-8", cwd }).trim();
const commit = (message) =>
  execSync(`git commit --allow-empty -m "${message}"`, {
    encoding: "utf-8",
    cwd,
  });
const initGit = () => execSync("git init", { encoding: "utf-8", cwd });

const getCurrentVersion = (product) => {
  try {
    return execSync(`"${cr}" version ${product}`, {
      encoding: "utf-8",
      cwd,
    }).trim();
  } catch (e) {
    return e.stderr.trim();
  }
};
const getPastVersions = (product) =>
  execSync(`"${cr}" versions ${product}`, { encoding: "utf-8", cwd })
    .trim()
    .split(/\r?\n/);

// Tests
checkBuild();
createCwd();
initGit();

commit("initial release");
appendLineToVersionsFile(`actyx-0.0.0 ${getHeadHash()}`);
appendLineToVersionsFile(`pond-1.3.5 ${getHeadHash()}`);
appendLineToVersionsIgnoreFile("# Nothing");

assertEq(
  getCurrentVersion("actyx"),
  "Error: No new version found for actyx",
  "got current version w/o changes"
);
assertEq(getPastVersions("actyx"), ["0.0.0"], "got wrong past versions");

assertEq(
  getCurrentVersion("pond"),
  "Error: No new version found for pond",
  "got current version w/o changes"
);
assertEq(getPastVersions("pond"), ["1.3.5"], "got wrong past versions");

commit("fix(actyx): patch");

assertEq(getCurrentVersion("actyx"), "0.0.1", "got wrong current version");
assertEq(getPastVersions("actyx"), ["0.0.0"], "got wrong past versions");

commit("feat(actyx): minor");

assertEq(getCurrentVersion("actyx"), "0.1.0", "got wrong current version");
assertEq(getPastVersions("actyx"), ["0.0.0"], "got wrong past versions");

commit("break(actyx): major");

assertEq(getCurrentVersion("actyx"), "1.0.0", "got wrong current version");
assertEq(getPastVersions("actyx"), ["0.0.0"], "got wrong past versions");

// Ignore the last commit (breaking change), meaning we should not have a major
// bump anymore
appendLineToVersionsIgnoreFile(getHeadHash());

assertEq(getCurrentVersion("actyx"), "0.1.0", "got wrong current version");
assertEq(getPastVersions("actyx"), ["0.0.0"], "got wrong past versions");

appendLineToVersionsFile(`actyx-1.0.0 ${getHeadHash()}`);

assertEq(
  getCurrentVersion("actyx"),
  "Error: No new version found for actyx",
  "got current version w/o changes"
);
assertEq(
  getPastVersions("actyx"),
  ["1.0.0", "0.0.0"],
  "got wrong past versions"
);

assertEq(
  getCurrentVersion("pond"),
  "Error: No new version found for pond",
  "got current version w/o changes"
);
assertEq(getPastVersions("pond"), ["1.3.5"], "got wrong past versions");

console.log("success");
