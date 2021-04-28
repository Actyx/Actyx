/* eslint-disable @typescript-eslint/no-explicit-any */
import { Client, DefaultClientOpts } from '@actyx/os-sdk'
import { EC2 } from 'aws-sdk'
import NodeEnvironment from 'jest-environment-node'
import { CLI } from '../src/cli'
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
     * That's why we have to re-setup some things re-creating functions.
     */
    for (const node of axNodeSetup.nodes) {
      // Reuse the identity the node was set up with
      const ax = await CLI.buildWithIdentityPath(
        node._private.axHost,
        node._private.axBinaryPath,
        node.ax.identityPath,
      )

      const opts = DefaultClientOpts()
      opts.Endpoints.EventService.BaseUrl = node._private.httpApiOrigin
      // TODO: use ts-sdk v2
      const httpApiClient = Client(opts)

      /** Objects that have functions */
      // eslint-disable-next-line @typescript-eslint/ban-ts-comment
      // @ts-ignore
      node.ax = ax
      // eslint-disable-next-line @typescript-eslint/ban-ts-comment
      // @ts-ignore
      node.httpApiClient = httpApiClient
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
