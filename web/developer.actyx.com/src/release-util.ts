// There are copied over from cosmos-release since I can't get them to import
// for some reason. Something to do with release being built with CommonJS
// modules and that not being translated correctly by Docusaurus's Babel.
export const toShortHash = (hash: string): string => hash.substr(0, 7)

const RELEASE_TAG_PREFIX = 'build-'
const DRAFT_RELEASE_TAG_PREFIX = `draft-${RELEASE_TAG_PREFIX}`

const mkReleaseTag = (buildMajor: number, buildMinor: string): string =>
  `${RELEASE_TAG_PREFIX}${buildMajor}.${buildMinor}`
const mkDraftReleaseTag = (buildMajor: number, commitHash: string): string =>
  `${DRAFT_RELEASE_TAG_PREFIX}${buildMajor}.${toShortHash(commitHash)}`

const isNonDraftRelease = (tag: string) => tag.startsWith(RELEASE_TAG_PREFIX)
const isDraftRelease = (tag: string) => tag.startsWith(DRAFT_RELEASE_TAG_PREFIX)

/**
 * This function compares two tags. It returns < 0 if a is _smaller_ than a. It
 * returns > 0 if a is _bigger_ than b. It returns 0 if they are the same. This
 * function should only be called with release tags, not the numbers themselves.
 *
 * Note: if you use this with `Array.sort()`, this will return the array sorted
 *       in the default ascending order. If you would like to sort in a descending
 *       fashion, use `arr.sort().reverse()`.
 *
 * Please don't make this any _smarter_; purposefully simple so that it is
 * easy to understand
 */
const compare = (a: string, b: string): number => {
  if (typeof a !== 'string') {
    throw new Error(`Tags.compare function got non-string valud: a=${a} (typeof: ${typeof a})`)
  }
  if (typeof b !== 'string') {
    throw new Error(`Tags.compare function got non-string valud: b=${b} (typeof: ${typeof b})`)
  }
  if (!a.startsWith(DRAFT_RELEASE_TAG_PREFIX) && !a.startsWith(RELEASE_TAG_PREFIX)) {
    throw new Error(`Please only pass tags to Tags.compare; got: a=${a}`)
  }
  if (!b.startsWith(DRAFT_RELEASE_TAG_PREFIX) && !b.startsWith(RELEASE_TAG_PREFIX)) {
    throw new Error(`Please only pass tags to Tags.compare; got: b=${b}`)
  }

  if (a.startsWith(DRAFT_RELEASE_TAG_PREFIX) && b.startsWith(DRAFT_RELEASE_TAG_PREFIX)) {
    return 0
  }
  if (a.startsWith(DRAFT_RELEASE_TAG_PREFIX)) {
    return 1
  }
  if (b.startsWith(DRAFT_RELEASE_TAG_PREFIX)) {
    return -1
  }

  // This is now a non-draft release tag
  const majorA = parseInt(a.substr(RELEASE_TAG_PREFIX.length).split('.')[0])
  const minorA = parseInt(a.substr(RELEASE_TAG_PREFIX.length).split('.')[1])
  const majorB = parseInt(b.substr(RELEASE_TAG_PREFIX.length).split('.')[0])
  const minorB = parseInt(b.substr(RELEASE_TAG_PREFIX.length).split('.')[1])

  if (!Number.isInteger(majorA) || !Number.isInteger(minorA)) {
    throw new Error(`Release tag compare passed invalid release tag ${a}`)
  }
  if (!Number.isInteger(majorB) || !Number.isInteger(minorB)) {
    throw new Error(`Release tag compare passed invalid release tag ${b}`)
  }

  if (majorA < majorB) {
    return -1
  }
  if (majorA > majorB) {
    return 1
  }
  if (minorA < minorB) {
    return -1
  }
  if (minorA > minorB) {
    return 1
  }
  return 0
}

export const Tags = {
  compare,
  isNonDraftRelease,
  isDraftRelease,
  ReleaseTag: mkReleaseTag,
  DraftReleaseTag: mkDraftReleaseTag,
  RELEASE_TAG_PREFIX,
  DRAFT_RELEASE_TAG_PREFIX,
}

// Return a list of changes to things flattened across all commits and using
// the last amended notes for each one
export const flattenToActual = (notes: any): { [thing: string]: string[] } => {
  const out: { [thing: string]: string[] } = {}
  const addToThing = (thing: string, changes: string[]) => {
    if (Object.keys(out).includes(thing)) {
      out[thing] = out[thing].concat(changes)
    } else {
      out[thing] = changes
    }
  }

  Object.entries(notes).forEach(([_c, notesForChangeCommit]) => {
    const { original, amendments } = notesForChangeCommit as any
    const mostActual =
      Object.keys(amendments).length < 1
        ? original
        : (Object.values(amendments).sort(
            (a, b) => ((b as any).at as any).getTime() - ((a as any).at as any).getTime(),
          )[0] as any).notes
    if (mostActual) {
      Object.entries(mostActual).forEach(([thing, changes]) =>
        addToThing(thing, changes as string[]),
      )
    }
  })
  return out
}
