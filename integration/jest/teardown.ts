/* eslint-disable @typescript-eslint/no-non-null-assertion */
import { deleteKey } from '../src/runner/aws'
import { MyGlobal } from './setup'

const teardown = async (_config: Record<string, unknown>): Promise<void> => {
  const axNodeSetup = (<MyGlobal>global).axNodeSetup

  process.stdout.write('****\n\nSHUTTING DOWN\n\n')

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

  for (const name in axNodeSetup.logs) {
    process.stdout.write(`\n****\nlogs for node ${name}\n****\n\n`)
    for (const entry of axNodeSetup.logs[name]) {
      process.stdout.write(`${entry.time.toISOString()} ${entry.line}\n`)
    }
  }
  process.stdout.write('\n')
}

export default teardown
