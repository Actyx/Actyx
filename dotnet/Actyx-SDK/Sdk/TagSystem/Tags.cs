using System;
using System.Collections.Generic;
using System.Linq;

namespace Actyx
{
    // A specific set of tags.
    public class Tags<E> : From<E>, ITags<E>
    {
        protected readonly RawTagSet underlying;
        public RawTagSet Underlying { get => underlying; }

        public static Tags<E> AllOf(params ITags<E>[] AllOfThese)
        {
            // TODO move to a RawTagSet ctor
            return new Tags<E>(
                RawTagSet.Create(
                    AllOfThese.SelectMany(x => x.Underlying.rawTags),
                    Array.Exists(AllOfThese, x => x.Underlying.onlyLocal)
                )
            );
        }

        // TODO Add more constructors

        public Tags(List<string> AllOfThese, bool OnlyLocal) : this(
            RawTagSet.Create(
                AllOfThese.Select(x => RawTag.Create(x)),
                OnlyLocal
            )
        )
        {
        }

        protected Tags(RawTagSet allOfThese) : base(new List<RawTagSet> { allOfThese })
        {
            this.underlying = allOfThese;
        }

        public Tags<E2> And<E2>(ITags<E2> MoreTags) where E2 : E
        {
            var copy = underlying.CopyWith(MoreTags.Underlying);
            return new Tags<E2>(copy);
        }

        public Tags<E> And(ITags<E> MoreTags)
        {
            var copy = underlying.CopyWith(MoreTags.Underlying);
            return new Tags<E>(copy);
        }

        public Tags<E> And(string tag)
        {
            var copy = underlying.CopyWith(RawTag.Create(tag));
            return new Tags<E>(copy);
        }

        public Tags<E> Local()
        {
            return new Tags<E>(this.underlying.Local());
        }

        public IEventDraft Apply(E eventData)
        {
            List<string> tags = this.underlying.AutoExtract(eventData);
            return new EventDraft
            {
                Tags = tags,
                Payload = eventData,
            };
        }

        public List<IEventDraft> Apply(params E[] events)
        {
            return events.Select(e => Apply(e)).ToList();
        }
    }
}
