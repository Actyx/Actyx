/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
// import { IpfsStore } from 'ada/store/ipfsStore'
// import { Timestamp } from '..'
// import log from '../loggers'
// import { Pond, TimeInjector } from '../pond'
// import { FishName, SequenceNumber, Source } from '../types'
// import { Command, CommandType, Event, EventType, genericMachineFish } from './generic-machine'

// const ev: (reading: number, timestamp: Timestamp) => Event = (reading, timestamp) => {
//   return {
//     type: EventType.counterSet,
//     id: 'someId',
//     reading,
//     timestamp,
//   }
// }
// const cmd: (reading: number, timestamp: Timestamp) => Command = (reading, timestamp) => {
//   return { type: CommandType.inject, ev: ev(reading, timestamp) }
// }

// const _sleep = (ms: number) => {
//   return new Promise(resolve => setTimeout(resolve, ms))
// }

// async function delayFor(ms: number): Promise<void> {
//   await _sleep(ms)
// }

// let startingSeq = 0
// const timestampGenerator = (generationPeriod: number) => {
//   startingSeq += 1
//   return startingSeq * generationPeriod * 1000
// }

// const readingsGenerator = () => (Math.random() * 1000) >> 0

// // generationPeriod in 1/Hz so according to SI in seconds
// export const injectMultipleMachineEvents = (
//   store: IpfsStore,
//   initialTimestamp: number,
//   eventsCount: number,
//   generationPeriod: number,
// ) => {
//   const incrementalTimeInjector: TimeInjector = (
//     _source: Source,
//     _sequence: SequenceNumber,
//     events: ReadonlyArray<any>,
//   ) => {
//     return events[0].timestamp
//   }
//   const machineFish = genericMachineFish.type
//   const name = FishName.of('wut')
//   const pond = Pond.of(store, { timeInjector: incrementalTimeInjector })

//   for (let _i = 0; _i < eventsCount; _i++) {
//     pond
//       .feed(machineFish, name)(
//         cmd(
//           readingsGenerator(),
//           Timestamp.of(initialTimestamp + timestampGenerator(generationPeriod)),
//         ),
//       )
//       .subscribe({ error: err => log.pond.error(err) })
//   }

//   // we need to wait for the db activity to complete
//   const awaitPendingDBOperations = delayFor(300).then(() => {
//     log.pond.info('slept')
//     return 'done'
//   })

//   return awaitPendingDBOperations
// }
