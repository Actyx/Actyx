const execa = require('execa')
const path = require('path')
const fs = require('fs')

const installAndBuild = async (cwd) => {
  if (!fs.existsSync(path.join(cwd, 'node_modules'))) {
    console.log(`Installing ${cwd} dependencies ...`)
    await execa('npm', ['install'], {
      cwd,
      shell: true,
    })
  } else {
    console.log(`Dependencies already installed (node_modules found)`)
  }
  if (!fs.existsSync(path.join(cwd, 'lib'))) {
    console.log(`Building ${cwd} ...`)
    await execa('npm', ['run', 'build'], {
      cwd,
      shell: true,
    })
  } else {
    console.log(`Package already build (lib found)`)
  }
  console.log(`Done!`)
}

;(async () => {
  const deps = ['pond', 'os-sdk']
  console.log(`Building monorepo dependencies (${deps.join(', ')})`)
  await installAndBuild(path.join('..', '..', 'js', deps[0]))
  await installAndBuild(path.join('..', '..', 'js', deps[1]))
})()
