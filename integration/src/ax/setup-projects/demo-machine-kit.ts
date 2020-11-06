import { gitClone, npmInstall, npmRun, TEMP_DIR, TestProject } from './util'

type Dirs = 'dashboard' | 'erpSimulator' | 'wagoConnector'

type DemoMachineKit = TestProject<Dirs>

const demoMachineKit = (): DemoMachineKit => {
  const dirDemoMachineKit = `${TEMP_DIR}/DemoMachineKit`
  const dirDashboard = `${TEMP_DIR}/DemoMachineKit/src/dashboard`
  const dirErpSimulator = `${TEMP_DIR}/DemoMachineKit/src/erp-simulator`
  const dirWagoConnector = `${TEMP_DIR}/DemoMachineKit/src/wago-connector`

  const npmRunBuild = (name: string) => npmRun(name)(dirDemoMachineKit)

  return {
    dirs: {
      dashboard: dirDashboard,
      erpSimulator: dirErpSimulator,
      wagoConnector: dirWagoConnector,
    },

    async setup() {
      console.log('Setup DemoMachineKit:')

      await gitClone('https://github.com/Actyx/DemoMachineKit.git', dirDemoMachineKit)

      await npmInstall(dirDemoMachineKit)
      await npmRunBuild('ui:dashboard:build')
      await npmRunBuild('ui:erp-simulator:build')
      await npmRunBuild('node:wago-connector:build')

      console.log('DemoMachineKit ready!')
    },
  }
}

export default demoMachineKit()
