import { MyGlobal } from '../../jest/setup'
import { selectNodes } from './nodeselection'
import { RwLock } from './rwlock'
import { ActyxOSNode, NodeSelection } from './types'

// This is provided by jest/setup.ts and passed by jest to our worker (serialised)
// therefore any contained functions will not work.
const nodes: ActyxOSNode[] = (<MyGlobal>global).nodeSetup.nodes

const lock = new RwLock()

const ts = () => new Date().toISOString()

/**
 * Run the given logic for each of the selected nodes in parallel and return
 * an array of results, in the same order that the selections were given.
 *
 * @param selection list of node selectors
 * @param exclusive when true, the given logic has exclusive access to the ActyxOS node
 * @param f the logic to be executed for each node
 */
export const runOnEach = async <T>(
  selection: NodeSelection[],
  exclusive: boolean,
  f: (node: ActyxOSNode) => Promise<T>,
): Promise<T[]> => {
  const n = selectNodes(selection, nodes)
  if (n === null) {
    throw new Error('cannot satisfy node selection ' + JSON.stringify(selection))
  }
  const ns = n.map((x) => x.name).join(', ')

  console.log(`${ts()} runOnEach on nodes [${ns}]`)
  const logic = () => Promise.all(n.map(f))

  let success = false
  try {
    const ret = await (exclusive ? lock.writeLock(logic) : lock.readLock(logic))
    success = true
    return ret
  } finally {
    console.log(`${ts()} runOnEach on nodes [${ns}] is done, success=${success}`)
  }
}

/**
 * Run the given logic once for all of the selected nodes and return the
 * computed value.
 *
 * @param selection list of node selectors
 * @param exclusive when true, the given logic has exclusive access to all selected ActyxOS nodes
 * @param f the logic to be executed with all selected nodes
 */
export const runOnAll = async <T>(
  selection: NodeSelection[],
  exclusive: boolean,
  f: (nodes: ActyxOSNode[]) => Promise<T>,
): Promise<T> => {
  const n = selectNodes(selection, nodes)
  if (n === null) {
    throw new Error('cannot satisfy node selection ' + JSON.stringify(selection))
  }
  const ns = n.map((x) => x.name).join(', ')

  console.log(`${ts()} runOnAll on nodes [${ns}]`)
  const logic = () => f(n)
  let success = false
  try {
    const ret = await (exclusive ? lock.writeLock(logic) : lock.readLock(logic))
    success = true
    return ret
  } finally {
    console.log(`${ts()} runOnAll on nodes [${ns}] is done, success=${success}`)
  }
}
