using Actyx;
using System;
using System.Collections.Generic;
using System.Linq;
using System.Threading.Tasks;

namespace Sdk.IntegrationTests
{
    class Tags<E>
    {
        private readonly List<string> rawTags = new List<string>();

        public Tags(IEnumerable<string> tags)
        {
            this.rawTags.AddRange(tags);
        }

        public Tags(params string[] tags)
        {
            this.rawTags.AddRange(tags);
        }

        public Tags<ESub> and<ESub>(Tags<ESub> other) where ESub : E
        {
            return new Tags<ESub>(rawTags.Concat(other.rawTags));
        }
    }

    class Tag<E> : Tags<E>
    {
        private readonly string rawTag;

        public Tag(string rawTag) : base(new string[] { rawTag })
        {
            this.rawTag = rawTag;
        }

        Tags<E> withId(string id)
        {
            return new Tags<E>(new string[] { this.rawTag, this.rawTag + ':' + id });
        }
    }

    public delegate S OnEvent<S, in E>(S oldState, E eventPayload);

    interface IUpdatedBy<in E>
    {
        void updateWith(E eventPayload);
    }

    interface IUpdateState<S>
    {
        S updateState(S oldState);
    }

    interface FishBuilder<S>
    {
        FishBuilder<S> subscribeTo<E>(Tags<E> subscription, OnEvent<S, E> handler);

        FishBuilder<S> subscribeTo<E>(Tags<E> subscription) where E : IUpdateState<S>;

        FishBuilder<S> subscribeToT<SConcrete, E>(Tags<E> subscription) where SConcrete : IUpdatedBy<E>, S;
    }

    interface FishBuilderX<S, X> : FishBuilder<S> where S : IUpdatedBy<X>
    {
        FishBuilderX<S, X> subscribeToX(Tags<X> subscription);
    }


    interface Ponder
    {
        FishBuilder<S> fish<S>(string fishId, S initialState);

        FishBuilderX<S, X> fish<S, X>(string fishId, S initialState) where S : IUpdatedBy<X>;
    }


    interface IOwnEvent : IUpdateState<MyState>
    {
    }

    interface IForeignEvent
    {

    }

    class MyState : IUpdatedBy<IForeignEvent>
    {
        public void updateWith(IForeignEvent evt)
        {
            return;
        }
    }

    class Fooo
    {
        static FishBuilder<MyState> Ok(Ponder p)
        {
            Tags<string> myTags = new Tags<string>("foo", "bar");

            return new FishBuilderC<MyState>("dummy", new MyState())
                .subscribeTo(myTags, (oldState, evt) => new MyState())
                .subscribeTo(new Tags<IOwnEvent>("own"))
                .subscribeToT<MyState, IForeignEvent>(new Tags<IForeignEvent>("foreign"));
        }
    }

    class FishBuilderC<S> : FishBuilder<S>
    {
        private readonly string fishId;
        private readonly S initialState;

        public FishBuilderC(string fishId, S initialState)
        {
            this.fishId = fishId;
            this.initialState = initialState;
        }

        public FishBuilder<S> subscribeTo<E>(Tags<E> subscription, OnEvent<S, E> handler)
        {
            return this;
        }

        public FishBuilder<S> subscribeTo<E>(Tags<E> subscription) where E : IUpdateState<S>
        {
            return this;
        }

        public FishBuilder<S> subscribeToT<SConcrete, E>(Tags<E> subscription) where SConcrete : IUpdatedBy<E>, S
        {
            return this;
        }
    }

}
