---
title: Hello World
---

The simplest possible ActyxOS app you can write is:

```html
<html>
  <head>
    <script>
      function publishClick() {
        return fetch("http://localhost:4454/api/v1/events/publish", {
          method: "POST",
          body: JSON.stringify({ data: [{
            semantics: "com.hello-world.message",
            name: "app1",
            payload: { message: "Hello, World!" }

          }]}),
          headers: { "Content-Type": "application/json" }
        });
      }
    </script>
  </head>
  <body>
    <a href="#" onclick="publishClick()">Send "Hello, World!"</a>
  </body>
</html>
```

This app publishes an event whenever the user clicks the link on the page. ActyxOS is a system for programming collaboration. How does this play in here? Well let's write a second app and run it on a second device:

```html
<html>
  <head>
    <script>
      fetch("http://localhost:4454/api/v1/events/subscribe", {
        method: "POST",
        body: JSON.stringify({
          subscriptions: [{ name: "app1" }]
        }),
        headers: { "Content-Type": "application/json"
                 , "Accept": "application/json" },
      })
        .then(r => r.body.getReader())
        .then(reader => {
          const dec = new TextDecoder();
          const loop = () => {
            reader.read().then(
              chunk => {
                if (!chunk.done) {
                  var payload = JSON.parse(dec.decode(chunk.value)));
                  var message = payload.message;
                  var li = document.getElementById('messages');
                  var newMessage = document.createTextNode(message)
                  li.appendChild(newMessage);
                  loop();
                }
              }
            );
          };
          loop();
        });

    </script>
  </head>
  <body>
    <li id="messages"></li>
  </body>
</html>
```

What you have just built is a distributed, peer-to-peer "Hello, World!" application where one app, on one device, publishes events and another app, on another device, receives these events; without any central servers, databases or other components. In ActyxOS terminology, such a network of collaborating devices is called a _swarm_.

Over the next few sections and chapters, you will get to know ActyxOS and all its features.

---

### How to read this guide {#how-to-read-this-guide}

This guide will take you through the following main concepts in a step-by-step fashion:

- Event streams
- Blob storage
- User interfaces
- Logging
- Thinking in ActyxOS

This is the first chapter of the guide. You can find a list of all its chapters in the navigation sidebar. If you’re reading this from a mobile device, you can access the navigation by pressing the button in the bottom right corner of your screen.

---

### Knowledge Level Assumptions {#knowledge-level-assumptions}

ActyxOS apps can be built with any programming languages. To keep things consistent, the example in this documentation are all in JavaScript or TypeScript. A very basic level of proficiency is required to follow the guide and understand the code examples.

> Need to freshen up on your JavaScript or TypeScript?
>
> If you would like to learn more about or brush up on your JavaScript or TypeScript, head to [developer.mozilla.org](https://developer.mozilla.org/en-US/docs/Web/JavaScript) or [www.typescriptlang.org](https://www.typescriptlang.org).

---

### Let's get started {#lets-get-started}

Jump to the next section and learn about how to [publish and react to events](/os/docs/event-streams.html).