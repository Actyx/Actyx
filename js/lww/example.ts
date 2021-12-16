import { Lww } from './src'
import { SDK } from '@actyx/sdk'

type Person = {
  firstName: string
  lastName: string
  friendIds: string[]
}

const Model = Lww<Person>('person.6')

const run = async () => {
  const sdk = await SDK.of({
    appId: 'com.example.lww',
    displayName: 'LWW example',
    version: '0.1.0',
  })

  const model = Model(sdk)

  // TODO add findByProps using AQL filters ;)

  const persons = await model.readAll()
  const existingJohn = persons.find(
    ({ data: { firstName, lastName } }) => firstName === 'John' && lastName === 'Doe',
  )
  let johnId: string = existingJohn?.meta.id || ''
  if (!existingJohn) {
    console.log(`did not find John; creating him`)
    johnId = await model.create({
      firstName: 'John',
      lastName: 'Doe',
      friendIds: [],
    })
  } else {
    console.log(`John already created`)
  }

  const existingJane = persons.find(
    ({ data: { firstName, lastName } }) => firstName === 'Jane' && lastName === 'Doe',
  )
  let janeId: string = existingJane?.meta.id || ''
  if (!existingJane) {
    console.log(`did not find Jane; creating her`)
    janeId = await model.create({
      firstName: 'Jane',
      lastName: 'Doe',
      friendIds: [],
    })
  } else {
    console.log(`Jane already created`)
  }

  if (existingJohn && !existingJohn.data.friendIds.includes(janeId)) {
    console.log(`adding Jane to John's friend list`)
    model.update(johnId, {
      ...existingJohn.data,
      friendIds: [...existingJohn.data.friendIds, janeId],
    })
  } else {
    console.log(`Jane is already in John's friend list`)
  }

  if (existingJane && !existingJane.data.friendIds.includes(johnId)) {
    console.log(`adding John to Jane's friend list`)
    model.update(janeId, {
      ...existingJane.data,
      friendIds: [...existingJane.data.friendIds, johnId],
    })
  } else {
    console.log(`John is already in Jane's friend list`)
  }

  ;(await model.readAll()).forEach((person) => {
    console.log(
      `${person.meta.id.substring(0, 6)}: ${person.data.firstName} ${
        person.data.lastName
      } (friends: ${person.data.friendIds.map((s) => s.substring(0, 6)).join(',')} )`,
    )
  })
}

run()
