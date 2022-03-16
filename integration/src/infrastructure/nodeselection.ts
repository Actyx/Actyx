import { ActyxNode, NodeSelection } from './types'

const nodeMatches =
  (selection: NodeSelection) =>
  (node: ActyxNode): boolean =>
    (selection.os === node.target.os || selection.os === undefined) &&
    (selection.arch === node.target.arch || selection.arch === undefined) &&
    (selection.host === node.host || selection.host === undefined)

/**
 * Select nodes from the given array of nodes, yielding an array containing
 * one entry per selection and in matching order; or yielding null if the
 * selection cannot be fulfilled.
 *
 * The algorithm is not the dumbest, but also not perfect: it may not always
 * find a solution to the constraints, even when one would exist.
 */
export const selectNodes = (
  selections: NodeSelection[],
  nodes: ActyxNode[],
): ActyxNode[] | null => {
  const found = selections.map((sel) => nodes.filter(nodeMatches(sel)))
  const assigned: (ActyxNode | null)[] = selections.map(() => null)

  while (assigned.some((a) => a === null)) {
    const min = Math.min(...found.map((x, idx) => (assigned[idx] === null ? x.length : Infinity)))
    if (min === 0) {
      return null
    }
    const idx = found.findIndex((x, idx) => assigned[idx] === null && x.length === min)
    const node = found[idx][0] // yes, should probably back-track later, but KISS
    assigned[idx] = node
    for (const nodes of found) {
      const i = nodes.findIndex((n) => n.name === node.name)
      if (i >= 0) {
        nodes.splice(i, 1)
      }
    }
  }
  return assigned as ActyxNode[]
}
