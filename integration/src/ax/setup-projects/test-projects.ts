import demoMachineKit from './demo-machine-kit'
import quickstart from './quickstart'
import { ensureDir } from 'fs-extra'

const setup = async (): Promise<void> => {
  try {
    await ensureDir('temp')
    await quickstart.setup()
    await demoMachineKit.setup()
  } catch (err) {
    console.error(err)
    process.exitCode = 1
    process.exit()
  }
}

const testProjects = {
  quickstart: {
    dirs: quickstart.dirs,
  },
  demoMachineKit: {
    dirs: demoMachineKit.dirs,
  },
  setup,
}

export default testProjects
