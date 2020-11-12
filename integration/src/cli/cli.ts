import { mkExec } from './exec'

export * from './types'

export class CLI {
  private readonly binaryPath: string
  public readonly Nodes
  public readonly Apps
  public readonly Settings
  public readonly Logs
  public readonly Swarms

  constructor(private readonly node: string, binaryPath: string) {
    // TODO get binary from Cosmos build
    this.binaryPath = binaryPath
    const exec = mkExec(this.binaryPath, this.node)
    this.Nodes = exec.Nodes
    this.Apps = exec.Apps
    this.Settings = exec.Settings
    this.Logs = exec.Logs
    this.Swarms = exec.Swarms
  }
}
