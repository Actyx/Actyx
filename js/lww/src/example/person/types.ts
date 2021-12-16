export type Person = {
  lastName: string
  firstName: string
  friendIds: string[]
  married: boolean
}

export const Person = {
  of: (lastName: string, firstName: string, married?: boolean): Person => ({
    lastName,
    firstName,
    married: married || false,
    friendIds: [],
  }),
  compare: (a: Person, b: Person) => {
    const strA = `${a.lastName}-${a.firstName}`
    const strB = `${b.lastName}-${b.firstName}`
    return strA.localeCompare(strB)
  },
  equals: (a: Person, b: Person) => Person.compare(a, b) === 0,
  addFriend: (person: Person, friendId: string): Person => ({
    ...person,
    friendIds: person.friendIds.filter((id) => id !== friendId).concat([friendId]),
  }),
  removeFriend: (person: Person, friendId: string): Person => ({
    ...person,
    friendIds: person.friendIds.filter((id) => id !== friendId),
  }),
  hasFriend: (person: Person, friendId: string): boolean => person.friendIds.includes(friendId),
  findByName:
    (lastName: string, firstName: string) =>
    (persons: Person[]): Person | undefined =>
      persons.find((p) => Person.equals(p, Person.of(lastName, firstName))),
}
