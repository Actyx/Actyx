/* eslint-disable @typescript-eslint/no-non-null-assertion */
import { deleteKey } from '../src/runner/aws'
import { MyGlobal } from './setup'

const teardown = async (_config: Record<string, unknown>): Promise<void> => {
  const nodeSetup = (<MyGlobal>global).nodeSetup
  for (const node of nodeSetup.nodes || []) {
    node.shutdown()
  }
  await deleteKey(nodeSetup.ec2!, nodeSetup.key!.keyName)
}

export default teardown
