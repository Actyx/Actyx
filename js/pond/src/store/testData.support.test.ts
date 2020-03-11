/*
 * Actyx Pond: A TypeScript framework for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 * 
 * Copyright (C) 2020 Actyx AG
 */
const keepalive = { type: 'keepalive', time: '1970-01-01T00:00:00Z' }
const publishSuccess = {
  type: 'publishResponse',
  id: 'id',
  status: { type: 'success', lastPsn: 1 },
}
const publishIgnored = {
  type: 'publishResponse',
  id: 'id',
  status: {
    type: 'ignored',
    lastPsn: 4243,
    window: 5,
  },
}
const eventsFromServer = {
  type: 'events',
  events: {
    kshgfkh: [
      // source id
      {
        subscriptions: [['article'], ['article', 'E12345678']],
        semantics: 'article',
        name: 'E12345678',
        sequence: 1234,
        psn: 12345,
        payload: { some: 'data', we: 'dont', care: 'about' },
      },
      {
        subscriptions: [['article']],
        semantics: 'article',
        name: 'E12345679',
        sequence: 1,
        psn: 12567, // when receiving events, PSNs are not necessarily consecutive because of filtering
        payload: { some: 'data', we: 'dont', care: 'about' },
      },
    ],
  },
}
const changedEventsFromServer = {
  type: 'events',
  events: {
    kshgfkh: [
      // source id
      {
        subscriptions: [['article'], ['article', 'E12345678']],
        semantics: 'article',
        name: 'E12345678',
        sequence: 1234,
        psn: 12345,
        payload: 'and now to something completely different...',
      },
      {
        subscriptions: [['article']],
        semantics: 'article',
        name: 'E12345679',
        sequence: 1,
        psn: 12567, // when receiving events, PSNs are not necessarily consecutive because of filtering
        payload: 'and now to something completely different...',
      },
    ],
  },
}
const oldEventsFromServer = {
  type: 'events',
  events: {
    kshgfkh: [
      // source id
      {
        subscriptions: [['article'], ['article', 'E12345678']],
        semantics: 'article',
        name: 'E12345678',
        sequence: 123,
        psn: 1,
        payload: { some: 'data', we: 'dont', care: 'about' },
      },
      {
        subscriptions: [['article']],
        semantics: 'article',
        name: 'E12345679',
        sequence: 2,
        psn: 2,
        payload: { some: 'data', we: 'dont', care: 'about' },
      },
    ],
  },
}
const expectedOffsets = [
  { id: ['kshgfkh', ['article', 'E12345678']], psn: 12345 },
  { id: ['kshgfkh', ['article']], psn: 12567 },
]
const expectedAdd = [
  { offsets: { kshgfkh: 12345 }, subscription: ['article', 'E12345678'] },
  { offsets: { kshgfkh: 12567 }, subscription: ['article'] },
]
const expectedStoredEvents = [
  {
    payload: { some: 'data', we: 'dont', care: 'about' },
    psn: 12345,
    timestamp: undefined,
    id: ['article', 'E12345678', 'kshgfkh', 1234],
  },
  {
    payload: { some: 'data', we: 'dont', care: 'about' },
    psn: 12567,
    timestamp: undefined,
    id: ['article', 'E12345679', 'kshgfkh', 1],
  },
]
export const testData = {
  keepalive,
  publishSuccess,
  publishIgnored,
  eventsFromServer,
  expectedOffsets,
  expectedAdd,
  expectedStoredEvents,
  oldEventsFromServer,
  changedEventsFromServer,
}
