export const waitForMs = (ms: number): Promise<void> =>
  new Promise((res) => setTimeout(() => res(), ms))
