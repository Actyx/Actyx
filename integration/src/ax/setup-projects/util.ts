import execa from 'execa'
import { mkdirs, pathExists } from 'fs-extra'

export const TEMP_DIR = 'temp'

export const canSetupAfterRemoveOrCreateTempDir = async (path: string): Promise<boolean> => {
  const hasTempDir = await pathExists(path)
  if (!hasTempDir) {
    await mkdirs(path)
    return true
  }
  return false
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
