import { Actyx as SDK, AppManifest } from '@actyx/sdk'
import { o } from 'ramda'
import { Lww, State } from '..'
import { mkUniqueId } from '../uuid'

const TEST_MANIFEST: AppManifest = {
  appId: 'com.example.lww.test',
  displayName: 'Testing LWW',
  version: '0.0.1',
}

type TestType = {
  n: number
  s: string
  b: boolean
}

const mkModel = (name: string) => Lww<TestType>(`${name}-${mkUniqueId()}-${Date.now().toString()}`)
const defaultModel = mkModel('lww-test')
// set in beforeAll
let sdk: SDK = undefined as any as SDK

beforeAll(async () => {
  sdk = await SDK.of(TEST_MANIFEST)
})

afterAll(() => {
  // If we don't dispose, the SDK's connection to Actyx will remain open and Jest will
  // warn that 'A worker process has failed to exit gracefully and has been force exited.
  // This is likely caused by tests leaking due to improper teardown. Try running with
  // --detectOpenHandles to find leaks. Active timers can also cause this, ensure that
  // .unref() was called on them.'
  if (sdk) {
    sdk.dispose()
  }
})

describe(`running with Acytx`, () => {
  it(`can connect to Actyx`, async () => {
    expect(await sdk.nodeId).toBeTruthy()
  })

  it(`can create instances`, async () => {
    await defaultModel(sdk).create({ s: 'a', b: true, n: 1 })
    await defaultModel(sdk).create({ s: 'b', b: false, n: 2 })
    const states = (await defaultModel(sdk).readAll()).map((s) => s.data)
    expect(states).toContainEqual({ s: 'a', b: true, n: 1 })
    expect(states).toContainEqual({ s: 'b', b: false, n: 2 })
  })

  it(`can find instances`, async () => {
    await defaultModel(sdk).create({ s: 'aaa', b: true, n: 3 })
    await defaultModel(sdk).create({ s: 'bbb', b: true, n: 6 })
    await defaultModel(sdk).create({ s: 'bbb', b: false, n: 13 })
    expect(await defaultModel(sdk).findOne({ s: 'aaa' })).toBeTruthy()
    expect(await defaultModel(sdk).findOne({ n: 3 })).toBeTruthy()
    expect(await defaultModel(sdk).findOne({ s: 'bbb' })).toBeTruthy()
    expect(await defaultModel(sdk).findOne({ n: 6 })).toBeTruthy()
    expect(await defaultModel(sdk).findOne({ s: 'ccc' })).toBeUndefined()
    expect(await defaultModel(sdk).findOne({ n: 9 })).toBeUndefined()
    expect(await defaultModel(sdk).findOne({ b: true })).toBeTruthy()
    expect(await defaultModel(sdk).find({ s: 'bbb' })).toHaveLength(2)
    expect((await defaultModel(sdk).find({ b: true })).length).toBeGreaterThanOrEqual(2)
  })

  it(`can update`, async () => {
    const id = await defaultModel(sdk).create({ s: 'a', b: true, n: 0 })
    const s0 = await defaultModel(sdk).read(id)
    expect(s0).toBeTruthy()
    expect(s0?.data.s).toBe('a')
    await defaultModel(sdk).update(id, { s: 'b', b: false, n: -1 })
    const s1 = await defaultModel(sdk).read(id)
    expect(s1).toBeTruthy()
    expect(s1?.data.s).toBe('b')
    expect(s1?.data.b).toBe(false)
    expect(s1?.data.n).toBe(-1)
  })

  it(`can subscribe by id`, async () => {
    const INITIAL_STATE: TestType = { s: 'string', b: true, n: 0 }
    const instanceId = await defaultModel(sdk).create(INITIAL_STATE)
    const subscribe = new Promise<State<TestType>[]>((res, rej) => {
      const states: State<TestType>[] = []
      const cancel = defaultModel(sdk).subscribe(
        instanceId,
        (state) => {
          states.push(state)
          if (states.length > 3) {
            cancel()
            res(states)
          }
        },
        rej,
      )
    })

    const updates = async () => {
      await defaultModel(sdk).update(instanceId, { ...INITIAL_STATE, n: 1 })
      await defaultModel(sdk).update(instanceId, { ...INITIAL_STATE, n: 2 })
      await defaultModel(sdk).update(instanceId, { ...INITIAL_STATE, n: 3 })
    }

    const [states] = await Promise.all([subscribe, updates()])
    expect(states).toHaveLength(4)
    expect(states[0].data).toStrictEqual({ ...INITIAL_STATE, n: 0 })
    expect(states[1].data).toStrictEqual({ ...INITIAL_STATE, n: 1 })
    expect(states[2].data).toStrictEqual({ ...INITIAL_STATE, n: 2 })
    expect(states[3].data).toStrictEqual({ ...INITIAL_STATE, n: 3 })
  })

  // This test makes some assumptions about ordering that probably don't hold
  it(`can subscribe to all`, async () => {
    const model = mkModel('sub1')
    const BASE_STATE: TestType = { s: '', b: true, n: 0 }
    const history: State<TestType>[][] = []
    const cancel = model(sdk).subscribeAll((states) => {
      history.push(states)
    }, console.error)

    const id1 = await model(sdk).create({ ...BASE_STATE, s: 'in1' })
    await new Promise((res) => setTimeout(res, 500))
    await model(sdk).update(id1, { ...BASE_STATE, s: 'in1', n: 88 })
    await new Promise((res) => setTimeout(res, 500))
    const id2 = await model(sdk).create({ ...BASE_STATE, s: 'in2' })
    await new Promise((res) => setTimeout(res, 500))
    await model(sdk).update(id2, { ...BASE_STATE, s: 'in2', n: 66 })
    await new Promise((res) => setTimeout(res, 500))
    cancel()
    const states = history.map((states) => states.map((s) => s.data))

    expect(states).toHaveLength(4)
    expect(states[0]).toStrictEqual([
      {
        ...BASE_STATE,
        s: 'in1',
      },
    ])
    expect(states[1]).toStrictEqual([
      {
        ...BASE_STATE,
        s: 'in1',
        n: 88,
      },
    ])
    expect(states[2]).toStrictEqual([
      {
        ...BASE_STATE,
        s: 'in1',
        n: 88,
      },
      {
        ...BASE_STATE,
        s: 'in2',
      },
    ])
    expect(states[3]).toStrictEqual([
      {
        ...BASE_STATE,
        s: 'in1',
        n: 88,
      },
      {
        ...BASE_STATE,
        s: 'in2',
        n: 66,
      },
    ])
  })

  // If you set the timeouts here to ~200, the test fails because the initial
  // query to get the current IDs (`_readIds`) returns 0 events. WTF
  it(`doesnt return intermittent results in subscribeAll`, async () => {
    const model = mkModel('sub2')(sdk)

    const entity1id = await model.create({ b: true, n: 1, s: 'a' })
    const entity2id = await model.create({ b: true, n: 1, s: 'a' })
    expect(entity1id).toBeTruthy()
    expect(entity2id).toBeTruthy()

    const subscribeResults: State<TestType>[][] = []
    const cancel = model.subscribeAll((states) => subscribeResults.push(states), console.error)

    await new Promise((res) => setTimeout(res, 500))

    expect(subscribeResults).toHaveLength(1)
    //expect(subscribeResults[0][0].meta.id).toBe(entity1id)
    //expect(subscribeResults[0][1].meta.id).toBe(entity2id)
    expect(subscribeResults[0].map((r) => r.meta.id)).toContain(entity1id)
    expect(subscribeResults[0].map((r) => r.meta.id)).toContain(entity2id)

    const entity3id = await model.create({ b: true, n: 1, s: 'a' })
    expect(entity3id).toBeTruthy()
    await new Promise((res) => setTimeout(res, 500))
    expect(subscribeResults).toHaveLength(2)
    expect(subscribeResults[1].map((r) => r.meta.id)).toContain(entity1id)
    expect(subscribeResults[1].map((r) => r.meta.id)).toContain(entity2id)
    expect(subscribeResults[1].map((r) => r.meta.id)).toContain(entity3id)

    cancel()

    expect(1).toBeTruthy()
  })

  it(`doesn't fail if query can't find anything`, async () => {
    const model = mkModel('test')(sdk)
    const results = await model.readAll()
    expect(results).toHaveLength(0)
  })

  it(`doesn't fail if query can't find anything`, async () => {
    const model = mkModel('test')(sdk)
    const results = await model.readIds()
    expect(results).toHaveLength(0)
  })

  it(`doesn't fail if subscribe can't find anything`, async () => {
    const model = mkModel('test')(sdk)
    let wasCalled = false
    const cancel = model.subscribeAll(() => {
      wasCalled = true
    }, console.error)
    await new Promise((res) => setTimeout(res, 500))
    cancel()
    expect(wasCalled).toBe(false)
  })
  it(`works with custom IDs`, async () => {
    const model = mkModel('test')(sdk)
    const id1 = await model.create({ b: true, n: 0, s: '' }, { id: 'id1' })
    expect(id1).toBe('id1')
    const id2 = await model.create({ b: true, n: 0, s: '' }, { id: 'id2' })
    expect(id2).toBe('id2')
    await new Promise((res) => setTimeout(res, 500))

    const entities = await model.readAll()
    expect(entities).toHaveLength(2)
    expect(entities[0].meta.id).toBe('id1')
    expect(entities[1].meta.id).toBe('id2')

    await model.update('id1', { b: false, n: 0, s: '' })
    await new Promise((res) => setTimeout(res, 500))
    const entity = await model.read('id1')
    expect(entity).not.toBeUndefined()
    expect(entity?.meta.id).toBe('id1')
  })
  it(`works with custom tags`, async () => {
    const model = mkModel('test')(sdk)
    await model.create({ b: true, n: 0, s: '0' }, { tags: ['0'] })
    await model.create({ b: true, n: 0, s: '1' }, { tags: ['0', '1'] })
    await model.create({ b: true, n: 0, s: '2.1' }, { tags: ['0', '2', '2.1'] })
    const id = await model.create({ b: true, n: 0, s: '2.2' }, { tags: ['0', '2', '2.2'] })
    await new Promise((res) => setTimeout(res, 500))

    expect(await model.readAll()).toHaveLength(4)
    expect(await model.readAll({ tags: ['0'] })).toHaveLength(4)
    expect(await model.readAll({ tags: ['1'] })).toHaveLength(1)
    expect(await model.readAll({ tags: ['2'] })).toHaveLength(2)
    expect(await model.readAll({ tags: ['2.1'] })).toHaveLength(1)
    expect(await model.readAll({ tags: ['2.2'] })).toHaveLength(1)

    await model.update(id, { b: true, n: 0, s: 'xxx' })
    const res = await model.readAll({ tags: ['2.2'] })
    expect(res).toHaveLength(1)
    expect(res[0]).toEqual(
      expect.objectContaining({
        data: {
          b: true,
          n: 0,
          s: 'xxx',
        },
      }),
    )
    await new Promise((res) => setTimeout(res, 500))

    const onStates = jest.fn()
    model.subscribeAll(onStates, () => {}, {
      tags: ['2'],
    })

    await new Promise((res) => setTimeout(res, 500))

    expect(onStates).toHaveBeenCalledTimes(1)
    expect(onStates).toHaveBeenNthCalledWith(
      1,
      expect.arrayContaining([
        expect.objectContaining({
          data: {
            b: true,
            n: 0,
            s: 'xxx',
          },
        }),
        expect.objectContaining({
          data: {
            b: true,
            n: 0,
            s: '2.1',
          },
        }),
      ]),
    )
    await model.update(id, { b: true, n: 0, s: 'yyy' })
    await new Promise((res) => setTimeout(res, 500))
    expect(onStates).toHaveBeenCalledTimes(2)
    expect(onStates).toHaveBeenNthCalledWith(
      2,
      expect.arrayContaining([
        expect.objectContaining({
          data: expect.objectContaining({
            s: 'yyy',
          }),
          meta: expect.objectContaining({
            id,
          }),
        }),
      ]),
    )
  })
})
