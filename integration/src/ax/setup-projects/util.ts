import execa, { ExecaReturnValue } from 'execa'
import { remove } from 'fs-extra'

export const TEMP_DIR = 'temp'

export type TestProject<T extends string> = Readonly<{
  dirs: Record<T, string>
  setup: () => Promise<void>
}>

export const gitClone = async (url: string, path: string): Promise<ExecaReturnValue<string>> => {
  await remove(path)
  console.log(`git clone ${url} into ${path}`)
  return execa('git', ['clone', url, path])
}

export const npmInstall = (path: string): Promise<ExecaReturnValue<string>> => {
  console.log(`npm install into ${path}`)
  return execa('npm', ['install'], { cwd: path })
}

export const npmRun = (scriptName: string) => (path: string): Promise<ExecaReturnValue<string>> => {
  console.log(`npm run ${scriptName} into ${path}`)
  return execa('npm', ['run', scriptName], { cwd: path })
}
