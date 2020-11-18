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

    const axNodeSetup = (<MyGlobal>global).axNodeSetup
    axNodeSetup.ec2 = new EC2({ region: 'eu-central-1' })
    axNodeSetup.envNodes = []

    for (const node of axNodeSetup.nodes) {
      node.ax = new CLI(node._private.axHost, node._private.axBinaryPath)

      const opts = DefaultClientOpts()
      opts.Endpoints.ConsoleService.BaseUrl = node._private.apiConsole
      opts.Endpoints.EventService.BaseUrl = node._private.apiEvent
      node.actyxOS = Client(opts)
    }
  }

  async teardown(): Promise<void> {
    for (const node of (<MyGlobal>global).axNodeSetup.envNodes || []) {
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
