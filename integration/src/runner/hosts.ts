import { MyGlobal } from '../../jest/setup'
import { selectNodes } from './nodeselection'
import { RwLock } from './rwlock'
import { ActyxOSNode, NodeSelection } from './types'

const nodes: ActyxOSNode[] = (<MyGlobal>global).nodeSetup.nodes || []

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
