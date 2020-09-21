import { ActyxOSNode, NodeSelection } from './types'

const matches = (selection: NodeSelection) => (node: ActyxOSNode) =>
  (selection.os === node.target.os || selection.os === undefined) &&
  (selection.arch === node.target.arch || selection.arch === undefined) &&
  (selection.host === node.host || selection.host === undefined) &&
  (node.runtimes.some((rt) => rt === selection.runtime) || selection.runtime === undefined)

export const selectNodes = (
  selections: NodeSelection[],
  nodes: ActyxOSNode[],
): ActyxOSNode[] | null => {
  const found = selections.map((sel) => nodes.filter(matches(sel)))
  const assigned: (ActyxOSNode | null)[] = selections.map(() => null)

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
  return assigned as ActyxOSNode[]
}
