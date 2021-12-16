import { SDK, AppManifest } from '@actyx/sdk'
import { Lww, State } from '..'

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

const mkModel = (name: string) => Lww<TestType>(`${name}-${Date.now().toString()}`)
const defaultModel = mkModel('lww-test')
// set in beforeAll
let sdk: SDK = {} as SDK

beforeAll(async () => {
  sdk = await SDK.of(TEST_MANIFEST)
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
    expect(async () => await defaultModel(sdk).create({ s: 'aaa', b: true, n: 3 })).not.toThrow()
    expect(async () => await defaultModel(sdk).create({ s: 'bbb', b: true, n: 6 })).not.toThrow()
    expect(async () => await defaultModel(sdk).create({ s: 'bbb', b: false, n: 12 })).not.toThrow()
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

  it(`can subscribe to all`, async () => {
    const model = mkModel('sub')
    const BASE_STATE: TestType = { s: '', b: true, n: 0 }
    const subscribe = new Promise<State<TestType>[][]>((res, rej) => {
      const history: State<TestType>[][] = []
      const cancel = model(sdk).subscribeAll((states) => {
        history.push(states)
        if (history.length > 3) {
          cancel()
          res(history)
        }
      }, rej)
    })

    const tasks = async () => {
      const id1 = await model(sdk).create({ ...BASE_STATE, s: 'in1' })
      await model(sdk).update(id1, { ...BASE_STATE, s: 'in1', n: 88 })
      const id2 = await model(sdk).create({ ...BASE_STATE, s: 'in2' })
      await model(sdk).update(id2, { ...BASE_STATE, s: 'in2', n: 66 })
    }

    const [history] = await Promise.all([subscribe, tasks()])
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
})
