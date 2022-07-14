import { FishId, Pond, Tags } from './lib/index.js'

const run = async () => {
  // const pond = await Pond.of({ appId: 'com.example.x', displayName: 'test', version: '1' }, {actyxHost:'what.ever'}, {})
  const pond = await Pond.default({ appId: 'com.example.x', displayName: 'test', version: '1' })
  console.log('got pond', pond)
  pond.observe(
    {
      fishId: FishId.of('test', 'test', 1),
      where: Tags(),
      initialState: 0,
      onEvent: (state, _event, meta) => {
        if (state % 1 == 0) console.log('onEvent', state, meta.timestampAsDate(), meta.tags)
        return state + 1
      },
      isReset: (_e, meta) => {
        const age = Math.floor(Date.now() - meta.timestampMicros / 1000)
        const reset = age > 1_000_000_000
        console.log('isReset', meta.timestampAsDate(), age, reset)
        return reset
      },
    },
    (state) => console.log('state', state),
  )
}
run()
