;(async () => {
  const { execa } = await import('execa')
  const path = require('path')

  const npmInstallAndBuild = async (cwd) => {
    console.log(`npm:[${cwd}] installing dependencies ...`)
    try {
      await execa('npm', ['install'], { cwd })
    } catch (error) {
      console.log(`npm:[${cwd}] error installing dependencies (errors below)`)
      console.log(error)
      throw new Error('aborting build')
    }
    console.log(`npm:[${cwd}] building ...`)
    try {
      await execa('npm', ['run', 'build'], { cwd })
    } catch (error) {
      console.log(`npm:[${cwd}] error building package (see errors below)`)
      console.log(error)
      throw new Error('aborting build')
    }
    console.log(`npm:[${cwd}] done!`)
  }

  const cargoBuild = async (cwd) => {
    console.log(`cargo:[${cwd}] build`)
    try {
      await execa('cargo', ['build', '--release'], { cwd })
    } catch (error) {
      console.log(`cargo:[${cwd}] error build (errors below)`)
      console.log(error)
      throw new Error('aborting build')
    }
    console.log(`cargo:[${cwd}] done`)
  }

  const npmDeps = [
    ['..', '..', 'js', 'sdk'],
    ['..', '..', 'js', 'pond'],
  ]
  const cargoDeps = [['..', '..', 'rust', 'release']]
  Promise.all(
    []
      .concat(npmDeps.map((d) => npmInstallAndBuild(path.join(...d))))
      .concat(cargoDeps.map((d) => cargoBuild(path.join(...d)))),
  ).catch(() => process.exit(1))
})()
