import React, { useEffect } from 'react'
import { NodeType, Node } from '../../common/types'
import { nodeAddrValid } from '../../common/util'
import { Layout } from '../components/Layout'
import { useAppState, AppActionKey } from '../app-state'
import clsx from 'clsx'
import { Button } from '../components/basics'
import { SolidStarIcon, UnsolidStarIcon } from '../components/icons'
import { useStore } from '../store'
import { StoreState } from '../store/types'

const nodeTypeToText = (type: NodeType) => {
  switch (type) {
    case NodeType.Reachable:
      return 'Connected'
    case NodeType.Unauthorized:
      return 'Not authorized'
    case NodeType.Unreachable:
      return 'Disconnected'
    case NodeType.Loading:
      return 'Loading'
  }
}

const isFavorite = (store: StoreState, node: Node) =>
  store.key === 'Loaded' && store.data.preferences.favoriteNodeAddrs.includes(node.addr)

const NodeCard: React.FC<{ node: Node; remove: () => void; view: () => void }> = ({
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
    <div key={node.addr} className="bg-white rounded p-4 w-56 mr-6 mb-6 relative">
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
            'text-green-300': node.type === NodeType.Reachable,
            'text-gray-300': node.type === NodeType.Loading,
          })}
        >
          {nodeTypeToText(node.type)}
        </span>
        {node.type === NodeType.Reachable && <span>&nbsp;{node.addr}</span>}
      </p>
      <p className="text-xl mt-0 font-medium truncate overflow-ellipsis">
        {node.type === NodeType.Reachable ? node.details.displayName : node.addr}
      </p>
      <div className="mt-6 flex">
        {node.type === NodeType.Reachable && (
          <Button color="blue" className="mr-2" onClick={view}>
            Details
          </Button>
        )}
        <Button color="gray" onClick={remove} disabled={isFavorite(store, node)}>
          Remove
        </Button>
      </div>
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
      <div className="flex flex-wrap flex-row">
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
