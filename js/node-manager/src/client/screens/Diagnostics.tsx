import React from 'react'
import { UiNode, NodeType, ReachableNodeUi as ReachableNodeT } from '../../common/types'
import { Layout } from '../components/Layout'
import { useAppState } from '../app-state'
import { SimpleCanvas } from '../components/SimpleCanvas'
import { Button, Tabs } from '../components/basics'
import { shutdownApp, toggleDevTools } from '../util'
import { useStore } from '../store'
import clsx from 'clsx'
import { HslPercentageSpectrum, RedToGreenPercentageSpectrum } from '../util/color'
import { OffsetInfo, Offset } from '../offsets'
import { isNone } from 'fp-ts/lib/Option'

const isConnectedTo = (from: ReachableNodeT, to: UiNode) => {
  if (to.type === NodeType.Reachable) {
    if (from.details.swarmState === null || to.details.swarmState === null) {
      return false
    } else {
      return from.details.swarmState.connections
        .map(({ peerId }) => peerId)
        .includes(to.details.swarmState.peerId)
    }
  } else {
    if (from.details.swarmState === null) {
      return false
    }
    return from.details.swarmState.connections.filter(({ addr }) => addr === to.addr).length > 0
  }
}

const nodeDisplayName = (node: UiNode) =>
  node.type === NodeType.Reachable ? `${node.details.displayName} (${node.addr})` : node.addr

const Connected = () => <span className="text-green-300 font-xs font-medium">Connected</span>
const Disconnected = () => <span className="text-red-300 font-xs font-medium">Not connected</span>

const OffsetMatrix: React.FC<{ nodes: UiNode[]; offsets: OffsetInfo }> = ({ nodes, offsets }) => {
  const headerSize = 44
  const cellWidth = 16
  const cellHeight = 8

  const mkRedGreenColor = HslPercentageSpectrum(0, 120, undefined, 75)

  const offsetToString = (offset: Offset) => {
    if (typeof offset === 'number') {
      return offset.toString()
    } else if (offset === 'NoOffsetFound') {
      return '!None'
    } else if (offset === 'SourceNotReachable') {
      return '!Source'
    } else if (offset === 'TargetNotReachable') {
      return '!Target'
    } else if (offset === 'SourceDoesntSupportOffsets') {
      return '!Version'
    }
  }

  // We ignore all loading nodes
  nodes = nodes.filter((n) => n.type !== NodeType.Fresh)

  return (
    <div className="w-max">
      {nodes.length < 2 ? (
        <p className="text-gray-400">
          Must be connected to at least two nodes to show offset matrix (current: {nodes.length}
          ).
        </p>
      ) : (
        <>
          <div className="flex flew-row text-xs">
            <div className={clsx(`w-${headerSize} h-${headerSize}`)}></div>
            {nodes.map((n, ix) => (
              <div
                key={'om-header-' + n.addr}
                className={clsx(
                  `truncate overflow-ellipsis h-${headerSize} w-${cellWidth} py-2 font-medium`,
                  {
                    'border-l': ix === 0,
                  },
                  'border-r',
                  'pl-5',
                )}
                style={{
                  writingMode: 'vertical-lr',
                }}
              >
                {nodeDisplayName(n)}
              </div>
            ))}
          </div>
          {nodes.map((from, ix) => {
            return (
              <div key={'om-row-' + from.addr} className="flex flew-row text-xs">
                <div
                  className={clsx(
                    `w-${headerSize} h-${cellHeight} font-medium truncate overflow-ellipsis px-2 pt-1.5 border-b`,
                    {
                      'border-t': ix === 0,
                    },
                  )}
                >
                  {nodeDisplayName(from)}
                </div>
                {nodes.map((to, ix) => {
                  const offset = offsets.matrix[from.addr][to.addr]
                  const highestKnown = offsets.highestKnown[to.addr]
                  const percentage = typeof offset !== 'number' ? 0 : (100 * offset) / highestKnown
                  return (
                    <div
                      key={`om-from-${from.addr}-to-${to.addr}`}
                      className={clsx(
                        `w-${cellWidth} h-${cellHeight} truncate overflow-ellipsis font-medium px-2 pt-1.5 border-b border-r`,
                      )}
                      style={{
                        backgroundColor: mkRedGreenColor(percentage),
                      }}
                    >
                      {offsetToString(offset)}
                    </div>
                  )
                })}
              </div>
            )
          })}
          <div className="pt-3 text-sm text-gray-400">
            <p>{offsetToString('NoOffsetFound')}: no offsets found.</p>
            <p>{offsetToString('SourceNotReachable')}: source node not reachable.</p>
            <p>{offsetToString('TargetNotReachable')}: target node not reachable.</p>
            <p>{offsetToString('SourceDoesntSupportOffsets')}: source version too old.</p>
          </div>
        </>
      )}
    </div>
  )
}
const ReachableNode: React.FC<{ node: ReachableNodeT; others: UiNode[] }> = ({ node, others }) => (
  <li className="pb-3">
    <span className="font-medium">{nodeDisplayName(node)}</span>
    <ul className="">
      {others.map((other) => (
        <li className="text-sm" key={node.addr + '->' + other.addr}>
          {isConnectedTo(node, other) ? <Connected /> : <Disconnected />} to{' '}
          {nodeDisplayName(other)}
        </li>
      ))}
    </ul>
  </li>
)

const UnreachableNode: React.FC<{ node: UiNode }> = ({ node }) => (
  <li className="text-gray-500 pb-3">
    <span className="font-medium">{nodeDisplayName(node)}</span> is not reachable
  </li>
)

const SwarmConnectivity: React.FC<{ nodes: UiNode[] }> = ({ nodes }) => {
  return (
    <div>
      {nodes.length < 2 ? (
        <p className="text-gray-400">
          Must be connected to at least two nodes to show swarm diagnostics (current: {nodes.length}
          ).
        </p>
      ) : (
        <ul className="">
          {nodes.map((node) =>
            node.type === NodeType.Reachable ? (
              <ReachableNode
                key={'from' + node.addr}
                node={node}
                others={nodes.filter((other) => other.addr !== node.addr)}
              />
            ) : (
              <UnreachableNode key={'from' + node.addr} node={node} />
            ),
          )}
        </ul>
      )}
    </div>
  )
}

const NodeManager: React.FC = () => {
  const { state, data } = useAppState()
  const store = useStore()
  return (
    <div>
      <Button className="mb-3" color="yellow" onClick={toggleDevTools} small>
        Toggle Dev Tools
      </Button>
      <Button className="ml-2 mb-3" color="red" onClick={shutdownApp} small>
        Exit Node Manager
      </Button>
      <p className="py-3 text-gray-400">
        Post this information if you are having issues with the Actyx Node Manager.
      </p>
      <p className="pb-3">App State</p>
      <pre className="text-xs p-1 border border-gray-300 bg-gray-100 break-all whitespace-pre-wrap">
        {JSON.stringify(state, null, 2)}
      </pre>
      <p className="py-3">Store State</p>
      <pre className="text-xs p-1 border border-gray-300 bg-gray-100 break-all whitespace-pre-wrap">
        {JSON.stringify(store, (_k, v) => (typeof v === 'function' ? '<function>' : v), 2)}
      </pre>
      <p className="py-3">Data</p>
      <pre className="text-xs p-1 border border-gray-300 bg-gray-100 break-all whitespace-pre-wrap">
        {JSON.stringify(data, null, 2)}
      </pre>
    </div>
  )
}

const Screen: React.FC = () => {
  const {
    data: { nodes, offsets },
  } = useAppState()

  return (
    <Layout title={`Diagnostics`}>
      <SimpleCanvas>
        <div className="flex flex-col h-full">
          <Tabs
            className="flex-grow flex-shrink"
            contentClassName="p-2 overflow-x-auto"
            tabs={[
              {
                text: 'Swarm Connectivity',
                elem: <SwarmConnectivity nodes={nodes} />,
              },
              {
                text: 'Offset Matrix',
                elem: isNone(offsets) ? (
                  <p>Loading...</p>
                ) : (
                  <OffsetMatrix nodes={nodes} offsets={offsets.value} />
                ),
              },
              {
                text: 'Node Manager',
                elem: <NodeManager />,
              },
            ]}
          />
        </div>
      </SimpleCanvas>
    </Layout>
  )
}

export default Screen
