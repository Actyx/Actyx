/* eslint-disable @typescript-eslint/no-explicit-any */
import { Client, DefaultClientOpts } from '@actyx/os-sdk'
import { EC2 } from 'aws-sdk'
import NodeEnvironment from 'jest-environment-node'
import { CLI } from '../src/cli'
import { setupStubs } from '../src/stubs'
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

    // global objects must be serializable to copy into jest's test context, that's why we have to re-setup some things
    for (const node of axNodeSetup.nodes) {
      // Reuse the identity the node was set up with
      node.ax = await CLI.buildWithIdentityPath(node._private.axHost, node._private.axBinaryPath, node.ax.identityPath)

      const opts = DefaultClientOpts()
      opts.Endpoints.ConsoleService.BaseUrl = node._private.apiConsole
      opts.Endpoints.EventService.BaseUrl = node._private.apiEvent
      node.actyxOS = Client(opts)
    }

    this.global.stubs = await setupStubs()
  }

  async teardown(): Promise<void> {
    for (const node of (<MyGlobal>(<unknown>this.global)).axNodeSetup.thisTestEnvNodes || []) {
      await node._private.shutdown()
    }
    await super.teardown()
  }

  // eslint-disable-next-line @typescript-eslint/explicit-module-boundary-types
  runScript<T>(script: any): T | null {
    return super.runScript(script)
  }
}

export default MyEnvironment
