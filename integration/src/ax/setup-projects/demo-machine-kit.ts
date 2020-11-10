import { gitClone, npmInstall, npmRun, TEMP_DIR } from './util'

export const demoMachineKitDirs = {
  demoMachineKit: `${TEMP_DIR}/DemoMachineKit`,
  dashboard: `${TEMP_DIR}/DemoMachineKit/src/dashboard`,
  erpSimulator: `${TEMP_DIR}/DemoMachineKit/src/erp-simulator`,
  wagoConnector: `${TEMP_DIR}/DemoMachineKit/src/wago-connector`,
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
