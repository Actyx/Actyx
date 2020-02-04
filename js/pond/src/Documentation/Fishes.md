# Fishes

Fishes are prototypes for entities participating in an distrbuted event sourcing system.

Fishes are defined using the `FishTypeImpl.of` factory method. This produces a type `FishTypeImpl<S, C, E, P>`, which has type
parameters for the private state of the fish `S`, the type of the consumed commands `C` the type of the produced events `E`
and the publicly observable state `P`.

When interacting with a fish outside of unit testing, knowledge of the internal state is not necessary, since it is not
possible to observe it. Such knowledge is also not desirable, since a fish might change its internal state at any time.

The type to be used to pass around fish references is therefore `FishType<C, E, P>`, which has only type parameters for the
publicly accessible types.

The most important methods to define behaviour of a fish are [onCommand](CommandApi.md) and [onEvent](EventApi.md).

# Testing

The [testkit](Testkit.md) offers various ways to test fishes.
