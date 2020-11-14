import { gitClone, npmInstall, npmRun } from './util'
import settings from '../../settings'

const { tempDir } = settings.testProjects

export const quickstartDirs = {
  quickstart: `${tempDir}/quickstart`,
  sampleWebviewApp: `${tempDir}/quickstart/sample-webview-app`,
  sampleDockerApp: `${tempDir}/quickstart/sample-docker-app`,
}

export const quickstartSetup = async (): Promise<void> => {
  const npmRunBuild = npmRun('build')

  console.log('Setup quickstart:')

  await gitClone('https://github.com/Actyx/quickstart.git', quickstartDirs.quickstart)

  await npmInstall(quickstartDirs.sampleWebviewApp)
  await npmRunBuild(quickstartDirs.sampleWebviewApp)

  await npmInstall(quickstartDirs.sampleDockerApp)
  await npmRunBuild(quickstartDirs.sampleDockerApp)

  await npmRun('build:image')(quickstartDirs.sampleDockerApp)

  console.log('quickstart ready!')
}
