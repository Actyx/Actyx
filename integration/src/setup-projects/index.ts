import execa from 'execa'

export const isDockerBuildxEnabled = async (): Promise<execa.ExecaChildProcess> =>
  await execa.command('docker buildx inspect')

export const getPipEnv = async (): Promise<string> => {
  return 'pipenv'
}
