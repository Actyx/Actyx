import { platform } from 'os'
import settings from '../settings'
import { runLocalDocker } from '../src/ax/setup-projects/local-docker'
import { setupTestProjects } from '../src/ax/setup-projects/test-projects'

const setup = async (_config: Record<string, unknown>): Promise<void> => {
  console.log('Running Jest with local Docker only')
  await runLocalDocker(platform(), settings.localDocker.containerName)
  await setupTestProjects()
}

export default setup
