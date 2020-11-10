import { gitClone, npmInstall, npmRun, TEMP_DIR } from './util'

export const quickstartDirs = {
  quickstart: `${TEMP_DIR}/quickstart`,
  sampleWebviewApp: `${TEMP_DIR}/quickstart/sample-webview-app`,
  sampleDockerApp: `${TEMP_DIR}/quickstart/sample-docker-app`,
}

export const quickstartSetup = async (): Promise<void> => {
  const npmRunBuild = npmRun('build')

  console.log('Setup quickstart:')

  await gitClone('https://github.com/Actyx/quickstart.git', quickstartDirs.quickstart)

  await npmInstall(quickstartDirs.sampleWebviewApp)
  await npmRunBuild(quickstartDirs.sampleWebviewApp)

  await npmInstall(quickstartDirs.sampleDockerApp)
  await npmRunBuild(quickstartDirs.sampleDockerApp)

  console.log('quickstart ready!')
}
