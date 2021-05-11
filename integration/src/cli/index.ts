import { settings } from '../infrastructure/settings'
import * as path from 'path'
import { mkExec } from './exec'

export * from './types'

export class CLI {
  private readonly binaryPath: string
  public readonly identityPath: string
  public readonly nodes
  public readonly settings
  public readonly logs
  public readonly swarms
  public readonly users
  public readonly version
  public readonly shortVersion

  public static async build(node: string, binaryPath: string): Promise<CLI> {
    const randIdentifier = Math.random().toString(36).substring(7)
    const identityPath = path.resolve(settings().tempDir, `${node}-${randIdentifier}`)
    const cli = new CLI(node, binaryPath, identityPath)

    // Generate local key pair
    await cli.users.keyGen(cli.identityPath)
    return cli
  }

  public static async buildWithIdentityPath(
    node: string,
    binaryPath: string,
    identityPath: string,
  ): Promise<CLI> {
    const cli = new CLI(node, binaryPath, identityPath)

    // Make sure a local keypair is available; ignore if the file already exists
    await cli.users.keyGen(cli.identityPath)
    return cli
  }

  private constructor(private readonly node: string, binaryPath: string, identityPath: string) {
    this.binaryPath = binaryPath
    this.identityPath = identityPath

    const exec = mkExec(this.binaryPath, this.node, this.identityPath)
    const shortVersion = exec.version().then((v) => v.replace('Actyx CLI ', '').split('-')[0])

    this.nodes = exec.nodes
    this.settings = exec.settings
    this.logs = exec.logs
    this.swarms = exec.swarms
    this.users = exec.users
    this.version = exec.version
    this.shortVersion = shortVersion
  }
}
