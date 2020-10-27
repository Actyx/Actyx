import execa from 'execa'
import { remove, mkdirs, pathExists } from 'fs-extra'
// import { exists } from '../util'

export const setup = {
  quickstart: {
    sameWebviewApp: {
      getReady: async (): Promise<string> => {
        console.log('get ready quickstart')

        const DIR = 'temp'
        const DIR_QUICKSTART = `${DIR}/quickstart`
        const DIR_SAMPLE_WEBVIEW_APP = `${DIR_QUICKSTART}/sample-webview-app`
        // const AX_MANIFEST = `${DIR_SAMPLE_WEBVIEW_APP}/ax-manifest.yml`

        try {
          const hasFolder = await pathExists(DIR)
          if (hasFolder) {
            await remove(DIR)
          }
          await mkdirs(DIR)

          console.log('cloning repo...')
          await execa('git', ['clone', 'https://github.com/Actyx/quickstart.git', DIR_QUICKSTART])

          console.log('installing...')
          await execa('npm', ['install'], { cwd: DIR_SAMPLE_WEBVIEW_APP })

          console.log('building...')
          await execa('npm', ['run', 'build'], { cwd: DIR_SAMPLE_WEBVIEW_APP })

          // await execa(`ax`, [`-j`, `apps`, `validate`, AX_MANIFEST])
          return Promise.resolve('installed and builded')
          // await remove(TEMP_DIR)
        } catch (err) {
          return Promise.reject(err)
        }
      },
    },
  },
}

setup.quickstart.sameWebviewApp.getReady().then(console.log).then(console.error)
