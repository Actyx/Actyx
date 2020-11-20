import { gitClone, npmInstall, npmRun } from './util'

type Dirs = {
  demoMachineKit: string
  dashboard: string
  erpSimulator: string
  wagoConnector: string
}

export const demoMachineKitDirs = (tempDir: string): Dirs => ({
  demoMachineKit: `${tempDir}/DemoMachineKit`,
  dashboard: `${tempDir}/DemoMachineKit/src/dashboard`,
  erpSimulator: `${tempDir}/DemoMachineKit/src/erp-simulator`,
  wagoConnector: `${tempDir}/DemoMachineKit/src/wago-connector`,
})

export const demoMachineKitSetup = async (tempDir: string): Promise<void> => {
  const dirs = demoMachineKitDirs(tempDir)
  const npmRunBuild = (name: string) => npmRun(name)(dirs.demoMachineKit)

  console.log('Setup DemoMachineKit:')

  await gitClone('https://github.com/Actyx/DemoMachineKit.git', dirs.demoMachineKit)

  await npmInstall(dirs.demoMachineKit)
  await npmRunBuild('ui:dashboard:build')
  await npmRunBuild('ui:erp-simulator:build')
  await npmRunBuild('node:wago-connector:build')

  console.log('DemoMachineKit ready!')
}
