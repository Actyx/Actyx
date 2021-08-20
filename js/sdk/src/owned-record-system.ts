import { CancelSubscription, NodeId } from './types'

/** Class of some records. All records of a given class should have the same type. */
export type RecordClass<T, A> = {
  className: string

  /** The value type of records of this class. */
  _phantomDataMut: T

  /** The static (readonly) part of records of this class. */
  _phantomDataReadonly: A
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type RecordId<T, A = any> = {
  /** Globally unique id for this record. (`eventId` of the event that created it) */
  uniqueId: string

  /** Class of the record. */
  recordClass: RecordClass<T, A>

  /** Arbitrary readonly-data associated with the record. Can be any JSON-serializable value. */
  readonlyData: A
}

export type Node = {
  // Unique node id given by Actyx system
  nodeId: NodeId

  // How the node calls itself. Not guaranteed to be unique across the swarm.
  nodeName: string
}

/** Whether to release a currently owned record to another node. */
export type DecideRelease = (
  pinned: boolean,
  recordId: RecordId<unknown, unknown>,
  currentState: unknown,
  Petitioner: Node,
) => boolean

export type TakeOwnershipResult<T> = {
  type: 'success' | 'rejected-by-owner' | 'already-ours'
  // Latest known state of the record. May already be outdated, even if we succeeded in claiming the record.
  state: T
}

/** A successful state update. */
export type UpdatedState<T> = {
  oldState: T
  newState: T
}

/** Functions for managing records with single-device ownership,
 * i.e. 1 device owns the record and serializes all access. @public */
export interface OwnedRecordSystem {
  // Set the name of the local node, for inspection by machines or humans.
  // Returns true if node name was replaced, false if it already had the right value.
  setNodeName(nodeName: string): Promise<boolean>

  /**
   * Create a new uniquely identified record.
   *
   * @param initialState The initial state of the record.
   * @param readonlyData Static data associated with the record.
   * @param pin          Whether to immediately pin the record locally. Defaults to false.
   *                     Pinning does nothing besides passing `pinned=true` to the local `DecideRelease` logic.
   *
   */
  create<T, A>(initialState: T, readonlyData: A, pin?: boolean): Promise<RecordId<T, A>>

  /**
   * List all locally known instances of a given record class.
   *
   * @param recordClass  Class of records to find.
   * @param parameters   Optionally, partial readonly-data of this class, to restrict the set of instances that will be found.
   *                     (Every returned `RecordId` will agree with `parameters` as far as values in `parameters` have been defined.
   *
   * @returns   A Promise that resolves to the locally known set of matching record ids.
   *
   */
  list<T, A>(recordClass: RecordClass<T, A>): Promise<RecordId<T, A>[]>
  list<T, A>(recordClass: RecordClass<T, A>, parameters: Partial<A>): Promise<RecordId<T, A>[]>

  /**
   * List all records we know to be owned be the specified node.
   *
   * @param nodeId  Node whose records to list. Defaults to our own.
   *
   **/
  listOwned(nodeId?: NodeId): Promise<RecordId<unknown>[]>

  /**
   * Observe the current state of any record.
   */
  observe<T>(recordId: RecordId<T>, cb: (state: T) => void): CancelSubscription

  /**
   * Mutate a record.
   * This involves contacting the current owner of the record, to make sure that we are applying the update to its current canonical state.
   *
   * @param recordId  Id of the record to be mutated.
   * @param doMutate  Function for updating the record. Note that this function will be called *multiple times* if mutation has to be retried due to conflicts.
   *
   * @returns A Promise that resolves to the updated state, once mutation has been completed.
   *          The Promise will NEVER resolve while the current owner of the record is unreachable.
   *          The Promise will reject if `recordId.readonlyData` is defined but does not match the actual readonly-data of the record.
   */
  mutate<T>(recordId: RecordId<T>, doMutate: (state: T) => void): Promise<UpdatedState<T>>

  /**
   * Replace the current value of a record with another one.
   * This involves contacting the current owner of the record, to make sure that we are computing our replacement value based on the current canonical state.
   *
   * @param recordId   Id of the record to be mutated.
   * @param mkNewState Function to compute a new state from the state. Note that this function will be called *multiple times* if replacement has to be retried due to conflicts.
   *
   * @returns A Promise that resolves to the updated state, once mutation has been completed.
   *          The Promise will NEVER resolve while the current owner of the record is unreachable.
   *          The Promise will reject if `recordId.readonlyData` is defined but does not match the actual readonly-data of the record.
   **/
  replace<T>(recordId: RecordId<T>, mkNewState: (state: T) => T): Promise<UpdatedState<T>>

  /**
   * Take ownership of a record.
   *
   * @param recordId  The record to take ownership of.
   * @param pin       Whether to immediately pin the record. Pinning does nothing besides passing `pinned=true` to the local `DecideRelease` logic.
   *
   * @returns A Promise that reports whether ownership was transfered, or the current owner’s `DecideRelease` logic rejected the transfer.
   *          The Promise will NEVER resolve as long as the current owner of the record is not reachable.
   */
  takeOwnership<T>(recordId: RecordId<T>, pin: boolean): Promise<TakeOwnershipResult<T>>

  /**
   * Creates an *owned copy* of an existing record.
   * This can be used to progress when facing an unresponsive record owner.
   *
   * @param recordId  The source record to make a copy of.
   * @param pin       Whether to immediately pin the record. Pinning does nothing besides passing `pinned=true` to the local `DecideRelease` logic.
   *
   * @returns The id of the **new** record created from the old record id.
   *          The record has **different** `uniqueId`, identical `readonlyData`, and a state equal to the source record’s latest state known to this node.
   */
  copy<T>(recordId: RecordId<T>, pin: boolean): Promise<RecordId<T>>

  /**
   * Push ownership of a record to another node.
   *
   * @param target    Node id of the node that should gain ownership. If the target node is unknown, an exception is thrown.
   * @param recordId  The record to transfer.
   */
  pushOwnership(target: NodeId, recordId: RecordId<unknown>): void

  /** Set the logic used to decide wether to release owned records to other nodes. Defaults to: `(pinned) => !pinned`  */
  setReleaseDecisionLogic(decisionLogic: DecideRelease): boolean
}
