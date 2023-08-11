import {
  SwarmProtocolType,
  checkProjection,
  checkSwarmProtocol,
} from "@actyx/machine-check";
import { describe } from "@jest/globals";
import { ProtocolEvents } from "./machines/protocol";
import { WaterPump, WateringRobot } from "./machines";

const swarmProtocol: SwarmProtocolType = {
  initial: "Initial",
  transitions: [
    {
      source: "Initial",
      target: "Docking",
      label: {
        cmd: "dockAvailable",
        role: "pump",
        logType: [ProtocolEvents.DockAvailable.type],
      },
    },
    {
      source: "Docking",
      target: "DockedAndWaitingForWater",
      label: {
        cmd: "docked",
        role: "robot",
        logType: [ProtocolEvents.RobotIsDocked.type],
      },
    },
    {
      source: "DockedAndWaitingForWater",
      target: "WaterDrawn",
      label: {
        cmd: "waterSupplied",
        role: "pump",
        logType: [ProtocolEvents.WaterSupplied.type],
      },
    },
    {
      source: "WaterDrawn",
      target: "Undocked",
      label: {
        cmd: "undocked",
        role: "robot",
        logType: [ProtocolEvents.RobotIsUndocked.type],
      },
    },
  ],
};

const waterPumpData = WaterPump.machine.createJSONForAnalysis(
  WaterPump._1_Initial
);
const wateringRobotData = WateringRobot.machine.createJSONForAnalysis(
  WateringRobot._1_WaitingForDock
);

const subscriptions = {
  pump: waterPumpData.subscriptions,
  robot: wateringRobotData.subscriptions,
};

describe("protocol", () => {
  it("is well-formed", () => {
    expect(checkSwarmProtocol(swarmProtocol, subscriptions)).toEqual({
      type: "OK",
    });
  });
});

describe("robot", () => {
  it("is well-formed", () => {
    expect(
      checkProjection(swarmProtocol, subscriptions, "robot", wateringRobotData)
    ).toEqual({
      type: "OK",
    });
  })
});

describe("pump", () => {
  it("is well-formed", () => {
    expect(
      checkProjection(swarmProtocol, subscriptions, "pump", waterPumpData)
    ).toEqual({
      type: "OK",
    });
  })
})
