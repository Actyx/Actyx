import { mkDir, gitClone, npmInstall, npmRun } from './util'

type DemoMachineKit = () => Readonly<{
  dirDashboard: string
  dirErpSimulator: string
  dirWagoConnector: string
  setup: () => Promise<string>
}>

const demoMachineKit: DemoMachineKit = () => {
  const dirDemoMachineKit = 'temp/DemoMachineKit'
  const dirDashboard = 'temp/DemoMachineKit/src/dashboard'
  const dirErpSimulator = 'temp/DemoMachineKit/src/erp-simulator'
  const dirWagoConnector = 'temp/DemoMachineKit/src/wago-connector'

  const npmRunBuild = (name: string) => npmRun(name)(dirDemoMachineKit)

  return {
    dirDashboard,
    dirErpSimulator,
    dirWagoConnector,

    async setup(): Promise<string> {
      console.log('Setup DemoMachineKit:')

      try {
        await mkDir(dirDemoMachineKit)

        await gitClone('https://github.com/Actyx/DemoMachineKit.git', dirDemoMachineKit)

        await npmInstall(dirDemoMachineKit)
        await npmRunBuild('ui:dashboard:build')
        await npmRunBuild('ui:erp-simulator:build')
        await npmRunBuild('node:wago-connector:build')

        return Promise.resolve('DemoMachineKit ready!')
      } catch (err) {
        return Promise.reject(err)
      }
    },
  }
}

export default demoMachineKit()
