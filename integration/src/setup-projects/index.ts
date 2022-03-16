import { execa, ExecaChildProcess } from 'execa'

export const isDockerBuildxEnabled = async (): Promise<ExecaChildProcess> =>
  await execa('docker buildx inspect')
