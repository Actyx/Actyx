import { ensureDir } from 'fs-extra'
import { demoMachineKitSetup } from './demo-machine-kit'
import { quickstartSetup } from './quickstart'

export const setupTestProjects = async (tempDir: string): Promise<void> => {
  await ensureDir(tempDir)
  await quickstartSetup(tempDir)
  await demoMachineKitSetup(tempDir)
}
