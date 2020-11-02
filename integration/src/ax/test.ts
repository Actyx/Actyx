import { stubNodeHostUnreachable } from '../stubs'
import quickstart from './setup-projects/quickstart'

const test = async () => {
  const response = await stubNodeHostUnreachable.ax.Apps.Validate(quickstart.dirSampleWebviewApp)
  console.log(JSON.stringify(response))
}

test()
  .then(() => console.log('all ok'))
  .catch(console.error)
