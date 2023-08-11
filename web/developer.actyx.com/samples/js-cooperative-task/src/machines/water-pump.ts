import { ProtocolEvents, protocol } from "./protocol";

export const machine = protocol.makeMachine("WaterPump");

export const _1_Initial = machine
  .designEmpty("Initial")
  .command("dockAvailable", [ProtocolEvents.DockAvailable], () => [{}])
  .finish();
export const _2_DockAvailable = machine.designEmpty("DockAvailable").finish();
export const _3_Pumping = machine
  .designEmpty("Pumping")
  .command("waterSupplied", [ProtocolEvents.WaterSupplied], () => [{}])
  .finish();
export const _4_Pumped = machine.designEmpty("Pumped").finish();
export const _5_DockCleared = machine.designEmpty("DockCleared").finish();

_1_Initial.react(
  [ProtocolEvents.DockAvailable],
  _2_DockAvailable,
  () => undefined
);
_2_DockAvailable.react(
  [ProtocolEvents.RobotIsDocked],
  _3_Pumping,
  () => undefined
);
_3_Pumping.react([ProtocolEvents.WaterSupplied], _4_Pumped, () => undefined);
_4_Pumped.react(
  [ProtocolEvents.RobotIsUndocked],
  _5_DockCleared,
  () => undefined
);
