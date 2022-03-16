import { Fish, FishId, Metadata, Pond, Tag, Tags } from "@actyx/pond";
import manifest from "./manifest.json";

const tags = [Tag("hello-world")];

const HelloWorldFish: Fish<string, string> = {
  fishId: FishId.of("com.example.quickstart", "quickstart", 0),
  initialState: "Hello, World!",
  onEvent: (_oldState: any, event: any, _metadata: Metadata) =>
    `Received:  ${event}`,
  where: Tags("hello-world"),
};

const main = async () => {
  const pond = await Pond.default(manifest);
  pond.observe(HelloWorldFish, (state) => console.log(state));

  var counter = 1;
  setInterval(async () => {
    const event = `Hello ${counter}`;
    const result = await pond.publish({
      tags: ["hello-world"],
      event,
    });
    console.log(`Published: ${event} [${result.eventId}]`);
    counter += 1;
  }, 3500);
};

main();
