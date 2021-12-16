import { EpochMs, UniqueId } from './base'
import {
  Actyx as SDK,
  AqlEventMessage,
  AqlOffsetsMsg,
  AqlResponse,
  CancelSubscription,
  OffsetMap,
} from '@actyx/sdk'
import { LWW_TAG, LWW_CREATED_TAG, LWW_UPDATED_TAG, LWW_VERSION } from './consts'
import { mkUniqueId } from './uuid'
import { toError } from './util'

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

type MkTag = {
  create: (id: UniqueId) => string[]
  update: (id: UniqueId) => string[]
}

const mkTags = (entityName: EntityName): MkTag => ({
  create: (id) => [
    entityName,
    `${entityName}:${id}`,
    LWW_TAG,
    `${LWW_TAG}-${LWW_VERSION}`,
    LWW_CREATED_TAG,
  ],
  update: (id) => [
    entityName,
    `${entityName}:${id}`,
    LWW_TAG,
    `${LWW_TAG}-${LWW_VERSION}`,
    LWW_UPDATED_TAG,
  ],
})

export type Lww<Data> = (sdk: SDK) => {
  create: (data: Data) => Promise<UniqueId>
  update: (id: UniqueId, data: Data) => Promise<boolean>
  archive: (id: UniqueId) => Promise<boolean>
  unarchive: (id: UniqueId) => Promise<boolean>
  read: (id: UniqueId) => Promise<State<Data> | undefined>
  readAll: () => Promise<State<Data>[]>
  readIds: () => Promise<UniqueId[]>
  find: (props: Partial<PickAqlTypes<Data>>) => Promise<State<Data>[]>
  findOne: (props: Partial<PickAqlTypes<Data>>) => Promise<State<Data> | undefined>
  subscribeIds: (
    onId: (id: UniqueId) => void,
    onError: (error: Error) => void,
  ) => CancelSubscription
  subscribe: (
    id: UniqueId,
    onState: (state: State<Data>) => void,
    onError: (error: Error) => void,
  ) => CancelSubscription
  subscribeAll: (
    onStates: (states: State<Data>[]) => void,
    onError: (error: Error) => void,
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
): Promise<UniqueId> => {
  const id = mkUniqueId()
  const event: CreatedE<Data> = {
    key: 'created',
    meta: {
      id,
      createdOn: Date.now(),
      entity: entityName,
      archived: false,
    },
    data,
  }
  await sdk.publish({ tags: mkTags(entityName).create(id), event })
  return id
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
  const query = `
			FEATURES(zøg aggregate)
			FROM allEvents & '${LWW_TAG}' & '${LWW_TAG}-${LWW_VERSION}' & ('${LWW_CREATED_TAG}' | '${LWW_UPDATED_TAG}') & '${entityName}' & '${entityName}:${id}'
			AGGREGATE
			LAST(_)
		`

  const results = await sdk.queryAql(query)
  const event = results.find((r: AqlResponse): r is AqlEventMessage => r.type === 'event')
  if (!event) {
    return undefined
  }
  const offsets = results.find((r: AqlResponse): r is AqlOffsetsMsg => r.type === 'offsets')
  if (!offsets) {
    throw new Error(`internal error; queryAql did not return an 'offsets' event`)
  }
  const state = event.payload as State<Data>
  return { offsets: offsets.offsets, state }
}

const _readIds = async (
  sdk: SDK,
  entityName: EntityName,
): Promise<{ ids: UniqueId[]; offsets: OffsetMap }> => {
  const query = `
			FROM allEvents & '${LWW_TAG}' & '${LWW_TAG}-${LWW_VERSION}' & '${LWW_CREATED_TAG}' & '${entityName}'
		`
  const results = await sdk.queryAql(query)
  const ids = results
    .filter((r: AqlResponse): r is AqlEventMessage => r.type === 'event')
    .map((e: any) => (e.payload as State<unknown>).meta.id)
  const offsets = results.find((r: AqlResponse): r is AqlOffsetsMsg => r.type === 'offsets')
  if (!offsets) {
    throw new Error(`internal error; queryAql did not return an 'offsets' event`)
  }

  return { ids, offsets: offsets.offsets }
}

const readIds = async (sdk: SDK, entityName: EntityName): Promise<UniqueId[]> => {
  const res = await _readIds(sdk, entityName)
  if (!res) {
    return res
  }
  return res.ids
}

// Read latest and return state and offset map for all
const readAll = async <Data>(sdk: SDK, entityName: EntityName): Promise<State<Data>[]> => {
  const ids = await readIds(sdk, entityName)
  return (await Promise.all(ids.map((id) => readById<Data>(sdk, entityName, id)))).filter(
    (state: State<Data> | undefined): state is State<Data> => state !== undefined,
  )
}
const subscribeIds = (
  sdk: SDK,
  entityName: EntityName,
  onId: (id: UniqueId) => void,
  onError: (error: Error) => void,
): CancelSubscription => {
  let cancelled = false
  let cancelSub: CancelSubscription | undefined = undefined

  const doCancel = () => {
    cancelled = true
    if (cancelSub) {
      cancelSub()
      cancelSub = undefined
    }
  }

  _readIds(sdk, entityName).then(({ ids, offsets }) => {
    ids.forEach(onId)

    cancelSub = sdk.subscribeAql({
      query: `
			FROM allEvents & '${LWW_TAG}' & '${LWW_TAG}-${LWW_VERSION}' & '${LWW_CREATED_TAG}' & '${entityName}'
      `,
      onResponse: (res: any) => {
        if (res.type === 'event' && !cancelled) {
          onId((res.payload as State<unknown>).meta.id)
        }
      },
      lowerBound: offsets,
      onError: (err: any) => {
        doCancel()
        onError(toError(err))
      },
    })
  })

  return doCancel
}

const subscribeAll = <Data>(
  sdk: SDK,
  entityName: EntityName,
  onStates: (states: State<Data>[]) => void,
  onError: (error: Error) => void,
): CancelSubscription => {
  let cancelled = false
  const states: Map<InstanceId, State<Data>> = new Map()
  let cancelSubs: CancelSubscription[] = []
  const handleError = (err: Error) => {
    if (!cancelled) {
      onError(err)
    }
    doCancel()
  }

  const handleState = (state: State<Data>) => {
    states.set(state.meta.id, state)
    if (!cancelled) {
      onStates([...states.values()])
    }
  }

  const doCancel: CancelSubscription = () => {
    cancelled = true
    cancelSubs.forEach((cancel) => cancel())
    cancelSubs = []
  }

  cancelSubs.push(
    subscribeIds(
      sdk,
      entityName,
      (id) => {
        cancelSubs.push(subscribeById(sdk, entityName, id, handleState, handleError))
      },
      handleError,
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
  let cancelled = false
  let cancelSub: undefined | CancelSubscription = undefined
  _readById<Data>(sdk, entityName, id)
    .then((res) => {
      if (!res) {
        if (!cancelled) {
          onError(new Error(`no ${entityName} entity with id ${id} found; cannot subscribe`))
        }
        return
      }
      if (!cancelled) {
        onState(res.state)
      }
      cancelSub = sdk.subscribeAql({
        query: `
			FROM allEvents & '${LWW_TAG}' & '${LWW_TAG}-${LWW_VERSION}' & ('${LWW_CREATED_TAG}' | '${LWW_UPDATED_TAG}') & '${entityName}' & '${entityName}:${id}'
		  `,
        onResponse: (res: any) => {
          if (res.type === 'event' && !cancelled) {
            onState(res.payload as State<Data>)
          }
        },
        lowerBound: res.offsets,
        onError: (err: any) => {
          if (!cancelled) {
            onError(toError(err))
          }
        },
      })
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
  const tags = mkTags(entityName).update(id)
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

  const tags = mkTags(entityName).update(id)
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
  props: Partial<PickAqlTypes<Data>>,
): Promise<State<Data>[]> => _find(sdk, entityName, props).then((r) => r.states)
const findOne = async <Data>(
  sdk: SDK,
  entityName: EntityName,
  props: Partial<PickAqlTypes<Data>>,
): Promise<State<Data> | undefined> =>
  _find(sdk, entityName, props).then((r) => (r.states.length > 0 ? r.states[0] : undefined))

const _mqAqlFilter = (name: string, value: AqlFilterTypes): string => {
  switch (typeof value) {
    case 'string': {
      return `_.data['${name}']='${value}'`
    }
    case 'number': {
      return `_.data['${name}']=${value}`
    }
    case 'boolean': {
      return `${value ? '' : '!'}_.data['${name}']`
    }
    default: {
      throw new Error(
        `unexpected got value type '${typeof value}' which is not compatible with AQL queries`,
      )
    }
  }
}

const _find = async <Data>(
  sdk: SDK,
  entityName: EntityName,
  props: Partial<PickAqlTypes<Data>>,
): Promise<{ states: State<Data>[]; offsets: OffsetMap }> => {
  let query = `FROM allEvents & '${LWW_TAG}' & '${LWW_TAG}-${LWW_VERSION}' & '${entityName}'`
  if (Object.entries(props).length > 0) {
    query += ' FILTER '
    query += Object.entries(props)
      // Can't get the types to work here atm
      .map(([k, v]) => _mqAqlFilter(k, v as AqlFilterTypes))
      .join(' & ')
  }
  // Reduce latest
  const latest: Map<InstanceId, State<Data>> = new Map()
  const results = await sdk.queryAql(query)
  results
    .filter((r: AqlResponse): r is AqlEventMessage => r.type === 'event')
    .map((e: any) => e.payload as State<Data>)
    .forEach((s) => latest.set(s.meta.id, s))
  const offsets = results.find((r: AqlResponse): r is AqlOffsetsMsg => r.type === 'offsets')
  if (!offsets) {
    throw new Error(`internal error; queryAql did not return an 'offsets' event`)
  }

  return { states: [...latest.values()], offsets: offsets.offsets }
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
    create: (data) => createEntity(sdk, entityName, data),
    update: (id, data) => update(sdk, entityName, id, data),
    archive: (id) => setArchivedState(sdk, entityName, id, true),
    unarchive: (id) => setArchivedState(sdk, entityName, id, false),
    read: (id) => readById(sdk, entityName, id),
    readIds: () => readIds(sdk, entityName),
    readAll: () => readAll(sdk, entityName),
    find: (props) => find(sdk, entityName, props),
    findOne: (props) => findOne(sdk, entityName, props),
    subscribe: (id, onState, onError) => subscribeById(sdk, entityName, id, onState, onError),
    subscribeAll: (onStates, onError) => subscribeAll(sdk, entityName, onStates, onError),
    subscribeIds: (onId, onError) => subscribeIds(sdk, entityName, onId, onError),
    //subscribeFind: (props, onStates, onError) =>
    //  subscribeFind(sdk, entityName, props, onStates, onError),
  })
