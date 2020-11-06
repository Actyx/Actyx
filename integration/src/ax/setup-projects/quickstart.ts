import { gitClone, npmInstall, npmRun, TEMP_DIR, TestProject } from './util'

type Dirs = 'quickstart' | 'sampleWebviewApp' | 'sampleDockerApp'

type Quickstart = TestProject<Dirs>

const quickstart = (): Quickstart => {
  const dirQuickstart = `${TEMP_DIR}/quickstart`
  const dirSampleWebviewApp = `${TEMP_DIR}/quickstart/sample-webview-app`
  const dirSampleDockerApp = `${TEMP_DIR}/quickstart/sample-docker-app`

  const npmRunBuild = npmRun('build')

  return {
    dirs: {
      quickstart: dirQuickstart,
      sampleWebviewApp: dirSampleWebviewApp,
      sampleDockerApp: dirSampleDockerApp,
    },

    async setup() {
      console.log('Setup quickstart:')

      await gitClone('https://github.com/Actyx/quickstart.git', dirQuickstart)

      await npmInstall(dirSampleWebviewApp)
      await npmRunBuild(dirSampleWebviewApp)

      await npmInstall(dirSampleDockerApp)
      await npmRunBuild(dirSampleDockerApp)

      console.log('quickstart ready!')
    },
  }
}

export default quickstart()
