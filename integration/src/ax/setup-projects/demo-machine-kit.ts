import { gitClone, npmInstall, npmRun, TEMP_DIR, TestProject } from './util'

type Dirs = 'dirDashboard' | 'dirErpSimulator' | 'dirWagoConnector'

type DemoMachineKit = TestProject<Dirs>

const demoMachineKit = (): DemoMachineKit => {
  const dirDemoMachineKit = `${TEMP_DIR}/DemoMachineKit`
  const dirDashboard = `${TEMP_DIR}/DemoMachineKit/src/dashboard`
  const dirErpSimulator = `${TEMP_DIR}/DemoMachineKit/src/erp-simulator`
  const dirWagoConnector = `${TEMP_DIR}/DemoMachineKit/src/wago-connector`

  const npmRunBuild = (name: string) => npmRun(name)(dirDemoMachineKit)

  return {
    dirs: {
      dirDashboard,
      dirErpSimulator,
      dirWagoConnector,
    },

    async setup(): Promise<string> {
      console.log('Setup DemoMachineKit:')

      await gitClone('https://github.com/Actyx/DemoMachineKit.git', dirDemoMachineKit)

      await npmInstall(dirDemoMachineKit)
      await npmRunBuild('ui:dashboard:build')
      await npmRunBuild('ui:erp-simulator:build')
      await npmRunBuild('node:wago-connector:build')

      return 'DemoMachineKit ready!'
    },
  }
}

export default demoMachineKit()
