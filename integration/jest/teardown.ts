/* eslint-disable @typescript-eslint/no-non-null-assertion */
import { deleteKey } from '../src/runner/aws'
import { MyGlobal } from './setup'

const teardown = async (_config: Record<string, unknown>): Promise<void> => {
  const nodeSetup = (<MyGlobal>global).nodeSetup

  for (const name in nodeSetup.logs) {
    process.stdout.write(`****\nlogs for node ${name}\n****\n`)
    for (const entry of nodeSetup.logs[name]) {
      process.stdout.write(`${entry.time.toISOString()} ${entry.line}\n`)
    }
  }

  process.stdout.write('****\n\nSHUTTING DOWN\n\n')

  try {
    await new Promise((res, rej) => {
      Promise.all(
        nodeSetup.nodes.map((node) =>
          node.shutdown().catch((err) => {
            console.log(`node ${node.name} error while shutting down: ${err}`)
          }),
        ),
      ).then(() => res())
      setTimeout(() => rej(new Error('timeout waiting for shutdown')), 10_000)
    })
  } catch (e) {
    console.error(e)
  }
  await deleteKey(nodeSetup.ec2, nodeSetup.key.keyName)
}

export default teardown
