import { createMachineRunner } from "@actyx/machine-runner";
import { WateringRobot, protocol } from "../machines";
import { Actyx } from "@actyx/sdk";

export const dockAndDrawWater = async (actyx: Actyx, dockingId: string) => {
  console.log("robot starts task:", dockingId);

  const tag = protocol.protocol.tagWithEntityId(dockingId);
  const machine = createMachineRunner(actyx, tag, WateringRobot.WaitingForAvailableDock, undefined);

  for await (const state of machine) {
    console.log("robot is:", state.type);

    const whenDocking = state.as(WateringRobot.Docking);
    if (whenDocking) {
      // Here the actual work would happen
      // Sleeping for a second as the customary replacement
      await sleep(1000);
      await whenDocking.commands?.docked();
    }

    const whenWaterPumped = state.as(WateringRobot.Undocking);
    if (whenWaterPumped) {
      // Here the actual work would happen
      // Sleeping for a second as the customary replacement
      await sleep(1000);
      await whenWaterPumped.commands?.undocked();
    }

    const whenDone = state.as(WateringRobot.Done);
    if (whenDone) {
      break;
    }
  }

  console.log("robot finishes task:", dockingId);
};

const sleep = (dur: number) => new Promise((res) => setTimeout(res, dur));
