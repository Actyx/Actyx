import { Pond } from '@actyx/pond'
import { MyGlobal } from '../../jest/setup'
import { trialManifest } from '../http-client'
import { newProcess } from '../util'
import { selectNodes } from './nodeselection'
import { getTestName } from './settings'
import { ActyxNode, NodeSelection } from './types'

// This is provided by jest/setup.ts and passed by jest to our worker (serialised)
// therefore any contained functions will not work.
const nodes: ActyxNode[] = (<MyGlobal>global).axNodeSetup.nodes

const ts = () => new Date().toISOString()

export const allNodeNames = (): string[] => nodes.map((n) => n.name)

/**
 * Run the given logic for each of the selected nodes in parallel and return
 * an array of results, in the same order that the selections were given.
 *
 * @param selection list of node selectors
 * @param exclusive when true, the given logic has exclusive access to the Actyx node
 * @param f the logic to be executed for each node
 */
export const runOnEach = async <T>(
  selection: NodeSelection[],
  f: (node: ActyxNode) => Promise<T>,
): Promise<T[]> => {
  const n = selectNodes(selection, nodes)
  if (n === null) {
    console.warn(
      `Can not satisfy node selection ${JSON.stringify(selection)}. Skipping ${getTestName()}`,
    )
    return Promise.resolve([])
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
  f: (nodes: ActyxNode[]) => ReadonlyArray<Promise<T>>,
): Promise<T[]> => {
  return runOnNodes('runOnAll', nodes, () => Promise.all(f(nodes)))
}

export const withPond = async <T>(
  node: ActyxNode,
  f: (pond: Pond, nodeName: string) => Promise<T>,
): Promise<T> => {
  const pond = await Pond.of(
    trialManifest,
    {
      actyxPort: node._private.apiEventsPort,
      onConnectionLost: () => console.error(node.name, 'Pond lost connection to the store'),
    },
    {},
  )
  try {
    return await f(pond, node.name)
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
  f: (nodes: ActyxNode[]) => Promise<T>,
): Promise<T> => {
  const n = selectNodes(selection, nodes)
  if (n === null) {
    throw new Error('cannot satisfy node selection ' + JSON.stringify(selection))
  }

  return runOnNodes('runOnAll', n, () => f(n))
}

/**
 * Run given logic on each node and return the computed value.
 *
 * @param f the logic to be executed with all available nodes
 */
export const runOnEvery = <T>(f: (node: ActyxNode) => Promise<T>): Promise<T[]> =>
  runOnNodes('runOnEvery', nodes, () =>
    Promise.all(
      nodes.map((node) =>
        f(node).catch((error) => {
          error.message += '\n\n... while testing node ' + node.name
          throw error
        }),
      ),
    ),
  )

export const runWithNewProcess = <T>(
  f: (node: ActyxNode) => Promise<T>,
): Promise<(T | undefined)[]> =>
  runOnEvery(async (node) => {
    if (node.host !== 'process' || node.target.os === 'macos') {
      return
    }
    const n = await newProcess(node)
    try {
      return await f(n)
    } finally {
      n._private.shutdown()
    }
  })

const runOnNodes = async <T>(label: string, n: ActyxNode[], f: () => Promise<T>): Promise<T> => {
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
