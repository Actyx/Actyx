/**
 * Avoids lint false positives like "Expression is always false. (strict-type-predicates)"
 */
export const lookup = <V>(m: { [k: string]: V }, k: string): V | undefined => m[k]
