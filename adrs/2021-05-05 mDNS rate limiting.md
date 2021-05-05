# Limiting mDNS network usage without impairing discovery speed

|  |  |
| --- | --- |
| date | 2020-05-05 |
| status | accepted |
| persons | @rkuhn, @dvc94ch |

## Decision

Configuration parameters: τ (discovery time target), φ (response frequency target)

Each node sends mDNS queries according to the following algorithm:

1. set timeout to a duration pulled at random from the interval [τ, 2τ].
2. upon reception of another node’s mDNS query go back to 1.
3. upon timeout, send mDNS query and go back to 1.

Each node responds to a mDNS query according to the following algorithm:

1. at reception of mDNS query, set timeout to a duration pulled at random from the interval [0, 100ms].
2. set counter to zero.
3. upon reception of another node’s mDNS response to this query, increment the counter.
4. when counter is greater than τ×φ end procedure.
5. upon timeout send response.

## Business background

Our customers do not appreciate using a significant portion of their available network bandwidth on broadcast mDNS traffic, regardless of swarm size.
Therefore we must limit broadcast bandwidth usage while still keeping discovery working properly.
The latter implies that we must regularly perform a basic discovery procedure which also needs to have decent chances of finding previously unknown nodes, otherwise partitions won’t heal reliably.

## Consequences

With the above approach we should get (re)discovery latency down to the order of τ (which could for example be 10sec) without incurring any coordination overhead.

Since the limitation of requests and responses are both based on randomised time intervals and local network observation, duplicates cannot be excluded.
This means that the target response rate φ can be exceeded, especially in very large swarms.
The excess rate will typically not be orders of magnitude larger than the target rate, though.
So it should behave like a somewhat soft but dependable boundary, not a hard limit.
