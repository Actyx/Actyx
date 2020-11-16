/* eslint-disable @typescript-eslint/no-non-null-assertion */
import settings from '../settings'
import { stopLocalDocker } from '../src/setup-projects/local-docker'
import { deleteKey } from '../src/runner/aws'
import { printTarget } from '../src/runner/types'
import { MyGlobal } from './setup'

const teardown = async (_config: Record<string, unknown>): Promise<void> => {
  const axNodeSetup = (<MyGlobal>global).axNodeSetup

  process.stdout.write('****\n\nSHUTTING DOWN\n\n')

  await stopLocalDocker(settings.localDocker.containerName)

  if (axNodeSetup.keepNodesRunning) {
    console.log('as per your request: NOT terminating instances')
    console.log('private SSH key file:')
    console.log(axNodeSetup.key.privateKey)
    console.log('Node list:')
    for (const n of axNodeSetup.nodes) {
      console.log(`  ${n.name} (${printTarget(n.target)})`)
      console.log('    console:', n._private.apiConsole)
      console.log('    event:', n._private.apiEvent)
      console.log('    pond:', n._private.apiPond)
    }
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
    await deleteKey(axNodeSetup.ec2, axNodeSetup.key.keyName).catch(console.error)
  }

  for (const name in axNodeSetup.logs) {
    process.stdout.write(`\n****\nlogs for node ${name}\n****\n\n`)
    for (const entry of axNodeSetup.logs[name]) {
      process.stdout.write(`${entry.time.toISOString()} ${entry.line}\n`)
    }
  }
  process.stdout.write('\n')

  if (axNodeSetup.keepNodesRunning) {
    console.log('process will not end since SSH forwarding remains active')
    console.log('please press ctrl-C when done (and shut down those instances!)')
  }
}

export default teardown
