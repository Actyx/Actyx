import React from 'react'
import { Node, NodeType, ReachableNode } from '../../common/types'
import { Layout } from '../components/Layout'
import { useAppState } from '../app-state'
import { SimpleCanvas } from '../components/SimpleCanvas'
import { Button, Tabs } from '../components/basics'
import { JsonEditor } from '../components/JsonEditor'
import { shutdownApp, toggleDevTools } from '../util'
import { useStore } from '../store'

const isConnectedTo = (from: ReachableNode, to: Node) => {
  if (to.type === NodeType.Reachable) {
    return from.details.swarmState.knownPeers
      .map(({ peerId }) => peerId)
      .includes(to.details.swarmState.peerId)
  } else {
    return (
      from.details.swarmState.knownPeers
        .map(({ addrs }) => addrs.includes(to.addr))
        .filter((x) => x).length > 0
    )
  }
}

const nodeDisplayName = (node: Node) =>
  node.type === NodeType.Reachable ? `${node.details.displayName} (${node.addr})` : node.addr

const Connected = () => <span className="text-green-300 font-xs font-medium">Connected</span>
const Disconnected = () => <span className="text-red-300 font-xs font-medium">Not connected</span>

const SwarmConnectivity: React.FC<{ nodes: Node[] }> = ({ nodes }) => {
  const ReachableNode: React.FC<{ node: ReachableNode; others: Node[] }> = ({ node, others }) => (
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

  const UnreachableNode: React.FC<{ node: Node }> = ({ node }) => (
    <li className="text-gray-500 pb-3">
      <span className="font-medium">{nodeDisplayName(node)}</span> is not reachable
    </li>
  )

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
    data: { nodes },
  } = useAppState()

  return (
    <Layout title={`Diagnostics`}>
      <SimpleCanvas>
        <div className="flex flex-col h-full">
          <Tabs
            className="flex-grow flex-shrink"
            contentClassName="p-2"
            tabs={[
              {
                text: 'Swarm Connectivity',
                elem: <SwarmConnectivity nodes={nodes} />,
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
