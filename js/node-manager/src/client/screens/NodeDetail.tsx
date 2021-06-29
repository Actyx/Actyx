import React, { useState } from 'react'
import { NodeType, ReachableNode as ReachableNodeT } from '../../common/types'
import { Layout } from '../components/Layout'
import { useAppState, AppActionKey } from '../app-state'
import { Error } from '../components/Error'
import { SimpleCanvas } from '../components/SimpleCanvas'
import { Button, Tabs } from '../components/basics'
import { SettingsEditor } from '..//components/SettingsEditor'
import clsx from 'clsx'

const Peers: React.FC<{ node: ReachableNodeT }> = ({
  node: {
    details: {
      swarmState: { knownPeers },
    },
  },
}) => (
  <div className="">
    <p className="pb-2">Node connected to {knownPeers.length} peers.</p>
    {knownPeers.map(({ peerId, addrs }) => (
      <p key={peerId} className="break-all pl-2 text-sm text-gray-500">
        - Node <code className="text-black">{peerId}</code> at{' '}
        <code className="text-black">{addrs.join(';')}</code>
      </p>
    ))}
  </div>
)

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
  const Row: React.FC<{ field: string; value: string; gray?: boolean }> = ({
    field,
    value,
    gray,
  }) => (
    <div className={clsx('py-2 sm:grid sm:grid-cols-3 sm:gap-4 sm:px-6', { 'bg-gray-50': gray })}>
      <dt className="text-sm font-medium text-gray-500">{field}</dt>
      <dd className="mt-1 text-sm text-gray-900 sm:mt-0 sm:col-span-2 selectable">{value}</dd>
    </div>
  )

  return (
    <div className="">
      <dl>
        <Row field="Display name" value={details.displayName} gray />
        <Row field="Bind addresses" value={details.addrs} />
        <Row field="Node ID" value={details.nodeId} gray />
        <Row field="Peer ID" value={details.swarmState.peerId} gray />
        <Row field="Running since" value={details.startedIso} />
        <Row field="Version" value={details.version} gray />
      </dl>
    </div>
  )
}

const Addresses: React.FC<{ node: ReachableNodeT }> = ({
  node: {
    details: {
      swarmState: { swarmAddrs, announceAddrs, adminAddrs },
    },
  },
}) => {
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
            <p className="pb-1">Node has {announceAddrs.length} external addresses.</p>
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
            <p className="pb-1">Node has {adminAddrs.length} admin addresses.</p>
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
