const start = async () => {
  const wasm = await import("ax-wasm")
  await wasm.default()
  require("./main.tsx")
}
start()
