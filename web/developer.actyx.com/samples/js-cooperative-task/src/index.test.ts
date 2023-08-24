import {
  SwarmProtocolType,
  checkProjection,
  checkSwarmProtocol,
} from "@actyx/machine-check";
import { WaterPump, WateringRobot, protocol } from "./machines";
import * as assert from "node:assert"

const { ProtocolEvents } = protocol

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
  WaterPump.ClearingDock
);
const wateringRobotData = WateringRobot.machine.createJSONForAnalysis(
  WateringRobot.WaitingForAvailableDock
);

const subscriptions = {
  pump: waterPumpData.subscriptions,
  robot: wateringRobotData.subscriptions,
};

assert.deepStrictEqual(checkSwarmProtocol(swarmProtocol, subscriptions), {
  type: "OK"
}, "protocol is not well-formed")

assert.deepStrictEqual(checkProjection(swarmProtocol, subscriptions, "robot", wateringRobotData), {
  type: "OK"
}, "robot is not well-formed")

assert.deepStrictEqual(checkProjection(swarmProtocol, subscriptions, "pump", waterPumpData), {
  type: "OK"
}, "pump is not well-formed")

console.log("all tests passed")