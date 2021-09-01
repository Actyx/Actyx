/*
 * Actyx SDK: Functions for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2021 Actyx AG
 */
import { Observable } from '../../node_modules/rxjs'
import { Metadata, PendingEmission } from './various'

/** Create a PendingEmission object from an Observable. @internal */
export const pendingEmission = (o: Observable<Metadata[]>): PendingEmission => ({
  subscribe: o.subscribe.bind(o),
  toPromise: () => o.toPromise(),
})

/**
 * Refinement that checks whether typeof x === 'string'
 * @public
 */
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export const isString = (x: any): x is string => typeof x === 'string'

/**
 * Refinement that checks whether typeof x === 'number'
 * @public
 */
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export const isNumber = (x: any): x is number => typeof x === 'number'

/**
 * Refinement that checks whether typeof x === 'number'
 * @public
 */
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export const isBoolean = (x: any): x is boolean => typeof x === 'boolean'
