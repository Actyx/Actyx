import { allEvents, FishId, Metadata, Pond, Tag } from '@actyx/pond'

const HelloWorldFish = {
  fishId: FishId.of('com.example.quickstart', 'quickstart', 0),
  initialState: 'Hello, World!',
  onEvent: (_oldState: any, event: any, _metadata: Metadata) => event,
  where: allEvents,
}

const main = async () => {
  const pond = await Pond.default()
  pond.observe(HelloWorldFish, state => console.log(state))

  var counter = 1
  setInterval(() => {
    pond.emit(Tag('Some tag'), `Hello ${counter}!`)
    counter += 1
  }, 3500)
}

main()
