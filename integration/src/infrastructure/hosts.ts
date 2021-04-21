import { Pond } from '@actyx/pond'
import { MyGlobal } from '../../jest/setup'
import { nodeMatches, selectNodes } from './nodeselection'
import { getTestName } from './settings'
import { ActyxOSNode, NodeSelection } from './types'

// This is provided by jest/setup.ts and passed by jest to our worker (serialised)
// therefore any contained functions will not work.
const nodes: ActyxOSNode[] = (<MyGlobal>global).axNodeSetup.nodes

const ts = () => new Date().toISOString()

export const allNodeNames = (): string[] => nodes.map((n) => n.name)

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
  f: (node: ActyxOSNode) => Promise<T>,
): Promise<T[]> => {
  const n = selectNodes(selection, nodes)
  if (n === null) {
    throw new Error('cannot satisfy node selection ' + JSON.stringify(selection))
  }

  return runOnNodes('runOnEach', n, () =>
    Promise.all(
      n.map((node) =>
        f(node).catch((error) => {
          error.message += '\n\n... while testing node ' + node.name
          throw error
        }),
      ),
    ),
  )
}

/**
 * Run the given logic once for all of the selected nodes and return the
 * computed value.
 *
 * @param f the logic to be executed with all available nodes
 */
export const runConcurrentlyOnAll = <T>(
  f: (nodes: ActyxOSNode[]) => ReadonlyArray<Promise<T>>,
): Promise<T[]> => {
  return runOnNodes('runOnAll', nodes, () => Promise.all(f(nodes)))
}

/**
 * Run the given logic on each node and return the
 * computed value.
 *
 * @param f the logic to be executed with all available nodes
 */
export const runConcurrentlyOnAllSameLogic = <T>(
  f: (node: ActyxOSNode) => Promise<T>,
): Promise<T[]> => {
  return runConcurrentlyOnAll((nodes) => nodes.map(f))
}

export const withPond = async <T>(node: ActyxOSNode, f: (pond: Pond) => Promise<T>): Promise<T> => {
  const pond = await Pond.of({ url: node._private.apiPond }, {})
  try {
    return await f(pond)
  } finally {
    pond.dispose()
  }
}

/**
 * Run the given logic once for all of the selected nodes and return the
 * computed value.
 *
 * @param selection list of node selectors
 * @param f the logic to be executed with all selected nodes
 */
export const runOnAll = async <T>(
  selection: NodeSelection[],
  f: (nodes: ActyxOSNode[]) => Promise<T>,
): Promise<T> => {
  const n = selectNodes(selection, nodes)
  if (n === null) {
    throw new Error('cannot satisfy node selection ' + JSON.stringify(selection))
  }

  return runOnNodes('runOnAll', n, () => f(n))
}

export const runOnEvery = async <T>(
  selection: NodeSelection,
  f: (node: ActyxOSNode) => Promise<T>,
): Promise<T[]> => {
  const n = nodes.filter(nodeMatches(selection))

  return runOnNodes('runOnEvery', n, () =>
    Promise.all(
      n.map((node) =>
        f(node).catch((error) => {
          error.message += '\n\n... while testing node ' + node.name
          throw error
        }),
      ),
    ),
  )
}

const runOnNodes = async <T>(label: string, n: ActyxOSNode[], f: () => Promise<T>): Promise<T> => {
  const ns = n.map((x) => x.name).join(', ')
  const testName = getTestName()
  process.stdout.write(`${ts()} ${testName} ${label} on nodes [${ns}]\n`)

  let success = false
  try {
    const ret = await f()
    success = true
    return ret
  } finally {
    process.stdout.write(
      `${ts()} ${testName} ${label} on nodes [${ns}] is done, success=${success}\n`,
    )
  }
}
