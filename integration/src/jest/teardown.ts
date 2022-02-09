/* eslint-disable @typescript-eslint/no-non-null-assertion */
import { deleteKey } from '../infrastructure/aws'
import { printTarget } from '../infrastructure/types'
import { MyGlobal } from './setup'

const teardown = async (_config: Record<string, unknown>): Promise<void> => {
  const axNodeSetup = (<MyGlobal>global).axNodeSetup

  process.stdout.write('****\n\nSHUTTING DOWN\n\n')

  if (typeof axNodeSetup.ec2 !== 'undefined' && typeof axNodeSetup.key !== 'undefined') {
    await deleteKey(axNodeSetup.ec2, axNodeSetup.key.keyName).catch(console.error)
  }

  if (axNodeSetup.settings.keepNodesRunning) {
    console.log('as per your request: NOT terminating instances')
    if (typeof axNodeSetup.key !== 'undefined') {
      console.log('private SSH key file:')
      console.log(axNodeSetup.key.privateKey)
    }
    console.log('Node list:')
    for (const n of axNodeSetup.nodes) {
      console.log(`    ${n.name} (${printTarget(n.target)})`)
      console.log('      hostname:', n._private.hostname)
      console.log('      api port:', n._private.apiPort)
      console.log('      admin port : %s (key %s)', n._private.adminPort, n.ax.identityPath)
    }
    process.stdout.write('\n')
    console.log('process will not end since SSH forwarding remains active')
    console.log('please press ctrl-C when done (and shut down those instances!)')
  } else {
    await new Promise<void>((res, rej) => {
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

  process.stdout.write('teardown complete\n')
}

export default teardown