import React, { useEffect, useMemo, useState } from 'react'
import { NodeType, UiNode } from '../../common/types'
import { nodeAddrValid } from '../../common/util'
import { Layout } from '../components/Layout'
import { useAppState, AppActionKey } from '../app-state'
import clsx from 'clsx'
import { Button } from '../components/basics'
import { SolidStarIcon, UnsolidStarIcon } from '../components/icons'
import { useStore } from '../store'
import { StoreState } from '../store/types'
import { connect, getNodeDetails } from '../util'
import deepEqual from 'deep-equal'

const ErrorSpan: React.FC<{ error: string }> = ({ children, error }) => {
  const [open, setOpen] = useState(false)
  return (
    <span onClick={() => setOpen(!open)}>
      {children}
      {open ? (
        <p
          style={{
            position: 'absolute',
            top: '1ex',
            left: '1ex',
            right: '1ex',
            border: '1px solid #555',
            backgroundColor: 'white',
            overflowX: 'hidden',
            whiteSpace: 'break-spaces',
            zIndex: 2,
          }}
        >
          {error}
        </p>
      ) : (
        <></>
      )}
    </span>
  )
}

const nodeToStatusText = (node: UiNode) => {
  switch (node.type) {
    case NodeType.Reachable: {
      if (node.details.swarmState === null) {
        return 'Starting'
      } else {
        return 'Connected'
      }
    }
    case NodeType.Unauthorized:
      return 'Not authorized'
    case NodeType.Unreachable:
      return <ErrorSpan error={node.error}>Unreachable</ErrorSpan>
    case NodeType.Fresh:
      return 'Just added'
    case NodeType.Disconnected:
      return 'Disconnected'
    case NodeType.Connecting:
      return <ErrorSpan error={node.prevError || 'no error'}>Connecting</ErrorSpan>
    case NodeType.Connected:
      return 'Connected'
  }
}

const isFavorite = (store: StoreState, node: UiNode) =>
  store.key === 'Loaded' && store.data.preferences.favoriteNodeAddrs.includes(node.addr)

const NodeCard: React.FC<{ node: UiNode; remove: () => void; view: () => void }> = ({
  node,
  remove,
  view,
}) => {
  const store = useStore()

  const toggleFavorite = () => {
    isFavorite(store, node) ? unmkFavorite() : mkFavorite()
  }

  const mkFavorite = () => {
    if (store.key !== 'Loaded') {
      return
    }
    store.actions.updateAndReload({
      ...store.data,
      preferences: {
        ...store.data.preferences,
        favoriteNodeAddrs: store.data.preferences.favoriteNodeAddrs.concat([node.addr]),
      },
    })
  }

  const unmkFavorite = () => {
    if (store.key !== 'Loaded') {
      return
    }
    store.actions.updateAndReload({
      ...store.data,
      preferences: {
        ...store.data.preferences,
        favoriteNodeAddrs: store.data.preferences.favoriteNodeAddrs.filter(
          (addr) => addr !== node.addr,
        ),
      },
    })
  }

  return (
    <div key={node.addr} className="bg-white rounded p-4 w-56 relative">
      <div className="absolute top-3 right-3 cursor-pointer" onClick={toggleFavorite}>
        {isFavorite(store, node) ? (
          <SolidStarIcon height={4} width={4} className="text-gray-400" />
        ) : (
          <UnsolidStarIcon height={4} width={4} className="text-gray-400" />
        )}
      </div>
      <p className="text-sm truncate overflow-ellipsis text-gray-300 pr-4">
        <span
          className={clsx('font-medium', {
            'text-red-300':
              node.type === NodeType.Unauthorized || node.type === NodeType.Unreachable,
            'text-green-300': node.type === NodeType.Reachable && node.details.swarmState !== null,
            'text-yellow-300':
              (node.type === NodeType.Reachable && node.details.swarmState === null) ||
              node.type === NodeType.Disconnected ||
              node.type === NodeType.Connecting,
            'text-gray-300': node.type === NodeType.Fresh || node.type === NodeType.Connected,
          })}
        >
          {nodeToStatusText(node)}
        </span>
        {node.type === NodeType.Reachable && <span>&nbsp;{node.addr}</span>}
      </p>
      <p className="text-xl mt-0 font-medium truncate overflow-ellipsis">
        {node.type === NodeType.Reachable && node.timeouts > 0 ? (
          <span className="text-red-500">(⏰ {node.timeouts})&nbsp;</span>
        ) : undefined}
        {node.type === NodeType.Reachable ? node.details.displayName : node.addr}
      </p>
      <div className="mt-6 flex">
        {node.type === NodeType.Reachable && (
          <Button color="blue" className="mr-2" onClick={view}>
            Details
          </Button>
        )}
        <Button
          color="gray"
          onClick={remove}
          disabled={isFavorite(store, node)}
          title={isFavorite(store, node) ? 'cannot remove starred node' : undefined}
        >
          Remove
        </Button>
      </div>
    </div>
  )
}

const PROTO = /^(.*\/tcp\/)(\d+)(\/.*$|$)/
type Peer = {
  addr: string[]
  peer: string
  displayName: string
}

const Search: React.FC<{ nodes: UiNode[]; addNodes: (addrs: string[]) => void }> = ({
  nodes,
  addNodes,
}) => {
  const [show, setShow] = useState(false)
  // one list of addresses per PeerId (which is assumed to be the same between Swarm and Admin)
  const [candidates, setCandidates] = useState<Record<string, Set<string>>>({})
  // these are the admin PeerIds
  const [peers, setPeers] = useState<Peer[]>([])
  // these are the selected addresses
  const [selected, setSelected] = useState<string[]>([])

  useEffect(() => {
    const connected = nodes
      .map((n) => (n.type === NodeType.Reachable ? n.peer : undefined))
      .filter((p): p is string => p !== undefined)
    const c = nodes
      .filter((n): n is UiNode & { type: NodeType.Reachable } => n.type === NodeType.Reachable)
      .map<Record<string, Set<string>>>((n) => {
        const peers = n.details.swarmState?.knownPeers || []
        return Object.fromEntries(
          peers
            .filter((p) => !connected.includes(p.peerId))
            .map((p) => [
              p.peerId,
              new Set(
                p.addrs
                  .concat(p.info?.listeners || [])
                  .map((addr) => {
                    const match = PROTO.exec(addr)
                    return match === null || Number(match[2]) < 4001 || Number(match[2]) > 4450
                      ? []
                      : [match[1] + (Number(match[2]) + 457) + match[3]]
                  })
                  .flat(),
              ),
            ]),
        )
      })
      .reduce((acc: Record<string, Set<string>>, rec) => {
        for (const [peer, addrs] of Object.entries(rec)) {
          const a = [...(acc[peer] || [])]
          const b = a.concat(...addrs)
          acc[peer] = new Set(b)
        }
        return acc
      }, {})
    setCandidates((cand) => (deepEqual(cand, c) ? cand : c))
  }, [nodes])

  useEffect(() => {
    const addNode = (addr: string, peer: string, displayName: string) => {
      setPeers((peers) => {
        if (nodes.find((n) => 'peer' in n && n.peer === peer)) return peers
        const p = peers.find((p) => p.peer === peer) || { addr: [], peer, displayName }
        if (p.addr.length === 0) {
          peers.push(p)
        }
        if (p.addr.includes(addr)) return peers
        console.log('adding', addr, peer, displayName)
        p.addr.push(addr)
        p.addr.sort()
        peers.sort((l, r) => (l.peer < r.peer ? -1 : l.peer > r.peer ? 1 : 0))
        return [...peers]
      })
    }
    nodes.forEach((n) => {
      if ('peer' in n) {
        setPeers((peers) => {
          const pos = peers.findIndex((p) => p.peer === n.peer)
          if (pos < 0) return peers
          const peer = peers[pos]
          setSelected((sel) => {
            let modified = false
            for (const addr of peer.addr) {
              const pos1 = sel.indexOf(addr)
              if (pos1 < 0) continue
              modified = true
              sel.splice(pos1, 1)
            }
            return modified ? [...sel] : sel
          })
          peers.splice(pos, 1)
          return [...peers]
        })
      }
    })
    for (const [peer, addrs] of show ? Object.entries(candidates) : []) {
      addrs.forEach(async (addr) => {
        try {
          const { peer } = await connect({ addr, timeout: 10 })
          const node = await getNodeDetails({ peer, timeout: 2 })
          node.type === NodeType.Reachable && addNode(addr, node.peer, node.details.displayName)
        } catch (e) {
          console.log('error while checking node at', addr, e)
        }
      })
    }
  }, [show, candidates, nodes])

  const select = (addr: string | null, addrs: string[]) => () => {
    setSelected((sel) => {
      const unset = sel.filter((a) => !addrs.includes(a))
      if (addr === null || sel.includes(addr)) return unset
      unset.push(addr)
      return unset
    })
  }

  return (
    <div key="/" className="bg-white rounded p-4 w-56">
      <p
        className="text-xl fond-bold italic text-gray-600 cursor-pointer"
        onClick={() => setShow(true)}
      >
        Search for other nodes …
      </p>
      {show && (
        <div
          className="absolute top-8 right-8 left-8 bottom-8 border rounded border-gray-200 bg-white"
          style={{ zIndex: 1 }}
        >
          <div className="border-b-2 border-gray-200 bg-gray-100 w-full flex items-center">
            <span className="text-xl p-4 cursor-pointer" onClick={() => setShow(false)}>
              ←
            </span>
            <div className="flex-grow text-center text-gray-300">
              {Object.values(candidates)
                .map((c) => c.size)
                .reduce((a, b) => a + b, 0)}{' '}
              address candidates
            </div>
            <Button
              className="ml-4"
              disabled={selected.length === 0}
              onClick={() => addNodes(selected)}
            >
              Add selected
            </Button>
            <Button className="m-4" onClick={() => addNodes(peers.map((p) => p.addr[0]))}>
              Add all
            </Button>
          </div>
          <div>
            {peers.length === 0 && (
              <p className="italic text-xl text-gray-300 m-8">
                This list is filled with addresses guessed by looking at the Swarm ports of peers
                connected to already added nodes, so it only works if you’re using default ports.
              </p>
            )}
            {peers.map(({ peer, addr, displayName }) => (
              <div key={peer} className="bg-white border border-gray-200 p-4 m-4 rounded">
                <p onClick={select(null, addr)} className="cursor-pointer">
                  <span
                    className={clsx(
                      'text-xl',
                      addr.some((a) => selected.includes(a)) && 'text-green-500',
                    )}
                  >
                    {displayName}
                  </span>{' '}
                  <span className="text-sm italic text-gray-300">{peer}</span>
                </p>
                {addr.map((a) => (
                  <p
                    key={a}
                    className={clsx(selected.includes(a) && 'text-green-500', 'cursor-pointer')}
                    onClick={select(a, addr)}
                  >
                    <input
                      type="radio"
                      name={peer}
                      checked={selected.includes(a)}
                      onChange={() => {
                        /* You know nuthin, Jon Snow */
                      }}
                      className="mx-4"
                    />
                    <code>{a}</code>
                  </p>
                ))}
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  )
}

const Screen: React.FC<{}> = () => {
  const {
    dispatch,
    actions: { addNodes, remNodes },
    data: { nodes },
  } = useAppState()

  const store = useStore()
  const favoriteNodeAddrs = store.key === 'Loaded' ? store.data.preferences.favoriteNodeAddrs : []

  useEffect(() => {
    addNodes(favoriteNodeAddrs.filter((a) => !nodes.map((n) => n.addr).includes(a)))
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [favoriteNodeAddrs])

  const inputValidator = (addr: string) =>
    addr !== '' && !nodes.map((n) => n.addr).includes(addr) && nodeAddrValid(addr)
  const viewNode = (addr: string) => dispatch({ key: AppActionKey.ShowNodeDetail, addr })
  const remNode = (addr: string) => remNodes([addr])

  return (
    <Layout
      title="Nodes"
      input={{
        onSubmit: (val) => addNodes([val]),
        placeholder: 'Node address',
        buttonText: 'Add node',
        validator: inputValidator,
      }}
      actions={[
        {
          target: () => remNodes(nodes.filter((n) => !isFavorite(store, n)).map((n) => n.addr)),
          text: 'Remove all nodes',
          disabled: nodes.filter((n) => !isFavorite(store, n)).length < 1,
        },
      ]}
    >
      {/* <div className="grid gap-6 grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 2xl:grid-cols-6"> */}
      <div className="grid gap-4 grid-cols-nodes">
        <Search nodes={nodes} addNodes={addNodes} />
        {nodes.map((node) => (
          <NodeCard
            key={node.addr}
            node={node}
            remove={() => remNode(node.addr)}
            view={() => viewNode(node.addr)}
          />
        ))}
      </div>
    </Layout>
  )
}

export default Screen
