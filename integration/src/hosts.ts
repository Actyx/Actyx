import { selectNodes } from './nodeselection'
import { RwLock } from './rwlock'
import { ActyxOSNode, NodeSelection, Target } from './types'

let nodes: ActyxOSNode[] = []

/**
 * Try to deploy ActyxOS on all given targets, each one receiving all compatible
 * variants (e.g. might deploy ActyxOS on Docker and ActyxOS on Windows when given
 * a Windows target that has Docker running)
 *
 * @param hosts list of target hosts on which ActyxOS can be deployed
 */
export const buildNodes = async (hosts: Target[]): Promise<void> => {
  const build: Promise<ActyxOSNode[]>[] = []
  let idx = 0
  const getName = () => {
    idx += 1
    return `h${idx}`
  }

  for (const host of hosts) {
    if (host.docker !== undefined) {
      build.push(deployDocker(host.docker, host, getName()).catch(() => []))
    }
    if (host.rsync !== undefined) {
      build.push(deployProcess(host.rsync, host, getName()).catch(() => []))
    }
    if (host.adb !== undefined) {
      build.push(deployAndroid(host.adb, host, getName()).catch(() => []))
    }
  }

  nodes = (await Promise.all(build)).flat()
}

const deployDocker = async (base: URL, host: Target, name: string): Promise<ActyxOSNode[]> => []
const deployProcess = async (base: string, host: Target, name: string): Promise<ActyxOSNode[]> => []
const deployAndroid = async (base: string, host: Target, name: string): Promise<ActyxOSNode[]> => []

const lock = new RwLock()

/**
 * Run the given logic for each of the selected nodes in parallel and return
 * an array of results, in the same order that the selections were given.
 *
 * @param selection list of node selectors
 * @param exclusive when true, the given logic has exclusive access to the ActyxOS node
 * @param f the logic to be executed for each node
 */
export const runOnEach = <T>(
  selection: NodeSelection[],
  exclusive: boolean,
  f: (node: ActyxOSNode) => Promise<T>,
): Promise<T[]> => {
  const n = selectNodes(selection, nodes)
  if (n === null) {
    throw new Error('cannot satisfy node selection ' + JSON.stringify(selection))
  }
  const logic = () => Promise.all(n.map(f))
  return exclusive ? lock.writeLock(logic) : lock.readLock(logic)
}

/**
 * Run the given logic once for all of the selected nodes and return the
 * computed value.
 *
 * @param selection list of node selectors
 * @param exclusive when true, the given logic has exclusive access to all selected ActyxOS nodes
 * @param f the logic to be executed with all selected nodes
 */
export const runOnAll = <T>(
  selection: NodeSelection[],
  exclusive: boolean,
  f: (nodes: ActyxOSNode[]) => Promise<T>,
): Promise<T> => {
  const n = selectNodes(selection, nodes)
  if (n === null) {
    throw new Error('cannot satisfy node selection ' + JSON.stringify(selection))
  }
  const logic = () => f(n)
  return exclusive ? lock.writeLock(logic) : lock.readLock(logic)
}
