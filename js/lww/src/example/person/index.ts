import { Lww } from '../..'
import { Person } from './types'
import { SDK } from '@actyx/sdk'

const Model = Lww<Person>('person.9')

const run = async () => {
  const sdk = await SDK.of({
    appId: 'com.example.lww',
    displayName: 'LWW example',
    version: '0.1.0',
  })

  const model = Model(sdk)

  // # Create instances
  // Create persons after checking if that person has already created. Note that
  // network partitions may lead to multiple instances being created. They are not
  // 100% equal since they have a unique ID, but this case should be handeled in an
  // admin view or similar where a human being resolves the conflict if there are
  // two different instances representing the same real-world instance.
  if (!(await model.findOne({ firstName: 'John', lastName: 'Doe' }))) {
    console.log(`Creating John Doe (married: true)`)
    await model.create(Person.of('Doe', 'John', true))
  }

  if (!(await model.findOne({ firstName: 'Jane', lastName: 'Doe' }))) {
    console.log(`Creating Jane Doe (married: false)`)
    await model.create(Person.of('Doe', 'Jane', false))
  }

  setInterval(async () => {
    const john = await model.findOne({ firstName: 'John' })
    if (john) {
      await model.update(john.meta.id, { ...john.data, married: !john.data.married })
    } else {
      throw new Error(`unexpectedly did not find John`)
    }
  }, 5000)

  //// # Subscribe to instances
  //// This will subscribe to all instances. The provided callback will be called
  //// whenever any of the instances is updated or a new instance is created. The
  //// callback is given all current instance states as an argument.
  model.subscribeAll((states) => {
    const numMarriedPeople = states.reduce((cnt, state) => cnt + (state.data.married ? 1 : 0), 0)
    const numUnmarriedPeople = states.reduce((cnt, state) => cnt + (state.data.married ? 0 : 1), 0)
    console.log(`there are ${numMarriedPeople} married and ${numUnmarriedPeople} unmarried people`)
  }, console.error)
}

run()
