---
id: thinking-in-actyxos
title: Thinking in ActyxOS
permalink: os/docs/thinking-in-actyxos.html
prev: logging.html
---

We have built ActyxOS on the premise of autonomously collaborating agents — machines and humans. Designing such a system works differently from the traditional monolithic approach.

On this page we walk through the design process on a concrete example, leading up to the point where the architecture can then be implemented using the techniques shown in the previous sections.

## Contents

- [Who are the participants?](#participants)
- [What do they know?](#knowledge)
- [Which information do they create?](#information)
- [Designing the event streams](#event-streams)

## Who are the participants? {#participants}

The first thing to consider when creating a distributed app is who the participants of the relevant system are. These participants will collaborate to solve the problem at hand, dividing the work among them and communicating with one another in the process.

Let’s take for example a production process in a factory that spans three workstations. The first station turns its input material into an intermediate product that is then brought to the second station, where the second step of the process is performed. After the third step is done at the last station, the finished goods are transported into the warehouse, awaiting shipment to the customer.

![](../images/shopfloor-logistics-illustration.png)

At each workstation, a person oversees the production of output materials from the delivered inputs while most of the physical work is performed by machines (e.g. cutting, drilling, surface treatment). A logistics robot is responsible for transporting intermediate products from one station to the next, for supplying input material to the first workstation before it runs out, and for bringing the finished products to the warehouse in the end.

We recognize the following participants:

- the machines at each workstation
- the human operator at each workstation
- the logistics robot

If you have been involved in factory software before, you’ll probably remark on further missing participants, like the planning and scheduling department that defines what to produce where at which time, or the warehouse management that keeps track of the location of all material in the factory. A factory is a complex network of collaborating stakeholders, in this example, we only take a look at a small selection of them.

## What do they know? {#knowledge}

The second step in creating a distributed app is to consider the knowledge each of the participants must have to fulfill their role in the system. To this end, it is easiest to start from the angle of the participants that tie the process together, for example by coordinating it or by facilitating the workflow. In this case, this is the logistics robot.

The robot will need to know

- how much of which material is where, including the intermediate products
- which station consumes which type of material
- what the robot is currently doing
- and where the stations are

With this knowledge, it can figure out when the input buffer of a workstation is about to run empty and needs to be filled (especially for the first workstation), and it knows when there is enough in the output buffer of a workstation to transport it to the next station or into the warehouse.

The human operator oversees what happens at each workstation, so we consider them next. They need to know what their workstation is currently working on, i.e. which step of which production process. This includes the definition of all required input materials and the procedure for the process step in terms of tools and machine configuration. The operator also needs to assess the quality of the produced output material and the quantity emitted so far (usually it is a good idea to keep track of the scrap as well). The operator thus knows how many pieces their station will produce in the imminent future and how many pieces are ready to be collected by the logistics robot.

At a high level, the human operator needs to know

- what exactly their workstation is processing
- and how much of it is ready

The machines support the human operator not only by doing the physical work, but they also take care of the low-level management of the production process at the workstation. For example they may have quality assurance equipment built-in to assess whether a produced piece is good or scrap and they will have counters keeping track of how many pieces have been processed; they may also have sensors for environmental conditions or process parameters (like temperatures), and equipment monitoring their performance or their consumption of operating fluids.

A machine thus needs to know

- its operating condition
- what it is working on
- and how many good and scrap pieces it has produced so far

> Note
>
> In this case, the decomposition of who knows what followed a clear hierarchy, from wider operational scope to narrower scope. This does not always need to be the case, the sequence of considering the knowledge of the participants is not crucial, there is no “wrong” order. If a hierarchy can be found, then it is usually helpful, though.

## Which information do they create? {#information}

So far we have identified the participants and described what knowledge each of them must possess to make the collaboration between them work. It is usually the case that several participants need to know the same thing, like “what is the first workstation working on” in our ongoing example. The most important step is now to define who owns this information, who creates it.

In a distributed system it is very important to ensure that there is only one source of truth for any given piece of information, the participant that owns it. Every other participant can ask for this information and use it, but they cannot change or create it.

In our factory example, there are several pieces of information that we must place:

- what material is where
- which workstation is working on what (i.e. what it is consuming and producing)
- the operating condition of each machine
- where the workstations are
- and what the robot is currently doing

Each of these are facts, objective truths that can be recognized by one of the participants who is in the right place at the right time. Whenever such a fact is recognized, it is written into that participant’s event stream as discussed in the previous sections. We have thus designed the event streams for the distributed apps making up the software for this particular factory example.

Regarding **material tracking**, the participant most apt to recognize changes is whoever brings about that change: the logistics robot should record the movements it performs, and the machines at the workstations shall record the material they place in their output buffers and the material they consume from their input buffers. The latter may be confirmed by the human operator, or the operator may be fully responsible for this, depending on the degree of automation in our example factory.

The logistics robot will then subscribe to the material tracking event streams from all participants to stay informed about all changes, including consumption and production of material.

In terms of **which workstation is working on what** it is the duty of the human operator to oversee their local workstation, and it is thus their responsibility to record changes to this information: whenever a workstation switches from one product to the next, or from one production order to the next, the operator will perform this change and inform the other participants by storing this information in an app. The app will persist this information in its event stream so that for example the logistics robot learns of the changed demand in input material.

Following the same principle, it is obvious that the **operational condition of the machine** needs to be recorded by the machine itself, emitting events accordingly via the app running in its control cabinet. This information is then available to the human operator who can take action to maintain the required performance, and it is also available for other processes like scheduling preventive or corrective maintenance in more severe cases.

The logistics robot needs to know **where the workstations are**. This could be hard-coded, but it could also be stored by the engineer setting up the workstations and updated whenever the shop-floor layout is changed. In the latter case, that engineer would use a separate app to record these changes.

By now it should be obvious that the information on **what the robot is currently doing** is owned by the robot, emitting events on the start and completion of every material transport mission it recognizes and executes. This information is then available to the human operators who can see when material is being delivered and when the produced pieces will be picked up from the output buffer.

## Designing the event streams {#event-streams}

Now that we know the participants and which information they create, the final step is to condense all our findings into the definition of event streams for our distributed app. This step entails that we look at the information and its changes in great detail, noting down which kinds of events need to be recorded and what their parameters are.

> Note
>
> You might want to perform this step as a group exercise in a so-called [event-storming session](https://en.wikipedia.org/wiki/Event_storming). This way every member of the team is part of the domain modeling process and will be in a good position to implement pieces of it afterward.

We do this exemplarily for the logistics robot: at a high level, the robot records the planning and execution of its transport missions and it also records the material movements that it performs in their cause. Diving into the latter, there are two potentially interesting events to emit, namely the pickup of material from one location and the delivery of material to another location. The pickup should include where it was made and which quantity of what material was picked up. The delivery should at minimum include how much of which material was delivered to what location; it would be very useful for external observers to make this event self-contained, though, by adding from which location the material was picked up earlier. This way, the external observer need not care about the internal state of the robot (i.e. where its current freight came from).

The other part of the robot — its mission planning — is captured in another event stream. Here, the robot recognizes the need for a transport mission by noticing a lack of input materials at the first workstation or the presence of enough output materials at any of the three workstations. After a mission is recognized, it will be started as soon as possible (i.e. while no other mission is ongoing and the robot is fully operational), leading to the emission of another event. In the cause of the mission, the robot may record more information about the route it is taking or the individual tasks it performs, and in the end, it will emit an event denoting the completion of the mission — either successful or not.

After performing similar analyses for the other participants, we arrive at the following event stream definition table:

| participant | stream name | events |
| ----------- | ----------- | ------ |
| logistics robot | `material-movements` | **pickedUp**(fromWhere, what, howMuch, byWhom)<br/>**delivered**(fromWhere, what, howMuch, toWhere, byWhom)
| | `robot-missions/<name>` | **missionRecognized**(fromWhere, what, howMuch, toWhere)<br/>**missionStarted**(fromWhere, what, howMuch, toWhere)<br/>**missionUpdate**( … whatever is interesting)<br/>**missionCompleted**(fromWhere, what, howMuch, toWhere)<br/>**missionFailed**(fromWhere, what, howMuch, toWhere, whatWentWrong)
| operator (tablet) | `workstation-status/<name>` | **workStarted**(productionProcess, step)<br/>**workStopped**(productionProcess, step)
| machine | `material-movements` | **consumed**(fromWhere, what, howMuch, byWhom)<br/>**produced**(what, howMuch, byWhom, where)
| | `machine-status/<name>` | **machineReady**<br/>**interrupted**(reason)<br/>**workStarted**(productionOrder, step)<br/>**workStopped**(productionOrder, step)<br/>**outputProduced**(what, howMuch)<br/>**scrapProduced**(what, howMuch, reason)

With the detailed description of information flows within our system we are now ready to fire up our editors and write the actual application code.

## And that’s it!

We have explored the design process for a distributed app on the example of collaboration between machines and humans on the factory shop-floor: first we identified the participants, then we figured out what each of them needs to know in order to fulfill their role, followed by identifying the owner of every needed piece of information. With this preparation, it was easy to flesh out the event streams emitted by each participant, capturing the facts that are recorded and making them available throughout the system.

As you repeat this process you will notice that it naturally flows from the consideration that machines and humans collaborate, each performing their part of the choreography with some autonomy. This results in a system design that avoids single points of failure and distributes the processing and communication load, making the whole system extremely resilient.
