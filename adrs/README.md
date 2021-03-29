# Architecture Decision Records (ADRs). 

These are lightweight documents that capture important architectural decisions along with the when, how and why of the decision.

ADRs are particularly useful for projects where not all the decisions will be made at once and long lifetime projects where a new person joining the team, or someone who hasn’t worked on part of the project recently, will need to understand the rationale and consequences of a decision that has been made.

An ADR should record:

- When the decision was made.
- The status of the ADR (proposed, accepted, rejected, etc.).
- The current context and business priorities that relate to the decision.
- The decision and what will be done because of the decision.
- The consequences of making the architectural decision.
- Who was involved.

At just enough detail that someone new can understand the decision or someone who hasn't looked at the code for 6 months can quickly grasp why it's like that.

Here are some example formats:

Short:

```
In the context of <use case/user story u>, facing <concern c> we decided for <option o> to achieve <quality q>, accepting <downside d>.
```

Long (extra section “because”):

```
In the context of <use case/user story u>, facing <concern c> we decided for <option o> and neglected <other options>, to achieve <system qualities/desired consequences>, accepting <downside d/undesired consequences>, because <additional rationale>.
```

ADRs are best stored in the revision control system alongside the code that they relate to, keeping the context with the code.

Further Reading: https://adr.github.io/


A great point made by ThoughtWorks Head of Technology:

>The real power of Decision Records is they give you the ability to go back in time and examine your decision with hindsight. You can then learn from it.

>They do this by acting as a time capsule. By capturing what you believed at the time, what assumptions you made, what information you held and what rationale you applied. You are literally gifting this to your future self enabling them to challenge you.

>Without a Decision Record, you are left to your memory. And you will misremember, mistake things, forget and ultimately change the narrative. And you will fail to learn.

