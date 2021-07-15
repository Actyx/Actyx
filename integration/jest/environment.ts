/* eslint-disable @typescript-eslint/no-explicit-any */
import { EC2 } from 'aws-sdk'
import NodeEnvironment from 'jest-environment-node'
import { CLI } from '../src/cli'
import { mkExecute } from '../src/infrastructure'
import { MyGlobal } from './setup'

class MyEnvironment extends NodeEnvironment {
  // eslint-disable-next-line @typescript-eslint/explicit-module-boundary-types
  constructor(config: any, _context: any) {
    super(config)
  }

  async setup(): Promise<void> {
    await super.setup()

    const axNodeSetup = (<MyGlobal>(<unknown>this.global)).axNodeSetup
    ;(<MyGlobal>global).axNodeSetup = axNodeSetup
    axNodeSetup.ec2 = new EC2({ region: 'eu-central-1' })
    axNodeSetup.thisTestEnvNodes = []

    /**
     * Global objects must be serializable to copy into jest's test context.
     * That's why we have to re-setup some things re-creating functions.  Note:
     * If any code relies on `instanceof` comparison (like for example rxjs
     * does), this won't work, as this code is executed not within the new VM
     * (https://github.com/facebook/jest/issues/7246).
     */
    for (const node of axNodeSetup.nodes) {
      // Reuse the identity the node was set up with
      const ax = await CLI.buildWithIdentityPath(
        `${node._private.hostname}:${node._private.adminPort}`,
        node._private.axBinaryPath,
        node.ax.identityPath,
      )
      node.target.execute = mkExecute(node.target.os, node.target.kind)
      if (node.target._private.executeInContainerPrefix !== undefined) {
        node.target.executeInContainer = (script: string) =>
          node.target.execute(`${node.target._private.executeInContainerPrefix}${script}`, [], {})
      }

      /** Objects that have functions */
      // eslint-disable-next-line @typescript-eslint/ban-ts-comment
      // @ts-ignore
      node.ax = ax
    }
  }

  async teardown(): Promise<void> {
    for (const node of (<MyGlobal>(<unknown>this.global)).axNodeSetup.thisTestEnvNodes || []) {
      await node._private.shutdown()
    }
    await super.teardown()
  }
}

export default MyEnvironment
