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
import { ObsValcon } from '../util/valcon'

const POLLING_INTERVAL_MS = 1000

export type NodeInfo = GetNodeDetailsResponse & { timeouts: number }
export type NodeInfoProvider = ReturnType<typeof NodeInfoProvider>
export const NodeInfoProvider = ({
  addr,
  peer,
  emitNodeInfoChange,
  emitDisconnect,
  timeoutRef,
}: {
  addr: string
  peer: string
  emitNodeInfoChange: () => unknown
  emitDisconnect: () => unknown
  timeoutRef: ObsValcon<number | null>
}) =>
  Serv.build()
    .api(({ isDestroyed }) => {
      const data = Object.seal({
        nodeInfo: null as null | NodeInfo,
      })

      const onDisconnect = (_: Electron.IpcRendererEvent, incomingPeer: string) => {
        if (incomingPeer !== peer) return
        emitDisconnect()
      }

      ;(async () => {
        ipcRenderer.on('onDisconnect', onDisconnect)
        while (!isDestroyed()) {
          const getTimeoutSec = timeoutRef.get() || DEFAULT_TIMEOUT_SEC
          const detail: null | NodeInfo = await getNodeDetails({
            peer,
            timeout: getTimeoutSec,
          })
            .then((res) => ({
              ...res,
              timeouts: 0,
            }))
            .catch(() => {
              const previousDetail = data.nodeInfo
              if (!previousDetail) {
                return null
              }
              return {
                ...previousDetail,
                timeouts: previousDetail.timeouts + 1,
              }
            })
          if (!deepEqual(detail, data.nodeInfo) && !isDestroyed()) {
            console.log(`+++ updating app-state/nodes +++`, detail)
            data.nodeInfo = detail
            emitNodeInfoChange()
          }
          await sleep(POLLING_INTERVAL_MS)
        }
      })().finally(() => {
        console.log('node provider destroyed', addr)
        ipcRenderer.off('onDisonnect', onDisconnect)
      })

      return {
        get: (): null | NodeInfo => data.nodeInfo,
        getAsReachableNodeUi: (): null | ReachableNodeUi => {
          const info = data.nodeInfo
          if (info?.type !== NodeType.Reachable) return null
          return { ...info, addr, timeouts: 0 }
        },
      }
    })
    .finish()
