import { ensureDir } from 'fs-extra'
import { demoMachineKitSetup } from './demo-machine-kit'
import { quickstartSetup } from './quickstart'

export const setupTestProjects = async (): Promise<void> => {
  const skipSetup = process.env.AX_INTEGRATION_SKIP_SETUP_TEST_PROJECTS === 'true'
  if (skipSetup) {
    console.log('Skip setup test projects')
    return
  }
  try {
    await ensureDir('temp')
    await quickstartSetup()
    await demoMachineKitSetup()
  } catch (err) {
    console.error(err)
    process.exitCode = 1
    process.exit()
  }
}
