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
    axNodeSetup.ec2 = new EC2({ region: 'eu-central-1' })
    axNodeSetup.thisTestEnvNodes = []

    for (const node of axNodeSetup.nodes) {
      node.ax = await CLI.build(node._private.axHost, node._private.axBinaryPath)

      const opts = DefaultClientOpts()
      opts.Endpoints.ConsoleService.BaseUrl = node._private.apiConsole
      opts.Endpoints.EventService.BaseUrl = node._private.apiEvent
      node.actyxOS = Client(opts)
    }
    ;(<MyGlobal>global).axNodeSetup = axNodeSetup

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
