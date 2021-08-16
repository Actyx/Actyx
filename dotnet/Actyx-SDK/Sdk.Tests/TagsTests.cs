using Actyx;
using System;
using System.Collections.Generic;
using Xunit;

namespace Sdk.Tests
{
    public class TagsTests
    {

        // "B or Z"
        interface BorZ
        {
            // Some generic ID for things of this type
            string ZorbId { get; }
        }

        // A is just A
        class A
        {
        }

        // B inherits from A and implements interface "B or Z"
        class B : A, BorZ
        {
            public string ZorbId { get; internal set; }
        }

        // C inherits from B
        class C : B
        {
        }

        // Z is unrelated to the ABC-hierarchy, except for a shared interface with B
        class Z : BorZ
        {
            public string ZorbId { get; internal set; }
        }

        // Tags to identify certain classes of event
        private Tag<A> tagA = new Tag<A>("A");
        private Tag<B> tagB = new Tag<B>("B");
        private Tag<C> tagC = new Tag<C>("C");
        private Tag<Z> tagZ = new Tag<Z>("Z");

        private Tag<object> tagAny = new Tag<object>("any");
        private Tag<BorZ> tagBorZ = new Tag<BorZ>("B or Z", evt => evt.ZorbId);
        private Tag<object> dangerousTag = new Tag<object>("any 'old' tag 'we' can think of");

        // Interface to test the "consumption" of the Tags-related interfaces. This is just to test
        // compilation maybe
        interface MockEventFunctions
        {

            void emit<E>(E payload, params ITags<E>[] tags);

            void subscribe<E>(Func<E, int> handler, params IFrom<E>[] tags);

        }

        private void AssertTagsApplied<E>(E payload, ITags<E> tags, params string[] expected)
        {
            Assert.Equal(new List<string>(expected), tags.Apply(payload).Tags);
        }

        [Fact]
        public void TagMoreSpecificType()
        {
            // Since C is all types (except Z), we can apply a lot of different tags to it.
            var c = new C();
            AssertTagsApplied(c, tagC, "C");
            AssertTagsApplied(c, tagB, "B");
            AssertTagsApplied(c, tagBorZ, "B or Z");
            AssertTagsApplied(c, tagA, "A");
            AssertTagsApplied(c, tagA.And(tagB), "A", "B");
            AssertTagsApplied(c, tagAny.And(tagB), "any", "B");
            AssertTagsApplied(c, Tags<B>.AllOf(tagB, tagAny), "B", "any");
        }

        [Fact]
        public void AutoExtractIdSingle()
        {
            var z = new Z { ZorbId = "foo" };
            AssertTagsApplied(z, tagBorZ, "B or Z", "B or Z:foo");
            AssertTagsApplied(z, tagBorZ.WithId("custom override"), "B or Z", "B or Z:custom override");
        }

        [Fact]
        public void AutoExtractIdMulti()
        {
            var z = new Z { ZorbId = "my Z" };
            AssertTagsApplied(z, tagAny.And(tagZ).And(tagBorZ), "any", "Z", "B or Z", "B or Z:my Z");
            AssertTagsApplied(
                z,
                tagAny.WithId("x").And(tagZ).And(tagBorZ.WithId("very Z")),
                "any", "any:x", "Z", "B or Z", "B or Z:very Z"
            );
        }

        private void AssertAql(IFrom<object> selector, string expectedTagExpr)
        {
            Assert.Equal("FROM " + expectedTagExpr, selector.ToAql());
        }

        private void AssertAql<T>(IFrom<T> selector, string expectedTagExpr)
        {
            Assert.Equal("FROM " + expectedTagExpr, selector.ToAql());
        }

        [Fact]
        public void TagToAQL()
        {
            AssertAql(tagA, "'A'");
            AssertAql(tagBorZ, "'B or Z'");
            AssertAql(tagBorZ.Local(), "'B or Z' & isLocal");
            AssertAql(dangerousTag, "'any ''old'' tag ''we'' can think of'");
        }

        [Fact]
        public void TagsToAQL()
        {
            AssertAql<A>(tagA.And(tagB), "'A' & 'B'");
            AssertAql<BorZ>(tagBorZ.WithId("my Z"), "'B or Z' & 'B or Z:my Z'");

            AssertAql<BorZ>(tagBorZ.WithId("my Z"), "'B or Z' & 'B or Z:my Z'");
            AssertAql<B>(tagBorZ.And(tagB), "'B or Z' & 'B'");
            AssertAql<Z>(tagZ.And(tagBorZ), "'Z' & 'B or Z'");

            AssertAql<A>(tagA.Local().And(tagB), "'A' & 'B' & isLocal");
            AssertAql<C>(Tags<C>.AllOf(tagA, tagB.Local(), tagC), "'A' & 'B' & 'C' & isLocal");
            AssertAql<object>(tagZ.And(dangerousTag), "'Z' & 'any ''old'' tag ''we'' can think of'");
        }

        [Fact]
        public void FromToAQL()
        {
            // Adding more specific types via Or works fine:
            AssertAql<A>(tagA.Or(tagB), "'A' | 'B'");
            AssertAql<BorZ>(tagBorZ.Or(tagB), "'B or Z' | 'B'");
            AssertAql<BorZ>(tagBorZ.Or(tagC), "'B or Z' | 'C'");

            // Going from more specific to more general types does not work via Or (except going
            // straight for 'object'), needs static helper.
            AssertAql<BorZ>(From<BorZ>.AnyOf(tagB, tagBorZ), "'B' | 'B or Z'");
            AssertAql<object>(tagBorZ.Or(tagZ).Or(tagA), "'B or Z' | 'Z' | 'A'");
            AssertAql<object>(tagA.Or(tagZ), "'A' | 'Z'");
            AssertAql<object>(tagBorZ.WithId("my Z").Or(tagAny), "'B or Z' & 'B or Z:my Z' | 'any'");

            // Locality is kept to the respective sets
            AssertAql<A>(tagA.Local().Or(tagB), "'A' & isLocal | 'B'");
            AssertAql(From<A>.AnyOf(tagA.Local().And(tagAny), tagB.Local(), tagC), "'A' & 'any' & isLocal | 'B' & isLocal | 'C'");

            // Id propagates
            AssertAql<object>(
                tagZ.WithId("zz zz").And(dangerousTag).Or(tagC.Local()),
                "'Z' & 'Z:zz zz' & 'any ''old'' tag ''we'' can think of' | 'C' & isLocal"
            );
        }

        // Theese are just to test compilation, not execution
        private void emit1(MockEventFunctions w)
        {
            w.emit(new B { }, Tags<B>.AllOf(tagA, tagB));

            w.emit(new B { }, tagA.And(tagB));
            w.emit(new B { }, tagB.And(tagA));
            w.emit(new B { }, tagB.And(tagB));
            w.emit(new B { }, tagA.And(tagA));
            // Fails:
            // w.emit(new A { }, tagA.And(tagB));
            // w.emit(new A { }, tagB.And(tagA));
            // tagA.And(tagZ);

            w.emit(new B { }, tagA, tagB);
            w.emit(new B { }, tagB, tagA);
            w.emit(new C { }, tagB, tagA, tagC);
            w.emit(new A { }, tagA);
            w.emit(new C { }, tagC, tagB);

            // Fails:
            // w.emit(new A { }, Tags<B>.AllOf(tagA, tagB));
            // w.emit(new A { }, tagC);
        }

        private void emit3(MockEventFunctions w)
        {

            w.emit(new C { }, tagA);
            w.emit(new B { }, tagA);
            w.emit(new A { }, tagA);

            // Fails:
            // w.emit(new A { }, tagB);

            w.emit(new B { }, tagB);
            w.emit(new C { }, tagB);

            w.emit(new C { }, tagC);


            w.subscribe((A a) => 5, tagC);
            w.subscribe((A a) => 5, tagB);
            w.subscribe((A a) => 5, tagA);
            w.subscribe((A a) => 5, tagB, tagC, tagA);

            w.subscribe((B b) => 5, tagC);
            w.subscribe((B b) => 5, tagB.Or(tagC));
            w.subscribe((B b) => 5, tagB.Or(tagB));

            // Both orders work when using AnyOf
            w.subscribe((B b) => 5, From<B>.AnyOf(tagC, tagB));
            w.subscribe((B b) => 5, From<B>.AnyOf(tagB, tagC));

            // Fails:
            // w.subscribe((C b) => 5, tagB);
            // w.subscribe((B b) => 5, tagA);

            w.subscribe((C c) => 5, tagC);
        }
    }
}
