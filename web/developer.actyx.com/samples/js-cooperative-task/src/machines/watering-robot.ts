import { ProtocolEvents, protocol } from "./protocol";

export const machine = protocol.makeMachine("WateringRobot");

export const _1_WaitingForDock = machine.designEmpty("WaitingForDock").finish();

export const _2_Docking = machine
  .designEmpty("Docking")
  .command("docked", [ProtocolEvents.RobotIsDocked], () => [{}])
  .finish();

export const _3_DockedAndWaitingForWater = machine
  .designEmpty("DockedAndWaitingForWater")
  .finish();

export const _4_WaterPumped = machine
  .designEmpty("WaterPumped")
  .command("undocked", [ProtocolEvents.RobotIsUndocked], () => [{}])
  .finish();

export const _5_DeliveringWaterToPlant = machine
  .designEmpty("DeliveringWaterToPlant")
  .finish();

_1_WaitingForDock.react(
  [ProtocolEvents.DockAvailable],
  _2_Docking,
  () => undefined
);

_2_Docking.react([ProtocolEvents.RobotIsDocked], _3_DockedAndWaitingForWater, () => undefined);

_3_DockedAndWaitingForWater.react(
  [ProtocolEvents.WaterSupplied],
  _4_WaterPumped,
  () => undefined
);

_4_WaterPumped.react(
  [ProtocolEvents.RobotIsUndocked],
  _5_DeliveringWaterToPlant,
  () => undefined
);