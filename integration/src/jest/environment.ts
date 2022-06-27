/* eslint-disable @typescript-eslint/no-explicit-any */
import { EC2Client as EC2 } from '@aws-sdk/client-ec2'
import NodeEnvironment from 'jest-environment-node'
import { type MyGlobal } from './setup'

class MyEnvironment extends NodeEnvironment {
  // eslint-disable-next-line @typescript-eslint/explicit-module-boundary-types
  constructor(config: any, _context: any) {
    super(config)
  }

  async setup(): Promise<void> {
    await super.setup()

    const axNodeSetup = (<MyGlobal>(<unknown>this.global)).axNodeSetup
    // mkExecute below needs the settings, which it takes from `global`, not `this.global`
    // (the test suite will later be run with `this.global` installed)
    ;(<MyGlobal>global).axNodeSetup = axNodeSetup
    axNodeSetup.ec2 = new EC2({ region: 'eu-central-1' })
    axNodeSetup.thisTestEnvNodes = []

    const { CLI } = await import('../cli')
    const { mkExecute } = await import('../infrastructure')

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

    ;(<MyGlobal>(<unknown>this.global)).isSuite = true
  }

  async teardown(): Promise<void> {
    // eslint-disable-next-line @typescript-eslint/consistent-type-assertions, @typescript-eslint/no-explicit-any
    const state = (<any>expect).getState()
    let testName: string = state.testPath
    if (testName.startsWith(process.cwd())) {
      testName = `<cwd>` + testName.substr(process.cwd().length)
    }
    for (const node of (<MyGlobal>(<unknown>this.global)).axNodeSetup.thisTestEnvNodes || []) {
      process.stderr.write(`shutting down node ${node.name} from ${testName}\n`)
      try {
        await new Promise((res, rej) => {
          node._private.shutdown().then(res)
          setTimeout(
            () => rej(new Error(`timeout stopping ad hoc node ${node.name} from ${testName}`)),
            10_000,
          )
        })
      } catch (e) {
        process.stderr.write(`${e}`)
      }
    }
    await super.teardown()
  }
}

export default MyEnvironment
