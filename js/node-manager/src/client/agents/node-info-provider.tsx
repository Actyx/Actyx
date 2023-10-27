// add nodes
// rem nodes
// restart nodes

import { getNodeDetails } from '../util'
import { Serv } from '../util/serv'
import { GetNodeDetailsResponse, NodeType, ReachableNodeUi } from '../../common/types/nodes'
import { ipcRenderer } from 'electron'
import { DEFAULT_TIMEOUT_SEC } from '../../common/consts'
import deepEqual from 'deep-equal'
import { sleep } from '../../common/util'

const POLLING_INTERVAL_MS = 1000

export type NodeInfoProvider = ReturnType<typeof NodeInfoProvider>
export const NodeInfoProvider = ({
  addr,
  peer,
  emitNodeInfoChange,
  emitDisconnect,
}: {
  addr: string
  peer: string
  emitNodeInfoChange: () => unknown
  emitDisconnect: () => unknown
}) =>
  Serv.build()
    .api(({ isDestroyed }) => {
      const data = Object.seal({
        nodeInfo: null as null | GetNodeDetailsResponse,
      })

      // const getTimeoutSec =
      //   (store.key === StoreStateKey.Loaded && store.data.preferences.nodeTimeout) ||
      //   DEFAULT_TIMEOUT_SEC
      const getTimeoutSec = DEFAULT_TIMEOUT_SEC

      ;(async () => {
        const listener = (event: Electron.IpcRendererEvent, incomingPeer: string) => {
          if (incomingPeer !== peer) return
          data.nodeInfo = { type: NodeType.Disconnected, peer }
          emitDisconnect()
        }

        ipcRenderer.on('onDisconnect', listener)
        while (!isDestroyed()) {
          const detail = await sleep(Math.round(Math.random() * 0)).then(() =>
            getNodeDetails({ peer, timeout: getTimeoutSec }),
          )
          if (!deepEqual(detail, data.nodeInfo) && !isDestroyed()) {
            console.log(`+++ updating app-state/nodes +++`, detail)
            data.nodeInfo = detail
            emitNodeInfoChange()
          }
          await sleep(POLLING_INTERVAL_MS)
        }
        console.log('node provider destroyed', addr)
        ipcRenderer.off('onDisonnect', listener)
      })()

      return {
        get: (): null | GetNodeDetailsResponse => data.nodeInfo,
        getAsReachableNodeUi: (): null | ReachableNodeUi => {
          const info = data.nodeInfo
          if (info?.type !== NodeType.Reachable) return null
          return { ...info, addr, timeouts: 0 }
        },
      }
    })
    .finish()
