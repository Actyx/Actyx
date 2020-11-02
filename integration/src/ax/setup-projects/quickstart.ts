import execa from 'execa'
import { remove, mkdirs, pathExists } from 'fs-extra'

type Quickstart = () => Readonly<{
  dirSampleWebviewApp: string
  dirSampleDockerApp: string
  setup: () => Promise<string>
}>

const quickstart: Quickstart = () => {
  const dirTemp = 'temp'
  const dirQuickstart = 'temp/quickstart'
  const dirSampleWebviewApp = 'temp/quickstart/sample-webview-app'
  const dirSampleDockerApp = 'temp/quickstart/sample-docker-app'

  return {
    dirSampleWebviewApp,
    dirSampleDockerApp,

    async setup(): Promise<string> {
      console.log('Setup quickstart:')

      try {
        const hasFolder = await pathExists(dirTemp)
        if (hasFolder) {
          await remove(dirTemp)
        }
        await mkdirs(dirTemp)

        console.log('cloning repo...')
        await execa('git', ['clone', 'https://github.com/Actyx/quickstart.git', dirQuickstart])

        console.log('sample-webview-app:')
        console.log('installing...')
        await execa('npm', ['install'], { cwd: dirSampleWebviewApp })

        console.log('building...')
        await execa('npm', ['run', 'build'], { cwd: dirSampleWebviewApp })

        console.log('sample-docker-app:')
        console.log('installing...')
        await execa('npm', ['install'], { cwd: dirSampleDockerApp })

        console.log('building...')
        await execa('npm', ['run', 'build:image'], { cwd: dirSampleDockerApp })

        return Promise.resolve('quickstart ready!')
      } catch (err) {
        return Promise.reject(err)
      }
    },
  }
}

export default quickstart()
