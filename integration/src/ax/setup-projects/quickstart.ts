import { gitClone, npmInstall, npmRun, TEMP_DIR } from './util'

type Quickstart = () => Readonly<{
  dirQuickstart: string
  dirSampleWebviewApp: string
  dirSampleDockerApp: string
  setup: () => Promise<string>
}>

const quickstart: Quickstart = () => {
  const dirQuickstart = `${TEMP_DIR}/quickstart`
  const dirSampleWebviewApp = `${TEMP_DIR}/quickstart/sample-webview-app`
  const dirSampleDockerApp = `${TEMP_DIR}/quickstart/sample-docker-app`

  const npmRunBuild = npmRun('build')

  return {
    dirQuickstart,
    dirSampleWebviewApp,
    dirSampleDockerApp,

    async setup(): Promise<string> {
      console.log('Setup quickstart:')

      await gitClone('https://github.com/Actyx/quickstart.git', dirQuickstart)

      await npmInstall(dirSampleWebviewApp)
      await npmRunBuild(dirSampleWebviewApp)

      await npmInstall(dirSampleDockerApp)
      await npmRunBuild(dirSampleDockerApp)

      return 'quickstart ready!'
    },
  }
}

export default quickstart()
