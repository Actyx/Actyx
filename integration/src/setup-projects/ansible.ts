import { currentOS } from '../../jest/types'
import execa from 'execa'
import { getPipEnv } from '.'

const getVersion = (str: string): [number, number, number] => {
  const version = str.match(/(?:(\d+)\.)?(?:(\d+)\.)?(\*|\d+)/) // example: "Python 2.7.16" => ['2.7.16', 2, 7, 16]
  if (version) {
    const major = Number(version[1])
    const minor = Number(version[2])
    const patch = Number(version[3])
    return [major, minor, patch]
  } else {
    throw `error: could not parse binary version for input string: ${str}`
  }
}

const checkBinaryInstalled = async (binaryName: string) => {
  try {
    await execa.command(`which ${binaryName}`)
  } catch (err) {
    console.error(err)
    throw `${binaryName} is required, please intall the latest version`
  }
}

const checkBinaryVersion = async (
  binaryName: string,
  predicateIsValidVersion: (major: number, minor: number, patch: number) => boolean,
) => {
  const { stdout } = await execa.command(`${binaryName} --version`)
  if (stdout && stdout.length > 0) {
    const isValid = predicateIsValidVersion(...getVersion(stdout))
    if (isValid) {
      console.log(`${binaryName} is currently installed with the right version`)
    } else {
      throw `${binaryName} version is not valid, please intall the latest version`
    }
  } else {
    throw `error: cannot read version for ${binaryName}`
  }
}

const checkPrerequisites = async () => {
  await checkBinaryInstalled('python')
  await checkBinaryVersion('python', (major, minor) => major >= 3 && minor >= 7)

  await checkBinaryInstalled('pipenv')
  await checkBinaryVersion('pipenv', (major) => major >= 2020)
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

  await execa.command('pipenv install', { cwd: 'ansible' })
}

export const setupAnsible = async (): Promise<void> => {
  try {
    await checkPrerequisites()

    if (currentOS() == 'macos') {
      await setupAnsibleMac()
    } else {
      await setupAnsibleDebianish()
    }
    console.log('pipenv set up for ansible')
  } catch (err) {
    console.error(err)
  }
}
