import { createMachineRunner } from "@actyx/machine-runner";
import { WaterPump, protocol } from "../machines";
import { Actyx } from "@actyx/sdk";

export const supplyWater = async (actyx: Actyx, dockingId: string) => {
  console.log("pump starts task:", dockingId);

  const tag = protocol.protocol.tagWithEntityId(dockingId);
  const machine = createMachineRunner(actyx, tag, WaterPump.ClearingDock, undefined);

  for await (const state of machine) {
    console.log("pump is:", state.type);

    const whenInitial = state.as(WaterPump.ClearingDock);
    if (whenInitial) {
      // Here the actual work would happen
      // Sleeping for a second as the customary replacement
      await sleep(1000);
      await whenInitial.commands?.dockAvailable();
    }

    const whenPumping = state.as(WaterPump.PumpingWater);
    if (whenPumping) {
      // Here the actual work would happen
      // Sleeping for a second as the customary replacement
      await sleep(1000);
      await whenPumping.commands?.waterSupplied();
    }

    const whenCleared = state.as(WaterPump.Done);
    if (whenCleared) {
      break;
    }
  }

  console.log("pump finishes task:", dockingId);
};

const sleep = (dur: number) => new Promise((res) => setTimeout(res, dur));
