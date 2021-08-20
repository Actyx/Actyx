import { NodeId } from './types'

export type RecordId<T> = {
  uniqueId: string

  _pantomData: T
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
  recordId: RecordId<unknown>,
  currentState: unknown,
  Petitioner: Node,
) => boolean

export type TakeOwnershipResult<T> =
  | {
      type: 'success'
      currentState: T
    }
  | {
      type: 'rejected-by-owner'
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
   * @param pin          Whether to immediately pin the record locally. Defaults to false.
   *
   */
  create<T>(initialState: T, pin?: boolean): Promise<RecordId<T>>

  /**
   * Mutate a record.
   * This involves contacting the current owner of the record, to make sure that we are applying the update to its current canonical state.
   *
   * @param recordId  Id of the record to be mutated.
   * @param doMutate  Function for updating the record. Note that this function will be called *multiple times* if mutation has to be retried due to conflicts.
   *
   * @returns A Promise that resolves to the updated state, once mutation has been completed.
   *          The Promise will NEVER resolve while the current owner of the record is unreachable.
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
   **/
  replace<T>(recordId: RecordId<T>, mkNewState: (state: T) => T): Promise<UpdatedState<T>>

  takeOwnership<T>(recordId: RecordId<T>): Promise<TakeOwnershipResult<T>>

  /** Set the logic used to decide wether to release owned records to other nodes. Defaults to: `(pinned) => !pinned`  */
  setReleaseDecisionLogic(decisionLogic: DecideRelease): boolean
}
