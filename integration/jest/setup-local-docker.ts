import { platform } from 'os'
import settings from '../settings'
import { runLocalDocker } from '../src/setup-projects/local-docker'
import { setupTestProjects } from '../src/setup-projects/test-projects'

const setup = async (_config: Record<string, unknown>): Promise<void> => {
  process.stdout.write('\n')
  console.log('Running Jest with local Docker only')

  await runLocalDocker(platform(), settings.localDocker.containerName)

  console.log('START WAITING ____________')
  await new Promise((res) => {
    setTimeout(() => res(), 6000)
  })
  console.log('CONTINUE ____________')

  await setupTestProjects()
}

export default setup
