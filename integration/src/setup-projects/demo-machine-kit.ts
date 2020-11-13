import { gitClone, npmInstall, npmRun } from './util'
import settings from '../../settings'

const { tempDir } = settings.testProjects

export const demoMachineKitDirs = {
  demoMachineKit: `${tempDir}/DemoMachineKit`,
  dashboard: `${tempDir}/DemoMachineKit/src/dashboard`,
  erpSimulator: `${tempDir}/DemoMachineKit/src/erp-simulator`,
  wagoConnector: `${tempDir}/DemoMachineKit/src/wago-connector`,
}

export const demoMachineKitSetup = async (): Promise<void> => {
  const npmRunBuild = (name: string) => npmRun(name)(demoMachineKitDirs.demoMachineKit)

  console.log('Setup DemoMachineKit:')

  await gitClone('https://github.com/Actyx/DemoMachineKit.git', demoMachineKitDirs.demoMachineKit)

  await npmInstall(demoMachineKitDirs.demoMachineKit)
  await npmRunBuild('ui:dashboard:build')
  await npmRunBuild('ui:erp-simulator:build')
  await npmRunBuild('node:wago-connector:build')

  console.log('DemoMachineKit ready!')
}
