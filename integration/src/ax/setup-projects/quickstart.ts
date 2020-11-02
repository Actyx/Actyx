import { mkDir, gitClone, npmInstall, npmRun } from './util'

type Quickstart = () => Readonly<{
  dirSampleWebviewApp: string
  dirSampleDockerApp: string
  setup: () => Promise<string>
}>

const quickstart: Quickstart = () => {
  const dirQuickstart = 'temp/quickstart'
  const dirSampleWebviewApp = 'temp/quickstart/sample-webview-app'
  const dirSampleDockerApp = 'temp/quickstart/sample-docker-app'

  const npmRunBuild = npmRun('build')

  return {
    dirSampleWebviewApp,
    dirSampleDockerApp,

    async setup(): Promise<string> {
      console.log('Setup quickstart:')

      try {
        await mkDir(dirQuickstart)

        await gitClone('https://github.com/Actyx/quickstart.git', dirQuickstart)

        await npmInstall(dirSampleWebviewApp)
        await npmRunBuild(dirSampleWebviewApp)

        await npmInstall(dirSampleDockerApp)
        await npmRunBuild(dirSampleDockerApp)

        return Promise.resolve('quickstart ready!')
      } catch (err) {
        return Promise.reject(err)
      }
    },
  }
}

export default quickstart()
