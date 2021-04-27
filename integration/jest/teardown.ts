/* eslint-disable @typescript-eslint/no-non-null-assertion */
import { deleteKey } from '../src/infrastructure/aws'
import { printTarget } from '../src/infrastructure/types'
import { MyGlobal } from './setup'

const teardown = async (_config: Record<string, unknown>): Promise<void> => {
  const axNodeSetup = (<MyGlobal>global).axNodeSetup

  process.stdout.write('****\n\nSHUTTING DOWN\n\n')

  await deleteKey(axNodeSetup.ec2, axNodeSetup.key.keyName).catch(console.error)

  if (axNodeSetup.settings.keepNodesRunning) {
    console.log('as per your request: NOT terminating instances')
    console.log('private SSH key file:')
    console.log(axNodeSetup.key.privateKey)
    console.log('Node list:')
    for (const n of axNodeSetup.nodes) {
      console.log(`    ${n.name} (${printTarget(n.target)})`)
      console.log('    http api origin:', n._private.httpApiOrigin)
      console.log('    pond:', n._private.apiPond)
    }
    process.stdout.write('\n')
    console.log('process will not end since SSH forwarding remains active')
    console.log('please press ctrl-C when done (and shut down those instances!)')
  } else {
    await new Promise((res, rej) => {
      Promise.all(
        axNodeSetup.nodes.map((node) =>
          node._private.shutdown().catch((err) => {
            console.log(`node ${node.name} error while shutting down: ${err}`)
          }),
        ),
      ).then(() => res())
      setTimeout(() => rej(new Error('timeout waiting for shutdown')), 10_000)
    }).catch(console.error)
  }

  process.stdout.write('\n')
}

export default teardown
