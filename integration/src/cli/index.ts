import { mkExec } from './exec'

export * from './types'

export class CLI {
  private readonly binaryPath: string
  public readonly nodes
  public readonly settings
  public readonly logs
  public readonly swarms

  constructor(private readonly node: string, binaryPath: string) {
    this.binaryPath = binaryPath
    const exec = mkExec(this.binaryPath, this.node)
    this.nodes = exec.nodes
    this.settings = exec.settings
    this.logs = exec.logs
    this.swarms = exec.swarms
  }
}
