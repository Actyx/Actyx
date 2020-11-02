import execa from 'execa'
import { mkdirs, pathExists, remove } from 'fs-extra'

export const mkDir = async (path: string): Promise<string> => {
  try {
    const hasDir = await pathExists(path)
    if (hasDir) {
      await remove(path)
    }
    await mkdirs(path)
    const message = `dir created ${path}`
    console.log(message)
    return Promise.resolve(message)
  } catch (error) {
    return Promise.reject(error)
  }
}

export const gitClone = (url: string, path: string): execa.ExecaChildProcess<string> => {
  console.log(`git clone ${url} into ${path}`)
  return execa('git', ['clone', url, path])
}

export const npmInstall = (path: string): execa.ExecaChildProcess<string> => {
  console.log(`npm install into ${path}`)
  return execa('npm', ['install'], { cwd: path })
}

export const npmRun = (scriptName: string) => (path: string): execa.ExecaChildProcess<string> => {
  console.log(`npm run ${scriptName} into ${path}`)
  return execa('npm', ['run', scriptName], { cwd: path })
}
