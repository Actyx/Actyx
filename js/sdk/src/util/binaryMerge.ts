/*
 * Actyx SDK: Functions for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 *
 * Copyright (C) 2021 Actyx AG
 */
/* eslint-disable @typescript-eslint/no-empty-function */
export type index = number
export type cr = number

export abstract class BinaryMerge {
  protected abstract compare(ai: index, bi: index): cr
  protected collision(_ai: index, _bi: index): void {}
  protected fromA(_a0: index, _a1: index, _bi: index): void {}
  protected fromB(_ai: index, _b0: index, _b1: index): void {}
  protected merge0(a0: index, a1: index, b0: index, b1: index): void {
    if (a0 === a1) {
      if (b0 !== b1) {
        this.fromB(a0, b0, b1)
      }
    } else if (b0 === b1) {
      this.fromA(a0, a1, b0)
    } else {
      const am = (a0 + a1) >>> 1
      const res = this.binarySearchB(am, b0, b1)
      if (res >= 0) {
        // same elements
        const bm = res
        // merge everything below a(am) with everything below the found element
        this.merge0(a0, am, b0, bm)
        // add the elements a(am) and b(bm)
        this.collision(am, bm)
        // merge everything above a(am) with everything above the found element
        this.merge0(am + 1, a1, bm + 1, b1)
      } else {
        const bm = -res - 1
        // merge everything below a(am) with everything below the found insertion point
        this.merge0(a0, am, b0, bm)
        // add a(am)
        this.fromA(am, am + 1, bm)
        // everything above a(am) with everything above the found insertion point
        this.merge0(am + 1, a1, bm, b1)
      }
    }
  }
  private binarySearchB(ai: index, b0: index, b1: index): index {
    let m = b0
    let n = b1 - 1
    while (m <= n) {
      const k = (n + m) >>> 1
      const cmp = this.compare(ai, k)
      if (cmp > 0) {
        m = k + 1
      } else if (cmp < 0) {
        n = k - 1
      } else {
        return k
      }
    }
    return -m - 1
  }
}
