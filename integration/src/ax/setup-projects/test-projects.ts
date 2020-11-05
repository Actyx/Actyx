import demoMachineKit from './demo-machine-kit'
import quickstart from './quickstart'
import { canSetupAfterRemoveOrCreateTempDir } from './util'

const setup = async (): Promise<void> => {
  try {
    const canSetup = await canSetupAfterRemoveOrCreateTempDir('temp')
    if (canSetup) {
      const quickstartStatusMessage = await quickstart.setup()
      console.log(quickstartStatusMessage)

      const demoMachineKitStatusMessage = await demoMachineKit.setup()
      console.log(demoMachineKitStatusMessage)
    } else {
      console.log('test projects are already setup')
    }
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
