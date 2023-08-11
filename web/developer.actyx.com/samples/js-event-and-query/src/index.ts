import { Actyx, AqlEventMessage, Tag } from "@actyx/sdk";
import * as uuid from "uuid";

const APP_MANIFEST = {
  appId: "com.example.whosintheroom",
  displayName: "Who is in the room?",
  version: "1.0.0",
}

async function robot(roomId: string) {
  const selfId = uuid.v4();
  console.log(`robot:${selfId} spawned`)

  const sdk = await Actyx.of(APP_MANIFEST);

  while (true) {
    const presentRobotIds = await queryPresence(sdk, roomId);
    logPresence(selfId, presentRobotIds)
    if (!presentRobotIds.includes(selfId)) {
      await publishPresence(sdk, roomId, selfId)
      console.log(`robot/${selfId} sends its presence`)
    }
    await sleep(1000)
  }
}

// Actyx calls
const presenceTagString = (roomId: string) => `robot-presence:${roomId}`

const publishPresence = (sdk: Actyx, roomId: string, robotId: string) => {
  const tagString = presenceTagString(roomId)
  const tagged = Tag(tagString).apply(robotId)
  return sdk.publish(tagged)
}

const queryPresence = async (sdk: Actyx, roomId: string) => {
  const tagString = presenceTagString(roomId)
  const aql = `FROM '${tagString}'`
  const result = await sdk.queryAql(aql)
  return result.filter((ev): ev is AqlEventMessage => ev.type === "event").map(ev => ev.payload as string)
}

// utilities

const sleep = (duration: number) => new Promise(res => setTimeout(res, duration))

const logPresence = (selfId: string, presentRobotIds: string[]) => {
  const neighboringRobots = presentRobotIds.filter(id => id !== selfId)
  const neighboringRobotsAsListString = neighboringRobots.map(id => ` - robot/${id}`).join("\n")
  console.log(`robot/${selfId} sees:`)
  console.log(neighboringRobotsAsListString)
}

// main 

async function main() {
  // So that every run is unique;
  const roomId = uuid.v4()
  console.log(`room/roomId}`)

  // Spawn some robots
  const robots = [
    robot(roomId),
    robot(roomId),
  ]

  await Promise.all(robots)
}

main()