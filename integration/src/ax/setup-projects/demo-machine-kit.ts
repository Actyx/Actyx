import execa from 'execa'
import { remove, mkdirs, pathExists } from 'fs-extra'

type DemoMachineKit = () => Readonly<{
  dirDashboard: string
  dirErpSimulator: string
  dirWagoConnector: string
  setup: () => Promise<string>
}>

const demoMachineKit: DemoMachineKit = () => {
  const dirDemoMachineKit = 'temp/DemoMachineKit'
  const dirDashboard = 'temp/DemoMachineKit/dashboard'
  const dirErpSimulator = 'temp/DemoMachineKit/erp-simulator'
  const dirWagoConnector = 'temp/DemoMachineKit/wago-connector'

  return {
    dirDashboard,
    dirErpSimulator,
    dirWagoConnector,

    async setup(): Promise<string> {
      console.log('Setup DemoMachineKit:')

      try {
        const hasFolder = await pathExists(dirDemoMachineKit)
        if (hasFolder) {
          await remove(dirDemoMachineKit)
        }
        await mkdirs(dirDemoMachineKit)

        console.log('cloning repo...')
        await execa('git', [
          'clone',
          'https://github.com/Actyx/DemoMachineKit.git',
          dirDemoMachineKit,
        ])

        console.log('installing...')
        await execa('npm', ['install'], { cwd: dirDemoMachineKit })

        console.log('dashboard:')
        console.log('building...')
        await execa('npm', ['run', 'ui:dashboard:build'], { cwd: dirDemoMachineKit })

        console.log('erp-simulator:')
        console.log('building...')
        await execa('npm', ['run', 'ui:erp-simulator:build'], { cwd: dirDemoMachineKit })

        console.log('node:wago-connector:build:')
        console.log('building...')
        await execa('npm', ['run', 'node:wago-connector:build'], { cwd: dirDemoMachineKit })

        return Promise.resolve('DemoMachineKit ready!')
      } catch (err) {
        return Promise.reject(err)
      }
    },
  }
}

export default demoMachineKit()
