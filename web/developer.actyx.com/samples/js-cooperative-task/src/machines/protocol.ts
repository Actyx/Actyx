import { MachineEvent, SwarmProtocol } from "@actyx/machine-runner";

export namespace ProtocolEvents {
  export const DockAvailable =
    MachineEvent.design("DockAvailable").withoutPayload();
  export const RobotIsDocked =
    MachineEvent.design("RobotIsDocked").withoutPayload();
  export const WaterSupplied =
    MachineEvent.design("WaterSupplied").withoutPayload();
  export const RobotIsUndocked =
    MachineEvent.design("RobotIsUndocked").withoutPayload();
  export const All = [
    DockAvailable,
    RobotIsDocked,
    WaterSupplied,
    RobotIsUndocked,
  ] as const;
}

export const protocol = SwarmProtocol.make(
  "water-drawing-exchange",
  ProtocolEvents.All
);
