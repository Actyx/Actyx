import { createMachineRunner } from "@actyx/machine-runner";
import { WateringRobot, protocol } from "../machines";
import { Actyx } from "@actyx/sdk";

export const dockAndDrawWater = async (actyx: Actyx, dockingId: string) => {
  console.log("robot starts task:", dockingId)

  const tag = protocol.protocol.tagWithEntityId(dockingId);
  const machine = createMachineRunner(
    actyx,
    tag,
    WateringRobot.WaitingForAvailableDock,
    undefined
  );

  for await (const state of machine) {
    console.log("robot is:", state.type)

    const whenDocking = state.as(WateringRobot.Docking);
    if (whenDocking) {
      await whenDocking.commands?.docked();
    }

    const whenWaterPumped = state.as(WateringRobot.Undocking);
    if (whenWaterPumped) {
      await whenWaterPumped.commands?.undocked();
    }

    const whenDone = state.as(WateringRobot.Done);
    if (whenDone) {
      break;
    }
  }

  console.log("robot finishes task:", dockingId)
};
