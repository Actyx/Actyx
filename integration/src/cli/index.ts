import { settings } from '../infrastructure/settings'
import * as path from 'path'
import { mkExec } from './exec'

export * from './types'

export class CLI {
  private readonly binaryPath: string
  private readonly identityPath: string
  public readonly nodes
  public readonly settings
  public readonly logs
  public readonly swarms
  public readonly users

  public static async build(node: string, binaryPath: string): Promise<CLI> {
    const cli = new CLI(node, binaryPath)

    // Make sure a local keypair is available; ignore if the file already exists
    await cli.users.keyGen(cli.identityPath)
    return cli
  }

  private constructor(private readonly node: string, binaryPath: string) {
    this.binaryPath = binaryPath
    this.identityPath = path.resolve(settings().tempDir, node)
    const exec = mkExec(this.binaryPath, this.node, this.identityPath)
    this.nodes = exec.nodes
    this.settings = exec.settings
    this.logs = exec.logs
    this.swarms = exec.swarms
    this.users = exec.users
  }
}
