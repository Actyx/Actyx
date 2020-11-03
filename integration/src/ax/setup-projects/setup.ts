import demoMachineKit from './demo-machine-kit'
import quickstart from './quickstart'
import { canSetupAfterTempDirHasBeenCreated } from './util'

export const setupProjects = async (): Promise<void> => {
  try {
    const canSetup = await canSetupAfterTempDirHasBeenCreated('temp')
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
