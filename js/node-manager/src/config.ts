type Config = {
  windowSize: {
    initial: {
      width: number
      height: number
    }
    minimum: {
      width: number
      height: number
    }
  }
}

export const CONFIG: Config = {
  windowSize: {
    initial: {
      width: 1000,
      height: 600,
    },
    minimum: {
      width: 850,
      height: 450,
    },
  },
}
