/*
 * Actyx SDK: Functions for writing distributed apps
 * deployed on peer-to-peer networks, without any servers.
 *
 * Copyright (C) 2021 Actyx AG
 */
export const decorateEConnRefused = (errMsg: string, triedToContact: string) =>
  errMsg && typeof errMsg === 'string' && errMsg.includes('ECONNREFUSED')
    ? `Error: unable to connect to Actyx at ${triedToContact}. Is the service running? -- Error: ${errMsg}`
    : `Error in connection to Actyx (${triedToContact}): ${errMsg}`
