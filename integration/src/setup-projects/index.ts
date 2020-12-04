import execa from 'execa'
import { ensureDir } from 'fs-extra'
import { demoMachineKitSetup } from './demo-machine-kit'
import { quickstartSetup } from './quickstart'

export const isDockerBuildxEnabled = async (): Promise<execa.ExecaChildProcess> =>
  await execa.command('docker buildx inspect')

export const setupTestProjects = async (tempDir: string): Promise<void> => {
  await isDockerBuildxEnabled().catch((err) => {
    throw `Docker Buildx is required! \n${err}`
  })
  await ensureDir(tempDir)
  await quickstartSetup(tempDir)
  await demoMachineKitSetup(tempDir)
}
