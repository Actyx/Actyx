import { createMachineRunner } from "@actyx/machine-runner";
import { WaterPump, protocol } from "../machines";
import { Actyx } from "@actyx/sdk";

export const dockAndSupplyWater = async (actyx: Actyx, dockingId: string) => {
  console.log("pump starts task:", dockingId)

  const tag = protocol.protocol.tagWithEntityId(dockingId);
  const machine = createMachineRunner(actyx, tag, WaterPump._1_Initial, undefined);

  for await (const state of machine) {
    console.log("pump is:", state.type)

    const whenInitial = state.as(WaterPump._1_Initial);
    if (whenInitial) {
      await whenInitial.commands?.dockAvailable();
    }

    const whenPumping = state.as(WaterPump._3_Pumping);
    if (whenPumping) {
      await whenPumping.commands?.waterSupplied();
    }

    const whenCleared = state.as(WaterPump._5_DockCleared);
    if (whenCleared) {
      break;
    }
  }

  console.log("pump finishes task:", dockingId)
};
