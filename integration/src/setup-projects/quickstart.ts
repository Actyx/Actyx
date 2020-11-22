import { gitClone, npmInstall, npmRun } from './util'

type Dirs = {
  quickstart: string
  sampleWebviewApp: string
  sampleDockerApp: string
}

export const quickstartDirs = (tempDir: string): Dirs => ({
  quickstart: `${tempDir}/quickstart`,
  sampleWebviewApp: `${tempDir}/quickstart/sample-webview-app`,
  sampleDockerApp: `${tempDir}/quickstart/sample-docker-app`,
})

export const quickstartSetup = async (tempDir: string): Promise<void> => {
  const dirs = quickstartDirs(tempDir)

  const npmRunBuild = npmRun('build')

  console.log('Setup quickstart:')

  await gitClone('https://github.com/Actyx/quickstart.git', dirs.quickstart)

  await npmInstall(dirs.sampleWebviewApp)
  await npmRunBuild(dirs.sampleWebviewApp)

  await npmInstall(dirs.sampleDockerApp)
  await npmRunBuild(dirs.sampleDockerApp)

  await npmRun('build:image')(dirs.sampleDockerApp)
  await npmRun('build:image:aarch64')(dirs.sampleDockerApp)

  console.log('quickstart ready!')
}
