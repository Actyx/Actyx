import { Actyx } from "@actyx/sdk";
import * as uuid from "uuid";
import { supplyWater } from "./consumers/water-pump";
import { dockAndDrawWater } from "./consumers/watering-robot";

async function main() {

  const APP_MANIFEST = {
    appId: "com.example.tomato-robot",
    displayName: "Tomato Robot",
    version: "1.0.0",
  }

  const sdk1 = await Actyx.of(APP_MANIFEST);
  const sdk2 = await Actyx.of(APP_MANIFEST);
  const dockingId = uuid.v4();

  // promises
  const simulatedPumpPart = supplyWater(sdk1, dockingId)
  const simulatedRobotPart = dockAndDrawWater(sdk2, dockingId)

  // wait until both processes ends
  await Promise.all([simulatedPumpPart, simulatedRobotPart]);

  sdk1.dispose();
  sdk2.dispose()
}

main()
