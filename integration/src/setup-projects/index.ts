import execa from 'execa'
import { ensureDir } from 'fs-extra'
import { currentOS } from '../../jest/types'
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

export const getPipEnv = async (): Promise<string> => {
  if (currentOS() == 'macos') {
    return 'pipenv'
  } else {
    const base = (await execa.command('python -m site --user-base')).stdout
    return `${base}/bin/pipenv --python python`
  }
}

export const setupAnsible = async (): Promise<void> => {
  if (currentOS() == 'macos') {
    await setupAnsibleMac()
  } else {
    await setupAnsibleDebianish()
  }
  console.log('pipenv set up for ansible')
}

const setupAnsibleMac = async (): Promise<void> => {
  const res = await execa.command('pip install --user pipenv').catch((e) => e)
  if (res instanceof Error) {
    console.log('pip didn’t work, trying pip3: ', res.shortMessage)
    await execa.command('pip3 install --user pipenv')
  }
  const pipEnv = await getPipEnv()
  await execa.command(`${pipEnv} install`, { cwd: 'ansible' })
}

const setupAnsibleDebianish = async (): Promise<void> => {
  await execa.command('pip3 install --user pipenv')

  const res2 = await execa.command('pipenv install', { cwd: 'ansible' }).catch((e) => e)
  if (res2 instanceof Error) {
    console.log('pipenv not found, looking for local user python bin dir')
    const pipenv = await getPipEnv()
    await execa.command(`${pipenv} --site-packages install`, {
      cwd: 'ansible',
    })
  }
}
