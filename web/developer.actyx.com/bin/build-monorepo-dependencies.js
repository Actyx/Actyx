const execa = require("execa");
const path = require("path");
const fs = require("fs");

const npmInstallAndBuild = async (cwd) => {
  if (!fs.existsSync(path.join(cwd, "node_modules"))) {
    console.log(`npm:[${cwd}] installing dependencies ...`);
    try {
      await execa("npm", ["install"], {
        cwd,
        shell: true,
      });
    } catch (error) {
      console.log(`npm:[${cwd}] error installing dependencies (errors below)`);
      console.log(error);
      return;
    }
  } else {
    console.log(
      `npm:[${cwd}] dependencies already installed (found node_modules)`
    );
  }
  if (!fs.existsSync(path.join(cwd, "lib"))) {
    console.log(`npm:[${cwd}] building ...`);
    try {
      await execa("npm", ["run", "build"], {
        cwd,
        shell: true,
      });
    } catch (error) {
      console.log(`npm:[${cwd}] error building package (see errors below)`);
      console.log(error);
      return;
    }
  } else {
    console.log(`npm:[${cwd}] package already built (found lib)`);
  }
  console.log(`npm:[${cwd}] done!`);
};

const cargoBuild = async (cwd) => {
  console.log(`cargo:[${cwd}] build`);
  try {
    await execa("cargo", ["build", "--release"], {
      cwd,
      shell: true,
    });
  } catch (error) {
    console.log(`cargo:[${cwd}] error build (errors below)`);
    console.log(error);
    return;
  }
  console.log(`cargo:[${cwd}] done`);
};

(async () => {
  const npmDeps = [
    ["..", "..", "js", "sdk"],
    ["..", "..", "js", "pond"],
  ];
  const cargoDeps = [["..", "..", "rust", "release"]];
  Promise.all(
    []
      .concat(npmDeps.map((d) => npmInstallAndBuild(path.join(...d))))
      .concat(cargoDeps.map((d) => cargoBuild(path.join(...d))))
  );
})();
