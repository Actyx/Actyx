/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
/**
 * Basic ipfs types that are used both in pubsub messages and in long-term storage
 * as well as in the implementation.
 */
import { FishName, Psn, Semantics, Timestamp } from '..'

// we don't need to store the source id, since we are storing events per
// source id.
export type IpfsEnvelope = Readonly<{
  semantics: Semantics
  name: FishName
  timestamp: Timestamp
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  payload: any
}>
export type StoredIpfsEnvelope = IpfsEnvelope &
  Readonly<{
    psn: Psn
  }>
