/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
import { List, Map } from 'immutable'

// After having stringified to JSON and parsed back, the immutable-Map will be a plain object.
export type SerializedMap<V> = Map<string, V> | { [key: string]: V }

export type SerializedList<V> = List<V> | ReadonlyArray<V>

export const SerializedMap = {
  // If it needs deserialization, it is probably not a Map anymore.  However, we leave it up to the
  // backend whether it chooses to give us back a value that went through a JSON-serialization cycle
  // or not.
  deserialize: <V>(m: SerializedMap<V>): Map<string, V> => (Map.isMap(m) ? m : Map(m)),
}

export const SerializedList = {
  deserialize: <V>(l: SerializedList<V>): List<V> => (List.isList(l) ? l : List(l)),
}
