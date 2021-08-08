import execa from 'execa'

export const isDockerBuildxEnabled = async (): Promise<execa.ExecaChildProcess> =>
  await execa.command('docker buildx inspect')
