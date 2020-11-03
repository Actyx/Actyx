import { mkExec } from './exec'

export * from './types'

export class CLI {
  private readonly binary: string
  public readonly Nodes
  public readonly Apps
  public readonly Settings
  public readonly Logs
  public readonly Swarms

  constructor(private readonly node: string, binary?: string) {
    // TODO get binary from Cosmos build
    this.binary = binary || 'ax'
    const exec = mkExec(this.binary, this.node)
    this.Nodes = exec.Nodes
    this.Apps = exec.Apps
    this.Settings = exec.Settings
    this.Logs = exec.Logs
    this.Swarms = exec.Swarms
  }
}
