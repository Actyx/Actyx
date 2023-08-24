import { ProtocolEvents, protocol } from "./protocol";

export const machine = protocol.makeMachine("WateringRobot");

export const WaitingForAvailableDock = machine.designEmpty("WaitingForAvailableDock").finish();

export const Docking = machine
  .designEmpty("Docking")
  .command("docked", [ProtocolEvents.RobotIsDocked], () => [{}])
  .finish();

export const WaitingForWater = machine
  .designEmpty("WaitingForWater")
  .finish();

export const Undocking = machine
  .designEmpty("Undocking")
  .command("undocked", [ProtocolEvents.RobotIsUndocked], () => [{}])
  .finish();

export const Done = machine
  .designEmpty("Done")
  .finish();

WaitingForAvailableDock.react(
  [ProtocolEvents.DockAvailable],
  Docking,
  () => undefined
);

Docking.react([ProtocolEvents.RobotIsDocked], WaitingForWater, () => undefined);

WaitingForWater.react(
  [ProtocolEvents.WaterSupplied],
  Undocking,
  () => undefined
);

Undocking.react(
  [ProtocolEvents.RobotIsUndocked],
  Done,
  () => undefined
);