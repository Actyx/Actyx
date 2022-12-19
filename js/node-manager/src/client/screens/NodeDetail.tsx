import React, { useState } from 'react'
import { Connection, NodeType, Peer, ReachableNodeUi as ReachableNodeT } from '../../common/types'
import { Layout } from '../components/Layout'
import { useAppState, AppActionKey } from '../app-state'
import { Error } from '../components/Error'
import { SimpleCanvas } from '../components/SimpleCanvas'
import { Button, Tabs } from '../components/basics'
import { SettingsEditor } from '../components/SettingsEditor'
import clsx from 'clsx'

const removePeerIdFromPeerAddr = (addr: string): string => {
  const parts = addr.split('/')
  if (parts.length < 2) {
    return addr
  }

  return parts.slice(0, -1).join('/')
}

const Peers_Peer: React.FC<{
  displayName: string
  peerId: string
  ping: React.ReactNode
  since: string
  connectedAddrs: string[]
  disconnectedAddrs: string[]
  issues: string[]
}> = ({ displayName, peerId, ping, since, connectedAddrs, disconnectedAddrs, issues }) => {
  const isConnected = connectedAddrs.length > 0
  const [show, setShow] = useState(false)

  return (
    <div className="w-full p-4 rounded bg-gray-100 mb-4" onClick={() => setShow(!show)}>
      <p
        className={clsx(
          'text-xl mt-0 font-medium',
          isConnected ? 'text-green-500' : 'text-red-500',
        )}
      >
        <span style={{ width: '1.5rem', display: 'inline-block' }}>{show ? '▼' : '►'}</span>
        {displayName}
      </p>
      <p>
        <code title={`PeerID: ${peerId}`}>{peerId.substring(0, 10)}…</code>
        {ping}
        <span className="text-blue-300">{since}</span>
      </p>
      {show && (
        <>
          <p>Swarm addresses:</p>
          <ul className="list-disc list-inside">
            {...connectedAddrs.map((addr) => (
              <li className="text-green-500" key={addr}>
                <code>{addr}</code>
              </li>
            ))}
            {...disconnectedAddrs.map((addr) => (
              <li key={addr}>
                <code>{addr}</code>
              </li>
            ))}
          </ul>
          <p>Recent connection issues</p>
          <ul className="list-disc list-inside">
            {...issues.map((issue) => (
              <li key={issue}>
                <code className="text-sm">{issue}</code>
              </li>
            ))}
          </ul>
        </>
      )}
    </div>
  )
}

const Peers: React.FC<{ node: ReachableNodeT }> = ({ node }) => {
  if (node.details.swarmState === null) {
    return <p className="text-yellow-500">Please wait for complete node startup...</p>
  }

  const { knownPeers, connections } = node.details.swarmState
  const formatMicros = (n: number) => (n >= 10000 ? `${Math.round(n / 1000)}ms` : `${n}µs`)
  const ping = (peer: Peer) => {
    const stats = peer.pingStats
    if (!stats) {
      return (
        <span>
          <i>no ping stats</i>
        </span>
      )
    }
    return (
      <code
        className={clsx(
          'px-8',
          stats.failureRate > 9999 || stats.decay3 > 1000000 // 1% or 1s
            ? 'text-red-500'
            : stats.failureRate > 999 || stats.decay3 > 10000 // 0.1% or 10ms
            ? 'text-yellow-500'
            : 'text-green-500',
        )}
      >
        ping {formatMicros(stats.current)} with{' '}
        {(stats.failureRate / 1000000).toLocaleString(undefined, {
          maximumFractionDigits: 1,
          style: 'percent',
        })}{' '}
        loss
      </code>
    )
  }
  const since = (peer: Peer) => {
    const conns = connections.map((c) => (c.peerId === peer.peerId ? c.since : 'A'))
    conns.sort()
    const since = conns.shift()
    if (since === undefined || since === 'A') return ''
    const date = Date.parse(since)
    let diff = Date.now() - date
    let ret = ''
    if (diff >= 86400000) {
      const days = Math.floor(diff / 86400000)
      ret += ` ${days}d`
      diff -= days * 86400000
    }
    if (diff >= 3600000) {
      const hours = Math.floor(diff / 3600000)
      ret += ` ${hours}h`
      diff -= hours * 3600000
    }
    if (diff >= 60000) {
      const minutes = Math.floor(diff / 60000)
      ret += ` ${minutes}m`
      diff -= minutes * 60000
    }
    if (diff >= 1000) {
      const seconds = Math.floor(diff / 1000)
      ret += ` ${seconds}s`
    }
    return ret
  }
  const connected = (peer: Peer) => {
    const c = Object.keys(
      connections.reduce((acc: Record<string, null>, conn) => {
        if (conn.peerId === peer.peerId) acc[conn.addr] = null
        return acc
      }, {}),
    )
    c.sort()
    return c.map((conn) => (peer.addrs.includes(conn) ? conn : conn + ' (NAT)'))
  }
  const disconnected = (peer: Peer) => {
    const c = peer.addrs.filter((addr) =>
      connections.every((conn) => conn.peerId !== peer.peerId || conn.addr !== addr),
    )
    c.sort()
    return c
  }

  return (
    <div>
      {...knownPeers.map((p) => (
        <Peers_Peer
          key={p.peerId}
          peerId={p.peerId}
          displayName={p.info?.agentVersion || p.peerId}
          ping={ping(p)}
          since={since(p)}
          connectedAddrs={connected(p)}
          disconnectedAddrs={disconnected(p)}
          issues={p.failures?.map((f) => `[${f.time}] ${f.addr}: ${f.display}`) || []}
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
