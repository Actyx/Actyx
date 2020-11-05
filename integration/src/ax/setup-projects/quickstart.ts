import { gitClone, npmInstall, npmRun, TEMP_DIR, TestProject } from './util'

type Dirs = 'dirQuickstart' | 'dirSampleWebviewApp' | 'dirSampleDockerApp'

type Quickstart = TestProject<Dirs>

const quickstart = (): Quickstart => {
  const dirQuickstart = `${TEMP_DIR}/quickstart`
  const dirSampleWebviewApp = `${TEMP_DIR}/quickstart/sample-webview-app`
  const dirSampleDockerApp = `${TEMP_DIR}/quickstart/sample-docker-app`

  const npmRunBuild = npmRun('build')

  return {
    dirs: {
      dirQuickstart,
      dirSampleWebviewApp,
      dirSampleDockerApp,
    },

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
