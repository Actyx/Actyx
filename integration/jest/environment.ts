/* eslint-disable @typescript-eslint/no-explicit-any */
import { Client, DefaultClientOpts } from '@actyx/os-sdk'
import NodeEnvironment from 'jest-environment-node'
import { CLI } from '../src/ax'
import { MyGlobal } from './setup'

class MyEnvironment extends NodeEnvironment {
  // eslint-disable-next-line @typescript-eslint/explicit-module-boundary-types
  constructor(config: any, _context: any) {
    super(config)
  }

  async setup(): Promise<void> {
    await super.setup()

    const axNodeSetup = (<MyGlobal>(<unknown>this.global)).axNodeSetup

    for (const node of axNodeSetup.nodes) {
      node.ax = new CLI(node._private.axHost, node._private.axBinary)

      const opts = DefaultClientOpts()
      opts.Endpoints.ConsoleService.BaseUrl = node._private.apiConsole
      opts.Endpoints.EventService.BaseUrl = node._private.apiEvent
      node.actyxOS = Client(opts)
    }
  }

  async teardown(): Promise<void> {
    await super.teardown()
  }

  // eslint-disable-next-line @typescript-eslint/explicit-module-boundary-types
  runScript<T>(script: any): T | null {
    return super.runScript(script)
  }
}

export default MyEnvironment
