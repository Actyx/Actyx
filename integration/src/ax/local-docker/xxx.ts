// REMOVE ME
import { runLocalDocker, stopLocalDocker } from './local-docker'
import { platform } from 'os'

const runTests = (): Promise<void> => {
  return new Promise((res) => {
    // add tests here
    setTimeout(res, 3000)
  })
}

runLocalDocker(platform(), 'temp1')
  .then(() => console.log('Docker container started. Start tests.'))
  .then(runTests)
  .then(() => console.log('Tests executed. Stop docker container.'))
  .then(stopLocalDocker)
  .then(() => console.log('Docker container stopped'))
  .catch(console.error)
