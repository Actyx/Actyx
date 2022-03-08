import { EpochMs, UniqueId } from './base'
import {
  Actyx as SDK,
  AqlEventMessage,
  AqlOffsetsMsg,
  AqlResponse,
  CancelSubscription,
  OffsetMap,
} from '@actyx/sdk'
import {
  LWW_TAG,
  LWW_CREATED_TAG,
  LWW_UPDATED_TAG,
  LWW_VERSION,
  LWW_CUSTOM_TAG_PREFIX,
} from './consts'
import { mkUniqueId } from './uuid'
import { toError } from './util'
import debug from 'debug'
import * as R from 'ramda'

const dbg = debug(`actyx:lww:debug`)
const trc = debug(`actyx:lww:trace`)

export type InstanceId = string
type EntityName = string

type PickType<T, V> = {
  [P in keyof T as T[P] extends V ? P : never]: T[P]
}
type PickAqlTypes<T> = PickType<T, AqlFilterTypes>
type AqlFilterTypes = string | number | boolean

export type Metadata = {
  id: InstanceId
  entity: EntityName
  archived: boolean
  createdOn: EpochMs
  lastUpdatedOn?: EpochMs
}

export type State<Data> = {
  data: Data
  meta: Metadata
  tags: string[]
}

type CreatedE<Data> = {
  readonly key: 'created'
  meta: {
    id: InstanceId
    entity: EntityName
    createdOn: EpochMs
    archived: false
  }
  data: Data
}
type UpdatedE<Data> = {
  readonly key: 'updated'
  meta: {
    id: InstanceId
    entity: EntityName
    updatedOn: EpochMs
    archived: boolean
  }
  data: Data
}

type Tags = {
  create: (id: UniqueId, customTags: string[]) => string[]
  update: (id: UniqueId, customTags: string[]) => string[]
  readIds: (customTags: string[]) => string[]
  custom: (customTags: string[]) => string[]
  getCustomTags: (tags: readonly string[]) => string[]
}

const Tags = (entityName: EntityName): Tags => ({
  create: (id, customTags: string[]) => [
    entityName,
    `${entityName}:${id}`,
    LWW_TAG,
    `${LWW_TAG}-${LWW_VERSION}`,
    LWW_CREATED_TAG,
    ...Tags(entityName).custom(customTags),
  ],
  update: (id, customTags) => [
    entityName,
    `${entityName}:${id}`,
    LWW_TAG,
    `${LWW_TAG}-${LWW_VERSION}`,
    LWW_UPDATED_TAG,
    ...Tags(entityName).custom(customTags),
  ],
  readIds: (customTags: string[]) => [
    LWW_TAG,
    `${LWW_TAG}-${LWW_VERSION}`,
    LWW_CREATED_TAG,
    entityName,
    ...Tags(entityName).custom(customTags),
  ],
  custom: (customTags: string[]) => customTags.map((t) => `${LWW_CUSTOM_TAG_PREFIX}${t}`),
  getCustomTags: (tags: readonly string[]) =>
    tags
      .filter((t) => t.startsWith(LWW_CUSTOM_TAG_PREFIX))
      .map((t) => t.substring(LWW_CUSTOM_TAG_PREFIX.length)),
})

export type Lww<Data> = (sdk: SDK) => {
  create: (
    data: Data,
    opts?: {
      id?: UniqueId
      tags?: string[]
    },
  ) => Promise<UniqueId>
  update: (id: UniqueId, data: Data) => Promise<boolean>
  archive: (id: UniqueId) => Promise<boolean>
  unarchive: (id: UniqueId) => Promise<boolean>
  read: (id: UniqueId) => Promise<State<Data> | undefined>
  readAll: (opts?: { tags?: string[] }) => Promise<State<Data>[]>
  readIds: (opts?: { tags?: string[] }) => Promise<UniqueId[]>
  find: (
    props: Partial<Data>,
    opts?: {
      tags?: string[]
    },
  ) => Promise<State<Data>[]>
  findOne: (
    props: Partial<Data>,
    opts?: {
      tags?: string[]
    },
  ) => Promise<State<Data> | undefined>
  subscribeIds: (
    onId: (id: UniqueId) => void,
    onError: (error: Error) => void,
    opts?: {
      tags?: string[]
    },
  ) => CancelSubscription
  subscribe: (
    id: UniqueId,
    onState: (state: State<Data>) => void,
    onError: (error: Error) => void,
  ) => CancelSubscription
  subscribeAll: (
    onStates: (states: State<Data>[]) => void,
    onError: (error: Error) => void,
    opts?: {
      tags?: string[]
    },
  ) => CancelSubscription

  // Does this actually work? It uses an AQL filter, but the
  // problem is that given the pre-set filter, a change which
  // leads to the entity no longer meeting the filter conditions
  // won't be noticed in the subscription. I.e. you will never
  // find our if an entity no longer meets the criteria.
  //
  // Commenting out for now
  //subscribeFind: (
  //  props: Partial<PickAqlTypes<Data>>,
  //  onStates: (states: State<Data>[]) => void,
  //  onError: (error: Error) => void,
  //
  //) => CancelSubscription

  // Idea: subscribeFindOne, but that is pretty tricky because
  // how do you deal with the situation where you have selected
  // "the one" and that one then, because of another update, no
  // longer matches the criteria.
}
const createEntity = async <Data>(
  sdk: SDK,
  entityName: EntityName,
  data: Data,
  opts?: {
    id?: UniqueId
    tags?: string[]
  },
): Promise<UniqueId> => {
  const entityId = opts?.id || mkUniqueId()
  const customTags = opts?.tags || []
  const event: CreatedE<Data> = {
    key: 'created',
    meta: {
      id: entityId,
      createdOn: Date.now(),
      entity: entityName,
      archived: false,
    },
    data,
  }
  await sdk.publish({ tags: Tags(entityName).create(entityId, customTags), event })
  return entityId
}
const readById = async <Data>(sdk: SDK, entityName: EntityName, id: InstanceId) => {
  const res = await _readById<Data>(sdk, entityName, id)
  if (!res) {
    return res
  }
  return res.state
}

// Read latest and return state and offset map by Id
const _readById = async <Data>(
  sdk: SDK,
  entityName: EntityName,
  id: InstanceId,
): Promise<undefined | { state: State<Data>; offsets: OffsetMap }> => {
  dbg(`_readById(entityName: '${entityName}', id: ${id})`)
  const query = `FEATURES(zøg aggregate) FROM allEvents & '${LWW_TAG}' & '${LWW_TAG}-${LWW_VERSION}' & ('${LWW_CREATED_TAG}' | '${LWW_UPDATED_TAG}') & '${entityName}' & '${entityName}:${id}' AGGREGATE LAST(_)`
  dbg(`query: ${query.trim()}`)

  dbg(`running query "${query}"`)
  const results = await sdk.queryAql(query)
  dbg(`query "${query}" returned ${results.length} results`)
  trc(results)
  const event = results.find((r: AqlResponse): r is AqlEventMessage => r.type === 'event')
  if (!event) {
    return undefined
  }
  const offsets = results.find((r: AqlResponse): r is AqlOffsetsMsg => r.type === 'offsets')
  if (!offsets) {
    throw new Error(`internal error in _readById; queryAql did not return an 'offsets' event`)
  }
  const state = {
    ...(event.payload as Omit<State<Data>, 'tags'>),
    tags: Tags(entityName).getCustomTags(event.meta.tags),
  }
  return { offsets: offsets.offsets, state }
}

const _readIds = async (
  sdk: SDK,
  entityName: EntityName,
  customTags: string[],
): Promise<{ ids: UniqueId[]; offsets: OffsetMap }> => {
  dbg(`_readIds(entityName: '${entityName}')`)
  const tags = Tags(entityName).readIds(customTags)
  const query = `FROM allEvents & ${tags.map((t) => `'${t}'`).join(' & ')}`

  dbg(`running query "${query}"`)
  const results = await sdk.queryAql(query)
  dbg(`query "${query}" returned ${results.length} results`)
  trc(results)
  const ids = results
    .filter((r: AqlResponse): r is AqlEventMessage => r.type === 'event')
    .map((e: any) => (e.payload as State<unknown>).meta.id)
  const offsets = results.find((r: AqlResponse): r is AqlOffsetsMsg => r.type === 'offsets')
  if (!offsets) {
    throw new Error(`internal error _readIds; queryAql did not return an 'offsets' event`)
  }

  return { ids, offsets: offsets.offsets }
}

const readIds = async (
  sdk: SDK,
  entityName: EntityName,
  customTags: string[],
): Promise<UniqueId[]> => {
  dbg(`readIds(entityName: '${entityName}')`)
  const res = await _readIds(sdk, entityName, customTags)
  return res.ids
}

// Read latest and return state and offset map for all
const _readAll = async <Data>(
  sdk: SDK,
  entityName: EntityName,
  customTags: string[],
): Promise<{ states: State<Data>[]; offsets: OffsetMap }> => {
  dbg(`readAll(entityName: '${entityName}')`)
  const all = await _readIds(sdk, entityName, customTags)
  dbg(`got ${all.ids.length} ids`)
  return {
    offsets: all.offsets,
    states: (await Promise.all(all.ids.map((id) => readById<Data>(sdk, entityName, id)))).filter(
      (state: State<Data> | undefined): state is State<Data> => state !== undefined,
    ),
  }
}

// Read latest and return state and offset map for all
const readAll = async <Data>(
  sdk: SDK,
  entityName: EntityName,
  customTags: string[],
): Promise<State<Data>[]> => {
  dbg(`readAll(entityName: '${entityName}')`)
  const ids = await readIds(sdk, entityName, customTags)
  dbg(`got ${ids.length} ids`)
  return (await Promise.all(ids.map((id) => readById<Data>(sdk, entityName, id)))).filter(
    (state: State<Data> | undefined): state is State<Data> => state !== undefined,
  )
}
const subscribeIds = (
  sdk: SDK,
  entityName: EntityName,
  onId: (id: UniqueId) => void,
  onError: (error: Error) => void,
  customTags: string[],
): CancelSubscription => {
  dbg(`subscribeIds(entityName: '${entityName}')`)
  let cancelled = false
  let cancelSub: CancelSubscription | undefined = undefined

  const doCancel = () => {
    cancelled = true
    if (cancelSub) {
      cancelSub()
      cancelSub = undefined
    }
  }

  _readIds(sdk, entityName, customTags).then(({ ids, offsets }) => {
    dbg(`got ${ids.length} initial ids`)
    ids.forEach(onId)

    //const query = `FROM allEvents & '${LWW_TAG}' & '${LWW_TAG}-${LWW_VERSION}' & '${LWW_CREATED_TAG}' & '${entityName}'`
    const tags = Tags(entityName).readIds(customTags)
    const query = `FROM allEvents & ${tags.map((t) => `'${t}'`).join(' & ')}`
    dbg(`subscribing with query: ${query}`)
    cancelSub = sdk.subscribeAql(
      query,
      (res) => {
        if (res.type === 'event') {
          if (!cancelled) {
            trc(`subscription "${query}" got new result`, res)
            onId((res.payload as State<unknown>).meta.id)
          } else {
            trc(`subscription "${query}" got new result but isn't calling callback since cancelled`)
          }
        } else {
          trc(`subscription "${query}" got non-event result`, res)
        }
      },
      (err) => {
        doCancel()
        onError(toError(err))
      },
      offsets,
    )
  })

  return doCancel
}

const subscribeAll = <Data>(
  sdk: SDK,
  entityName: EntityName,
  onStates: (states: State<Data>[]) => void,
  onError: (error: Error) => void,
  customTags: string[],
): CancelSubscription => {
  dbg(`subscribeAll(entityName: '${entityName}')`)

  let cancelled = false
  const states: Map<InstanceId, State<Data>> = new Map()
  let cancelSubs: CancelSubscription[] = []
  const receivedInitialStates: Map<UniqueId, boolean> = new Map()

  const handleError = (err: Error) => {
    dbg(`subscribeAll(entityName: '${entityName}') got error`, err)
    if (!cancelled) {
      onError(err)
    }
    doCancel()
  }

  const handleState = (state: State<Data>) => {
    trc(`handling new state`, state, receivedInitialStates)
    states.set(state.meta.id, state)
    // This ensure we don't call onStates for every initial state
    if (
      !cancelled &&
      Array.from(receivedInitialStates.values()).findIndex((v) => v === false) === -1
    ) {
      onStates([...states.values()])
    } else {
      trc(
        `not calling onStates yet since we haven't received all initial states`,
        receivedInitialStates,
      )
    }
  }

  const doCancel: CancelSubscription = () => {
    trc(`subscribeAll(entityName: '${entityName}') is cancelling`)
    cancelled = true
    cancelSubs.forEach((cancel) => cancel())
    cancelSubs = []
  }

  cancelSubs.push(
    subscribeIds(
      sdk,
      entityName,
      (id) => {
        let isFirst = true
        trc(`adding id ${id} to receivedInitialStates`)
        receivedInitialStates.set(id, false)
        cancelSubs.push(
          subscribeById<Data>(
            sdk,
            entityName,
            id,
            (state) => {
              trc(`subscription to ${id} got new state`, state)
              if (isFirst) {
                trc(`is first result, so adding setting receivedInitialStates accordingly`)
                receivedInitialStates.set(id, true)
              }
              isFirst = false
              handleState(state)
            },
            handleError,
          ),
        )
      },
      handleError,
      customTags,
    ),
  )

  return doCancel
}

const subscribeById = <Data>(
  sdk: SDK,
  entityName: EntityName,
  id: UniqueId,
  onState: (state: State<Data>) => void,
  onError: (error: Error) => void,
): CancelSubscription => {
  dbg(`subscribeById(entityName: '${entityName}', id: '${id}')`)
  let cancelled = false
  let cancelSub: undefined | CancelSubscription = undefined
  _readById<Data>(sdk, entityName, id)
    .then((res) => {
      trc(`subscribeById(entityName: '${entityName}', id: '${id}') got first read result`, res)
      if (!res) {
        if (!cancelled) {
          onError(new Error(`no ${entityName} entity with id ${id} found; cannot subscribe`))
        }
        return
      }
      if (!cancelled) {
        onState(res.state)
      } else {
        trc(
          `subscribeById(entityName: '${entityName}', id: '${id}') has been cancelled, so not calling onState`,
        )
      }
      const query = `FROM allEvents & '${LWW_TAG}' & '${LWW_TAG}-${LWW_VERSION}' & ('${LWW_CREATED_TAG}' | '${LWW_UPDATED_TAG}') & '${entityName}' & '${entityName}:${id}'`
      dbg(`subscribing with query "${query}"`)
      cancelSub = sdk.subscribeAql(
        query,
        (res) => {
          trc(`subscription with query "${query}" got result`, res)
          if (res.type === 'event' && !cancelled) {
            onState({
              ...(res.payload as Omit<State<Data>, 'tags'>),
              tags: Tags(entityName).getCustomTags(res.meta.tags),
            })
          }
        },
        (err) => {
          if (!cancelled) {
            onError(toError(err))
          }
        },
        res.offsets,
      )
    })
    .catch((err) => {
      if (!cancelled) {
        onError(toError(err))
      }
    })

  return () => {
    cancelled = true
    if (cancelSub) {
      cancelSub()
      cancelSub = undefined
    }
  }
}

const update = async <Data>(
  sdk: SDK,
  entityName: EntityName,
  id: InstanceId,
  data: Data,
): Promise<boolean> => {
  const current = await readById<Data>(sdk, entityName, id)
  if (!current) {
    return false
  }

  const updatedOn = Date.now()
  const tags = Tags(entityName).update(id, current.tags)
  const event: UpdatedE<Data> = {
    key: 'updated',
    meta: {
      ...current.meta,
      updatedOn,
    },
    data,
  }
  await sdk.publish({ tags, event })

  return true
}

const setArchivedState = async (
  sdk: SDK,
  entityName: EntityName,
  id: InstanceId,
  to: boolean,
): Promise<boolean> => {
  const current = await readById<unknown>(sdk, entityName, id)
  if (!current) {
    return false
  }
  if (current.meta.archived === to) {
    return false
  }

  const tags = Tags(entityName).update(id, current.tags)
  const event: UpdatedE<unknown> = {
    key: 'updated',
    meta: {
      ...current.meta,
      updatedOn: Date.now(),
      archived: to,
    },
    data: current.data,
  }
  await sdk.publish({ tags, event })

  return true
}

const find = async <Data>(
  sdk: SDK,
  entityName: EntityName,
  props: Partial<Data>,
  customTags: string[],
): Promise<State<Data>[]> => _find(sdk, entityName, props, customTags).then((r) => r.states)

const findOne = async <Data>(
  sdk: SDK,
  entityName: EntityName,
  props: Partial<Data>,
  customTags: string[],
): Promise<State<Data> | undefined> =>
  _find(sdk, entityName, props, customTags).then((r) =>
    r.states.length > 0 ? r.states[0] : undefined,
  )

//const _mqAqlFilter = (name: string, value: AqlFilterTypes): string => {
//  switch (typeof value) {
//    case 'string': {
//      return `_.data['${name}']='${value}'`
//    }
//    case 'number': {
//      return `_.data['${name}']=${value}`
//    }
//    case 'boolean': {
//      return `${value ? '' : '!'}_.data['${name}']`
//    }
//    default: {
//      throw new Error(
//        `unexpected got value type '${typeof value}' which is not compatible with AQL queries`,
//      )
//    }
//  }
//}

const _find = async <Data>(
  sdk: SDK,
  entityName: EntityName,
  props: Partial<Data>,
  customTags: string[],
): Promise<{ states: State<Data>[]; offsets: OffsetMap }> => {
  const all = await _readAll<Data>(sdk, entityName, customTags)
  const matches = all.states.filter((s) => {
    const sProps = R.pick(R.keys(props), s.data)
    // @ts-ignore
    return R.equals(sProps, props) // TODO: fix types
  })
  return { states: matches, offsets: all.offsets }

  // You can't do this because it will return outdated instances that match the filters. E.g. you have
  // an instance with a name. The name is currently 'a'. You change it to 'b'. This way will now find
  // the outdated instance.
  //let query = `FROM allEvents & '${LWW_TAG}' & '${LWW_TAG}-${LWW_VERSION}' & '${entityName}'`
  //if (Object.entries(props).length > 0) {
  //  query += ' FILTER '
  //  query += Object.entries(props)
  //    // Can't get the types to work here atm
  //    .map(([k, v]) => _mqAqlFilter(k, v as AqlFilterTypes))
  //    .join(' & ')
  //}
  //dbg(`finding with query "${query}"`)
  //// Reduce latest
  //const latest: Map<InstanceId, State<Data>> = new Map()
  //const results = await sdk.queryAql(query)
  //const events = results.filter((r: AqlResponse): r is AqlEventMessage => r.type === 'event')
  //trc(
  //  `find with query "${query}" got events with payloads`,
  //  events.map((e) => e.payload),
  //)
  //events.map((e: any) => e.payload as State<Data>).forEach((s) => latest.set(s.meta.id, s))
  //const offsets = results.find((r: AqlResponse): r is AqlOffsetsMsg => r.type === 'offsets')
  //if (!offsets) {
  //  throw new Error(`internal error in _find; queryAql did not return an 'offsets' event`)
  //}
  //return { states: [...latest.values()], offsets: offsets.offsets }
}

// See commend above
//const subscribeFind = <Data>(
//  sdk: SDK,
//  entityName: EntityName,
//  props: Partial<PickAqlTypes<Data>>,
//  onStates: (states: State<Data>[]) => void,
//  onError: (error: Error) => void,
//): CancelSubscription => {
//  let cancelled = false
//  let states: Map<InstanceId, State<Data>> = new Map()
//  let cancelSub: CancelSubscription | undefined = undefined
//  const handleError = (err: Error) => {
//    if (!cancelled) {
//      onError(err)
//    }
//    doCancel()
//  }
//
//  const handleState = (state: State<Data>) => {
//    console.log(`adding state`, state)
//    states.set(state.meta.id, state)
//    if (!cancelled) {
//      console.log(`calling onStates with`, [...states.values()])
//      onStates([...states.values()])
//    }
//  }
//
//  const doCancel: CancelSubscription = () => {
//    cancelled = true
//    if (cancelSub) {
//      cancelSub()
//    }
//    cancelSub = undefined
//  }
//
//  _find(sdk, entityName, props).then(({ states, offsets }) => {
//    states.forEach(handleState)
//
//    let query = `FROM allEvents & '${LWW_TAG}' & '${LWW_TAG}-${LWW_VERSION}' & '${entityName}'`
//    if (Object.entries(props).length > 0) {
//      query += ' FILTER '
//      query += Object.entries(props)
//        // Can't get the types to work here atm
//        .map(([k, v]) => _mqAqlFilter(k, v as AqlFilterTypes))
//        .join(' & ')
//    }
//
//    console.log(`query: ${query}`)
//
//    sdk.subscribeAql({
//      query,
//      lowerBound: offsets,
//      onResponse: (res) => {
//        console.log(`got res`, res)
//        if (res.type === 'event' && !cancelled) {
//          handleState(res.payload as State<Data>)
//        }
//      },
//      onError: (err) => handleError(toError(err)),
//    })
//  })
//
//  return doCancel
//}

export const Lww =
  <Data>(entityName: EntityName): Lww<Data> =>
  (sdk) => ({
    create: (data, opts) => createEntity(sdk, entityName, data, opts),
    update: (id, data) => update(sdk, entityName, id, data),
    archive: (id) => setArchivedState(sdk, entityName, id, true),
    unarchive: (id) => setArchivedState(sdk, entityName, id, false),
    read: (id) => readById(sdk, entityName, id),
    readIds: (opts) => readIds(sdk, entityName, opts?.tags || []),
    readAll: (opts) => readAll(sdk, entityName, opts?.tags || []),
    find: (props, opts) => find(sdk, entityName, props, opts?.tags || []),
    findOne: (props, opts) => findOne(sdk, entityName, props, opts?.tags || []),
    subscribe: (id, onState, onError) => subscribeById(sdk, entityName, id, onState, onError),
    subscribeAll: (onStates, onError, opts) =>
      subscribeAll(sdk, entityName, onStates, onError, opts?.tags || []),
    subscribeIds: (onId, onError, opts) =>
      subscribeIds(sdk, entityName, onId, onError, opts?.tags || []),
    //subscribeFind: (props, onStates, onError) =>
    //  subscribeFind(sdk, entityName, props, onStates, onError),
  })
