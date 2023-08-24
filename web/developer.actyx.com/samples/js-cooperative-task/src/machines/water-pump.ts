import { ProtocolEvents, protocol } from "./protocol";

export const machine = protocol.makeMachine("WaterPump");

export const ClearingDock = machine
  .designEmpty("ClearingDock")
  .command("dockAvailable", [ProtocolEvents.DockAvailable], () => [{}])
  .finish();

export const WaitingForRobotToDock = machine.designEmpty("WaitingForRobotToDock").finish();

export const PumpingWater = machine
  .designEmpty("PumpingWater")
  .command("waterSupplied", [ProtocolEvents.WaterSupplied], () => [{}])
  .finish();
export const WaitingForRobotToUndock = machine.designEmpty("WaitingForRobotToUndock").finish();

export const Done = machine.designEmpty("Done").finish();

ClearingDock.react(
  [ProtocolEvents.DockAvailable],
  WaitingForRobotToDock,
  () => undefined
);

WaitingForRobotToDock.react(
  [ProtocolEvents.RobotIsDocked],
  PumpingWater,
  () => undefined
);

PumpingWater.react([ProtocolEvents.WaterSupplied], WaitingForRobotToUndock, () => undefined);

WaitingForRobotToUndock.react(
  [ProtocolEvents.RobotIsUndocked],
  Done,
  () => undefined
);
