import settings from '../settings'
import { stopLocalDocker } from '../src/setup-projects/local-docker'

const teardown = async (_config: Record<string, unknown>): Promise<void> => {
  process.stdout.write('****\n\nSHUTTING DOWN\n\n')

  await stopLocalDocker(settings.localDocker.containerName)
}
export default teardown
