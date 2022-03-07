import crossFetch from 'cross-fetch'
import * as t from 'io-ts'
import * as E from 'fp-ts/Either'
import { lt } from 'semver'
import { EventKey, Milliseconds, OffsetMap } from '..'
import {
  InvalidateAllSnapshots,
  InvalidateSnapshots,
  RetrieveSnapshot,
  SnapshotStore,
  StoreSnapshot,
} from '../snapshotStore'
import { EventKeyIO, OffsetMapIO } from '../types/wire'
import { mkHeaders } from './utils'
import log from '../internal_common/log'

const ENDPOINT = 'blob'
const entityFolder = (api: string, semantics: string, entity: string) =>
  `${api}/${ENDPOINT}/-/@pond/snap/${semantics}/${entity}`
const versionFolder = (api: string, semantics: string, entity: string, version: number) =>
  `${api}/${ENDPOINT}/-/@pond/snap/${semantics}/${entity}/${version}`
const snapFolder = (api: string, semantics: string, entity: string, version: number, tag: string) =>
  `${api}/${ENDPOINT}/-/@pond/snap/${semantics}/${entity}/${version}/${tag}`

const Folder = t.record(
  t.string,
  t.union([
    t.type({
      type: t.literal('folder'),
    }),
    t.type({
      type: t.literal('file'),
      originalSize: t.number,
      compressedSize: t.number,
      atimeMillis: t.number,
      ctimeMillis: t.number,
    }),
  ]),
)
type Folder = t.TypeOf<typeof Folder>

const Meta = t.type({
  key: EventKeyIO,
  offsets: OffsetMapIO,
  horizon: t.union([EventKeyIO, t.undefined]),
  cycle: t.number,
})
type Meta = t.TypeOf<typeof Meta>
const mkMeta = (
  key: EventKey,
  offsets: OffsetMap,
  horizon: EventKey | undefined,
  cycle: number,
) => ({
  key,
  offsets,
  horizon,
  cycle,
})

const Persistent = t.type({
  mode: t.literal('persistent'),
  hardQuota: t.number,
})
const Elastic = t.type({
  mode: t.literal('elastic'),
  fixedAllowance: t.number,
  ceiling: t.union([t.number, t.undefined]),
  cleaningOrder: t.keyof({
    atimeAsc: 1,
    atimeDesc: 1,
    ctimeAsc: 1,
    ctimeDesc: 1,
    timeAsc: 1,
    timeDesc: 1,
    sizeAsc: 1,
    sizeDesc: 1,
  }),
})
const Elasticity = t.union([Persistent, Elastic])

const SetElasticity = t.type({
  type: t.literal('setElasticity'),
  definition: Elasticity,
})

const fetch: typeof crossFetch = async (input, init) => {
  const res = await crossFetch(input, init)
  if (!res.ok) {
    const method = init?.method || 'GET'
    throw new Error(`fetch ${method} ${input}: (${res.status}) ${await res.text()}`)
  }
  return res
}

export class BlobSnapshotStore implements SnapshotStore {
  constructor(
    private api: string,
    private currentToken: () => string,
    private currentActyxVersion: () => string,
    reservedStorage: number,
  ) {
    const headers = mkHeaders(currentToken())
    const cmd = SetElasticity.encode({
      type: 'setElasticity',
      definition: {
        mode: 'elastic',
        fixedAllowance: reservedStorage,
        ceiling: undefined,
        cleaningOrder: 'timeAsc',
      },
    })
    fetch(`${api}/${ENDPOINT}/-/@pond/snap`, {
      method: 'POST',
      headers,
      body: JSON.stringify(cmd),
    })
      .then(async (resp) => {
        if (!resp.ok) {
          const msg = await resp.text()
          log.actyx.warn(`cannot set Pond snapshot elasticity (code ${resp.status}):`, msg)
        }
      })
      .catch((e) => log.actyx.warn('cannot set Pond snapshot elasticity', e))
  }

  storeSnapshot: StoreSnapshot = async (
    semantics,
    entity,
    key,
    offsets,
    horizon,
    cycle,
    version,
    tag,
    blob,
  ) => {
    if (lt(this.currentActyxVersion(), '2.12.0')) return false

    log.http.debug('storeSnapshot start', semantics, entity, tag)
    try {
      const headers = mkHeaders(this.currentToken())

      const folder = snapFolder(this.api, semantics, entity, version, tag)
      await fetch(folder, { method: 'DELETE', headers })

      await fetch(`${folder}/blob`, { method: 'PUT', body: blob, headers })
      await fetch(`${folder}/meta`, {
        method: 'PUT',
        body: JSON.stringify(mkMeta(key, offsets, horizon, cycle)),
        headers,
      })

      const entityF = entityFolder(this.api, semantics, entity)
      const ls = Folder.decode(await (await fetch(entityF, { headers })).json())
      if (E.isRight(ls)) {
        for (const v of Object.keys(ls.right)) {
          if (Number(v) < version) {
            await fetch(`${entityF}/${v}`, { method: 'DELETE', headers })
          }
        }
      }

      return true
    } catch (e) {
      log.http.error('storeSnapshot', semantics, entity, tag, e)
      return false
    } finally {
      log.http.debug('storeSnapshot done', semantics, entity, tag)
    }
  }

  retrieveSnapshot: RetrieveSnapshot = async (semantics, entity, version) => {
    if (lt(this.currentActyxVersion(), '2.12.0')) return undefined

    log.http.debug('retrieveSnapshot start', semantics, entity)
    try {
      const headers = mkHeaders(this.currentToken())

      const folder = versionFolder(this.api, semantics, entity, version)
      const ls = Folder.decode(await (await fetch(folder, { headers })).json())
      if (E.isLeft(ls)) return

      let meta: Meta | undefined = undefined
      let blob = ''
      for (const tag of Object.keys(ls.right)) {
        const metaRes = await crossFetch(`${folder}/${tag}/meta`, { headers })
        if (!metaRes.ok) continue
        const m = Meta.decode(await metaRes.json())
        if (E.isLeft(m)) continue
        if (meta !== undefined && EventKey.ord.compare(meta.key, m.right.key) > 0) continue
        const blobRes = await crossFetch(`${folder}/${tag}/blob`, { headers })
        if (!blobRes.ok) continue
        meta = m.right
        blob = await blobRes.text()
      }

      if (meta === undefined) return
      return {
        state: blob,
        offsets: meta.offsets,
        eventKey: meta.key,
        horizon: meta.horizon,
        cycle: meta.cycle,
      }
    } catch (e) {
      log.http.error('retrieveSnapshot', semantics, entity, e)
      return
    } finally {
      log.http.debug('retrieveSnapshot done', semantics, entity)
    }
  }

  invalidateSnapshots: InvalidateSnapshots = async (semantics, entity, key) => {
    if (lt(this.currentActyxVersion(), '2.12.0')) return

    log.http.debug('invalidateSnapshots start', semantics, entity, key)
    try {
      const headers = mkHeaders(this.currentToken())

      const folder = entityFolder(this.api, semantics, entity)
      const ls = Folder.decode(await (await fetch(folder, { headers })).json())
      if (E.isLeft(ls)) return

      for (const version of Object.keys(ls)) {
        const vf = `${folder}/${version}`
        const vfls = Folder.decode(await (await fetch(vf, { headers })).json())
        if (E.isLeft(vfls)) continue

        for (const tag of Object.keys(vfls)) {
          const meta = Meta.decode(await (await fetch(`${vf}/${tag}/meta`, { headers })).json())
          if (E.isLeft(meta)) continue

          if (EventKey.ord.compare(key, meta.right.key) < 0) {
            await fetch(`${vf}/${tag}`, { method: 'DELETE', headers })
          }
        }
      }
    } catch (e) {
      log.http.error('invalidateSnapshot', semantics, entity, key, e)
    } finally {
      log.http.debug('invalidateSnapshots', semantics, entity, key)
    }
  }

  invalidateAllSnapshots: InvalidateAllSnapshots = async () => {
    if (lt(this.currentActyxVersion(), '2.12.0')) return
    try {
      const headers = mkHeaders(this.currentToken())
      await fetch(`${this.api}/${ENDPOINT}/-/@pond/snap`)
    } catch (e) {
      log.http.error('invalidateAllSnapshots', e)
    }
  }
}
