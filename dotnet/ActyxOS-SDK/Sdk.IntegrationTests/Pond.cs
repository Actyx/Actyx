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

    class MyState : IUpdatedBy<IOwnEvent>, IUpdatedBy<IForeignEvent>
    {
        public MyState handleForeignEvent(IForeignEvent evt)
        {
            return this;
        }


        public void updateWith(IOwnEvent eventPayload)
        {
            return;
        }

        public void updateWith(IForeignEvent eventPayload)
        {
            return;
        }
    }

    class Fooo
    {
        static FishBuilder<MyState> Ok(Ponder p)
        {
            Tags<string> myTags = new Tags<string>("foo", "bar");

            return FishBuilderC<MyState>.mkWithStart("dummy", new MyState(), new Tags<IOwnEvent>("own"), new Tags<IForeignEvent>("foreign"));

            // return new FishBuilderC<MyState>("dummy", new MyState())
            //     .subscribeTo(myTags, (oldState, evt) => new MyState())
            //     .subscribeTo(new Tags<IOwnEvent>("own"))
            //     .subscribeTo(new Tags<IForeignEvent>("foreign"), MyState.handleForeignEvent);
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


        public static FishBuilderC<X> mkWithStart<X, E, F>(string fishId, X initialState, Tags<E> ee, Tags<F> ef) where X : IUpdatedBy<E>, IUpdatedBy<F>
        {
            return new FishBuilderC<X>(fishId, initialState);
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
