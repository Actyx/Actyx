import React, { useState } from 'react'
import { NodeType, ReachableNode as ReachableNodeT } from '../../common/types'
import { Layout } from '../components/Layout'
import { useAppState, AppActionKey } from '../app-state'
import { Error } from '../components/Error'
import { SimpleCanvas } from '../components/SimpleCanvas'
import { Button, Tabs } from '../components/basics'
import { SettingsEditor } from '../components/SettingsEditor'
import clsx from 'clsx'
import { string } from 'fp-ts'

const removePeerIdFromPeerAddr = (addr: string): string => {
  const parts = addr.split('/')
  if (parts.length < 2) {
    return addr
  }

  return parts.slice(0, -1).join('/')
}

const Peers_Peer: React.FC<{
  peerId: string
  connectedAddr?: string
  disconnectedAddrs: string[]
  showAddresses: boolean
}> = ({ peerId, connectedAddr, disconnectedAddrs, showAddresses }) => {
  const isConnected = !!connectedAddr
  const allAddresses = disconnectedAddrs.concat(connectedAddr ? [connectedAddr] : [])

  return (
    <p
      className={clsx('text-sm text-gray-500', {
        'pb-1': showAddresses,
      })}
    >
      {!connectedAddr && <span className="font-medium text-red-300">Disconnected</span>}
      {connectedAddr && <span className="font-medium text-green-300">Connected</span>} node{' '}
      <code className="text-black">{peerId}</code> with {allAddresses.length} known address
      {allAddresses.length > 1 ? 'es' : ''}
      {showAddresses ? ':' : ''}
      {showAddresses && (
        <ul className="list-disc pl-5">
          {connectedAddr && (
            <li>
              <code className="text-black">{removePeerIdFromPeerAddr(connectedAddr)}</code>{' '}
              <span className="italic">(currently connected via this address)</span>
            </li>
          )}
          {disconnectedAddrs.map((addr, ix) => (
            <li key={`${peerId}-disconnected-addr-${ix}`}>
              <code>{removePeerIdFromPeerAddr(addr)}</code>
            </li>
          ))}
        </ul>
      )}
    </p>
  )
}

const Peers: React.FC<{ node: ReachableNodeT }> = ({ node }) => {
  const [showConnectedAddresses, setConnectedShowAddresses] = useState(true)
  const [showDisconnectedAddresses, setDisconnectedShowAddresses] = useState(false)
  if (node.details.swarmState === null) {
    return <p className="text-yellow-500">Please wait for complete node startup...</p>
  }
  const { knownPeers, connections: connectedPeers } = node.details.swarmState
  const disconnectedPeers = knownPeers.filter(
    ({ peerId }) => !connectedPeers.find((p) => p.peerId === peerId),
  )
  return (
    <div className="">
      <p className="pb-2">
        Node connected to {connectedPeers.length} peers
        {connectedPeers.length > 0 && (
          <>
            {' '}
            (
            <span
              className="text-sm underline text-blue-500 cursor-pointer pb-2"
              onClick={() => setConnectedShowAddresses((curr) => !curr)}
            >
              {showConnectedAddresses ? 'hide' : 'show'} addresses
            </span>
            )
          </>
        )}
        {connectedPeers.length > 0 ? ':' : '.'}
      </p>
      {connectedPeers.map(({ peerId, addr: connectedAddr }) => {
        const knownPeer = knownPeers.find((p) => p.peerId === peerId)
        const disconnectedAddrs = !knownPeer
          ? []
          : knownPeer.addrs.filter((a) => a !== connectedAddr)
        return (
          <Peers_Peer
            key={`connected-${peerId}`}
            peerId={peerId}
            disconnectedAddrs={disconnectedAddrs}
            connectedAddr={connectedAddr}
            showAddresses={showConnectedAddresses}
          />
        )
      })}
      <p className="pt-4 pb-2">
        Node not connected to {disconnectedPeers.length} known peers
        {disconnectedPeers.length > 0 && (
          <>
            {' '}
            (
            <span
              className="text-sm underline text-blue-500 cursor-pointer pb-2"
              onClick={() => setDisconnectedShowAddresses((curr) => !curr)}
            >
              {showDisconnectedAddresses ? 'hide' : 'show'} addresses
            </span>
            )
          </>
        )}
        {disconnectedPeers.length > 0 ? ':' : '.'}
      </p>
      {disconnectedPeers.map(({ peerId, addrs }) => (
        <Peers_Peer
          key={`disconnected-${peerId}`}
          peerId={peerId}
          disconnectedAddrs={addrs}
          showAddresses={showDisconnectedAddresses}
        />
      ))}
    </div>
  )
}

const Actions: React.FC<{ node: ReachableNodeT }> = ({ node: { addr } }) => {
  const [shuttingDown, setShuttingDown] = useState(false)
  const {
    actions: { shutdownNode },
  } = useAppState()
  return (
    <div className="">
      <Button
        className=""
        color="red"
        onClick={() => {
          setShuttingDown(true)
          shutdownNode(addr)
        }}
        working={shuttingDown}
        small
      >
        Shutdown node
      </Button>
      <p className="pt-2 italic text-gray-400">
        Note that the Actyx node may automatically be restarted if it is managed as a system
        service.
      </p>
    </div>
  )
}

const Info: React.FC<{ node: ReachableNodeT }> = ({ node: { details } }) => {
  const Row: React.FC<{ field: string; value: string; gray?: boolean; waiting?: boolean }> = ({
    field,
    value,
    waiting,
    gray,
  }) => (
    <div className={clsx('py-2 sm:grid sm:grid-cols-3 sm:gap-4 sm:px-6', { 'bg-gray-50': gray })}>
      <dt className="text-sm font-medium text-gray-500">{field}</dt>
      <dd
        className={clsx(
          'mt-1 text-sm text-gray-900 sm:mt-0 sm:col-span-2 selectable truncate overflow-ellipsis',
          {
            'text-yellow-500': waiting,
          },
        )}
      >
        {value}
      </dd>
    </div>
  )

  return (
    <div className="">
      <dl>
        <Row field="Display name" value={details.displayName} gray />
        <Row
          field="Bind addresses"
          value={
            details.addrs === null ? 'Please wait for complete node startup...' : details.addrs
          }
          waiting={details.addrs === null}
        />
        <Row field="Node ID" value={details.nodeId} gray />
        <Row
          field="Peer ID"
          value={
            details.swarmState === null
              ? 'Please wait for complete node startup...'
              : details.swarmState.peerId
          }
          waiting={details.addrs === null}
        />
        <Row field="Running since" value={details.startedIso} gray />
        <Row field="Version" value={details.version} />
      </dl>
    </div>
  )
}

const Offsets: React.FC<{ node: ReachableNodeT }> = ({
  node: {
    details: { offsets },
  },
}) => {
  const {
    data: { nodes },
  } = useAppState()
  const allReachableNodes = nodes.filter((n) => n.type === NodeType.Reachable) as ReachableNodeT[]

  const Row: React.FC<{ field: string; value: string; gray?: boolean }> = ({
    field,
    value,
    gray,
  }) => (
    <div className={clsx('py-2 sm:grid sm:grid-cols-6 sm:gap-4 sm:px-6', { 'bg-gray-50': gray })}>
      <dt className="text-sm font-medium text-gray-500 sm:col-span-5">{field}</dt>
      <dd className="mt-1 text-sm text-gray-900 sm:mt-0 sm:col-span-1 selectable">{value}</dd>
    </div>
  )

  const streamDisp = (streamName: string) => {
    const nodeId = streamName.slice(0, -2)
    const match = allReachableNodes.find((node) => node.details.nodeId === nodeId)
    if (!match) {
      return nodeId
    }
    return `${match.details.displayName} (${match.addr})`
  }

  if (offsets === null) {
    return (
      <p className="text-red-300 italic">
        The node&apos;s version of Actyx doesn&apos;t support this feature yet. Please upgrade to a
        later version.
      </p>
    )
  }

  return (
    <div className="">
      <dl>
        {Object.entries(offsets.present).filter(([streamId]) => streamId.endsWith('-0')).length <
        1 ? (
          <p className="text-sm pl-2">None</p>
        ) : (
          Object.entries(offsets.present)
            .filter(([streamId]) => streamId.endsWith('-0'))
            .map(([streamId, offset], ix) => {
              const toReplicate =
                streamId in offsets.toReplicate ? offsets.toReplicate[streamId] : 0
              return (
                <Row
                  key={'offsets' + streamId}
                  field={streamDisp(streamId)}
                  value={
                    offset.toString() + (toReplicate > 0 ? `(+${toReplicate.toString()})` : '')
                  }
                  gray={!(ix % 2)}
                />
              )
            })
        )}
      </dl>
    </div>
  )
}

const Addresses: React.FC<{ node: ReachableNodeT }> = ({ node }) => {
  if (node.details.swarmState === null) {
    return <p className="text-yellow-500">Please wait for complete node startup...</p>
  }

  const {
    details: {
      swarmState: { swarmAddrs, announceAddrs, adminAddrs },
    },
  } = node

  return (
    <>
      <div className="pb-3">
        {swarmAddrs.length < 1 ? (
          <p>Node is not listening to swarm on any addresses.</p>
        ) : (
          <>
            <p className="pb-1">Node is listening to swarm on {swarmAddrs.length} addresses:</p>
            <ul className="list-disc pl-5 text-sm">
              {swarmAddrs.map((addr) => (
                <li key={'swarm' + addr}>
                  <code>{addr}</code>
                </li>
              ))}
            </ul>
          </>
        )}
      </div>
      <div className="pb-3">
        {announceAddrs.length < 1 ? (
          <p>Node has no external addresses.</p>
        ) : (
          <>
            <p className="pb-1">Node has {announceAddrs.length} external addresses:</p>
            <ul className="list-disc pl-5 text-sm">
              {announceAddrs.map((addr) => (
                <li key={'announce' + addr}>
                  <code>{addr}</code>
                </li>
              ))}
            </ul>
          </>
        )}
      </div>
      <div className="pb-3">
        {adminAddrs.length < 1 ? (
          <p>Node has no admin addresses.</p>
        ) : (
          <>
            <p className="pb-1">Node has {adminAddrs.length} admin addresses:</p>
            <ul className="list-disc pl-5 text-sm">
              {adminAddrs.map((addr) => (
                <li key={'admin' + addr}>
                  <code>{addr}</code>
                </li>
              ))}
            </ul>
          </>
        )}
      </div>
    </>
  )
}

const ReachableNode: React.FC<{ node: ReachableNodeT }> = ({ node }) => (
  <Tabs
    className="flex-grow flex-shrink"
    contentClassName="p-2"
    tabs={[
      {
        text: 'Info',
        elem: <Info node={node} />,
      },
      {
        text: 'Addresses',
        elem: <Addresses node={node} />,
      },
      {
        text: 'Peers',
        elem: <Peers node={node} />,
      },
      {
        text: 'Offsets',
        elem: <Offsets node={node} />,
      },
      {
        text: 'Settings',
        elem: <SettingsEditor node={node} />,
      },
      {
        text: 'Actions',
        elem: <Actions node={node} />,
      },
    ]}
  />
)

const Screen: React.FC<{ addr: string }> = ({ addr }) => {
  const {
    dispatch,
    data: { nodes },
  } = useAppState()
  const node = nodes.find((n) => n.addr === addr)
  if (!node) {
    return (
      <Layout title="Error">
        <Error>
          <p>No data available about node {addr}.</p>
        </Error>
      </Layout>
    )
  }

  if (node.type !== NodeType.Reachable) {
    dispatch({ key: AppActionKey.ShowOverview })
    return (
      <Layout title="Error">
        <Error>
          <p>Node {addr} not connected.</p>
        </Error>
      </Layout>
    )
  }

  return (
    <Layout
      title={`Nodes > ${node.details.displayName} (${addr})`}
      actions={[
        {
          target: () => dispatch({ key: AppActionKey.ShowOverview }),
          text: 'Back to overview',
        },
      ]}
    >
      <SimpleCanvas>
        <ReachableNode node={node} />
      </SimpleCanvas>
    </Layout>
  )
}

export default Screen
