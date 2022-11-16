import { NodeType, UiNode } from '../common/types'

const EventsStreamName = (nodeId: string) => `${nodeId}-0`

export type Offset =
  | 'SourceDoesntSupportOffsets'
  | 'SourceNotReachable'
  | 'TargetNotReachable'
  | 'NoOffsetFound'
  | number

export type OffsetMatrix = { [sourceNodeAddr: string]: { [targetNodeAddr: string]: Offset } }

export type HighestKnownOffsets = { [targetNodeAddr: string]: number }

export type OffsetInfo = {
  matrix: OffsetMatrix
  highestKnown: HighestKnownOffsets
}
export const OffsetInfo = {
  of: (nodes: UiNode[]): OffsetInfo => {
    const matrix: OffsetMatrix = {}
    const highestKnown: HighestKnownOffsets = {}

    nodes.forEach((sourceNode) => {
      nodes.forEach((targetNode) => {
        const targetNodeAddr = targetNode.addr
        const sourceNodeAddr = sourceNode.addr
        if (!(sourceNodeAddr in matrix)) {
          matrix[sourceNodeAddr] = {}
        }
        if (!(targetNodeAddr in highestKnown)) {
          // Avoid a div by zero error
          highestKnown[targetNodeAddr] = 0.000001
        }

        if (sourceNode.type !== NodeType.Reachable) {
          matrix[sourceNodeAddr][targetNodeAddr] = 'SourceNotReachable'
          return
        }
        const offsets = sourceNode.details.offsets
        if (offsets === null) {
          matrix[sourceNodeAddr][targetNodeAddr] = 'SourceDoesntSupportOffsets'
          return
        }
        if (targetNode.type !== NodeType.Reachable) {
          matrix[sourceNodeAddr][targetNodeAddr] = 'TargetNotReachable'
          return
        }

        const streamName = EventsStreamName(targetNode.details.nodeId)
        if (!(streamName in offsets.present)) {
          matrix[sourceNodeAddr][targetNodeAddr] = 'NoOffsetFound'
          return
        }

        const targetOffset = offsets.present[streamName]
        matrix[sourceNodeAddr][targetNodeAddr] = targetOffset

        highestKnown[targetNodeAddr] = Math.max(highestKnown[targetNodeAddr], targetOffset)
      })
    })

    return { matrix, highestKnown }
  },
}
