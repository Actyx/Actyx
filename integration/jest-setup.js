// stub `window.alert`, because jsdom does not implement it
if (process.env.NODE_ENV === 'test') {
  window.alert = jest.fn()
}

// https://github.com/facebook/jest/issues/3251
if (!process.env.LISTENING_TO_UNHANDLED_REJECTION) {
  process.on('unhandledRejection', (reason) => {
    // fail fast
    throw reason
  })

  // Avoid memory leak by adding too many listeners
  process.env.LISTENING_TO_UNHANDLED_REJECTION = 'true'
}
