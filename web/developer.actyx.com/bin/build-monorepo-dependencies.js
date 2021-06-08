const execa = require('execa')
const path = require('path')
const fs = require('fs')

const installAndBuild = async (cwd) => {
  if (!fs.existsSync(path.join(cwd, 'node_modules'))) {
    console.log(`[${cwd}] installing dependencies ...`)
    try {
      await execa('npm', ['install'], {
        cwd,
        shell: true,
      })
    } catch (error) {
      console.log(`[${cwd}] error installing dependencies (errors below)`)
      console.log(error)
      return
    }
  } else {
    console.log(`[${cwd}] dependencies already installed (found node_modules)`)
  }
  if (!fs.existsSync(path.join(cwd, 'lib'))) {
    console.log(`Building ${cwd} ...`)
    try {
      await execa('npm', ['run', 'build'], {
        cwd,
        shell: true,
      })
    } catch (error) {
      console.log(`[${cwd}] error building package (see errors below)`)
      console.log(error)
      return
    }
  } else {
    console.log(`[${cwd}] package already built (found lib)`)
  }
  console.log(`[${cwd}] done!`)
}

;(async () => {
  const deps = [
    ['..', '..', 'js', 'sdk'],
    ['..', '..', 'js', 'pond'],
  ]
  Promise.all(deps.map((d) => installAndBuild(path.join(...d))))
})()
